import Database from "@tauri-apps/plugin-sql";
import { appDataDir, join } from "@tauri-apps/api/path";

// Valid table names - used to prevent SQL injection
export const VALID_TABLES = [
  "emails",
  "orders",
  "items",
  "images",
  "deliveries",
  "htmls",
  "order_emails",
  "order_htmls",
] as const;

export type ValidTableName = (typeof VALID_TABLES)[number];

export function isValidTableName(name: string): name is ValidTableName {
  return VALID_TABLES.includes(name as ValidTableName);
}

export function sanitizeTableName(tableName: string): string {
  if (!isValidTableName(tableName)) {
    throw new Error(`Invalid table name: ${tableName}`);
  }
  // The whitelist check above is sufficient since VALID_TABLES is a const array
  // containing only safe, pre-validated table names
  return tableName;
}

/**
 * Singleton database manager for SQLite connections
 *
 * This class manages a single database connection throughout the application lifecycle.
 * It handles connection initialization, caching, and cleanup with proper resource management.
 *
 * Key features:
 * - Singleton pattern ensures only one database connection exists
 * - Automatic cleanup on pagehide and beforeunload events
 * - Race condition protection during initialization
 * - Support for both sync and async cleanup
 *
 * @example
 * ```typescript
 * const manager = DatabaseManager.getInstance();
 * const db = await manager.getDatabase();
 * const results = await db.select("SELECT * FROM emails");
 * ```
 */
export class DatabaseManager {
  private static instance: DatabaseManager | null = null;
  private db: Database | null = null;
  private dbPath: string | null = null;
  private initPromise: Promise<Database> | null = null;
  private isClosing = false;
  private abortController: AbortController | null = null;

  private constructor() {
    // Register cleanup handlers with AbortController for proper cleanup
    if (typeof window !== "undefined") {
      this.abortController = new AbortController();
      const signal = this.abortController.signal;

      // Use pagehide for more reliable cleanup (works on mobile Safari and modern browsers)
      window.addEventListener('pagehide', () => {
        this.cleanup();
      }, { signal });

      // Also register beforeunload as fallback for older browsers
      // Note: beforeunload cannot reliably complete async operations
      // The database connection will be cleaned up by Tauri/browser when the process ends
      window.addEventListener('beforeunload', () => {
        this.cleanup();
      }, { signal });

      // Note: visibilitychange listener removed as it causes issues:
      // - Closes connection when user switches tabs, requiring reconnection
      // - Can interrupt ongoing operations
      // - Connection cleanup on page unload is sufficient for Tauri desktop apps
    }
  }

  static getInstance(): DatabaseManager {
    if (!DatabaseManager.instance) {
      DatabaseManager.instance = new DatabaseManager();
    }
    return DatabaseManager.instance;
  }

  async getDatabase(): Promise<Database> {
    // Don't allow new connections if we're in the process of closing
    if (this.isClosing) {
      throw new Error('DatabaseManager is closing, cannot get database connection');
    }

    // If already initialized, verify it's still valid and return
    if (this.db && this.dbPath && !this.isClosing) {
      return this.db;
    }

    // If already initializing, wait for that promise
    if (this.initPromise) {
      try {
        const db = await this.initPromise;
        // Check again in case cleanup() started while we were waiting
        if (this.isClosing) {
          throw new Error('DatabaseManager is closing, cannot get database connection');
        }
        return db;
      } finally {
        // Clear initPromise after waiting completes
        this.initPromise = null;
      }
    }

    // Initialize database
    this.initPromise = (async () => {
      const appDataDirPath = await appDataDir();
      const dbPath = await join(appDataDirPath, "paa_data.db");
      const db = await Database.load(`sqlite:${dbPath}`);

      // Check again if we started closing while initializing
      if (this.isClosing) {
        // Close the newly created connection and throw
        await db.close().catch(() => {});
        throw new Error('DatabaseManager closed during initialization');
      }

      // Set instance variables only after successful initialization
      this.dbPath = dbPath;
      this.db = db;

      return db;
    })();

    try {
      const db = await this.initPromise;
      // Check again in case cleanup() started while we were waiting
      if (this.isClosing) {
        throw new Error('DatabaseManager is closing, cannot get database connection');
      }
      return db;
    } catch (error) {
      // Clear initPromise on error so initialization can be retried
      this.initPromise = null;
      throw error;
    } finally {
      // Clear initPromise after completion (success or failure)
      this.initPromise = null;
    }
  }

  /**
   * Cleanup the database connection
   * Called automatically on pagehide and beforeunload events
   * Can also be called manually
   *
   * Note: In event handler contexts (pagehide, beforeunload), we cannot await
   * async operations. However, we initiate the close() operation and it will
   * be processed by the browser/Tauri. For Tauri desktop apps, the process
   * cleanup will handle any remaining connections.
   *
   * Design decision: This method intentionally does not wait for initPromise
   * to complete. During page unload, we want to immediately signal that the
   * manager is closing to prevent new operations. Any in-flight initialization
   * will be abandoned. This is acceptable because cleanup() is only called
   * during page teardown when no further operations are expected.
   */
  cleanup(): void {
    this.isClosing = true;

    // Cancel any in-flight initialization by setting initPromise to null
    // This is intentional - we don't wait for completion during page unload
    this.initPromise = null;

    if (this.db) {
      const dbToClose = this.db;
      // Set to null immediately to prevent new operations
      this.db = null;
      this.dbPath = null;

      // Close the database connection asynchronously
      // Best effort - may not complete if called during page unload
      dbToClose.close().catch((err) => {
        console.error('Error closing database:', err);
      });
    }

    // Remove event listeners to prevent memory leaks
    if (this.abortController) {
      this.abortController.abort();
      this.abortController = null;
    }
  }

  /**
   * Async version of cleanup for contexts where we can await
   * Use this when you need to ensure the connection is fully closed
   * before proceeding (e.g., in tests or programmatic cleanup)
   *
   * Note: After cleanup completes, the instance cannot be reused.
   * Use resetAsync() instead if you need to reinitialize the connection.
   */
  async cleanupAsync(): Promise<void> {
    this.isClosing = true;

    // Cancel any in-flight initialization
    const currentInitPromise = this.initPromise;
    const currentDb = this.db;
    this.initPromise = null;

    let initDb: Database | null = null;

    // Wait for any ongoing initialization to complete before closing
    if (currentInitPromise) {
      try {
        initDb = await currentInitPromise;
        // Always close the initialized connection during cleanup
        await initDb.close();
      } catch {
        // Initialization failed or was cancelled, that's ok
      }
    }

    // Close the current database if it exists and wasn't already closed above
    if (currentDb && currentDb !== initDb) {
      try {
        await currentDb.close();
      } catch (err) {
        console.error('Error closing database:', err);
      }
    }

    // Clear fields after cleanup
    this.db = null;
    this.dbPath = null;

    // Remove event listeners to prevent memory leaks
    if (this.abortController) {
      this.abortController.abort();
      this.abortController = null;
    }
  }

  /**
   * Reset the singleton instance (useful for testing or cleanup)
   */
  static reset(): void {
    if (DatabaseManager.instance) {
      DatabaseManager.instance.cleanup();
      DatabaseManager.instance = null;
    }
  }

  /**
   * Async version of reset for contexts where we can await
   */
  static async resetAsync(): Promise<void> {
    if (DatabaseManager.instance) {
      await DatabaseManager.instance.cleanupAsync();
      DatabaseManager.instance = null;
    }
  }
}
