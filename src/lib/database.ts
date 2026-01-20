import Database from '@tauri-apps/plugin-sql';
import { appDataDir, join } from '@tauri-apps/api/path';

// Valid table names - used to prevent SQL injection
export const VALID_TABLES = [
  'emails',
  'orders',
  'items',
  'images',
  'deliveries',
  'htmls',
  'order_emails',
  'order_htmls',
] as const;

export type ValidTableName = (typeof VALID_TABLES)[number];

export function isValidTableName(name: string): name is ValidTableName {
  return VALID_TABLES.includes(name as ValidTableName);
}

export function sanitizeTableName(tableName: string): string {
  if (!isValidTableName(tableName)) {
    throw new Error(
      `Table "${tableName}" is not allowed. ` +
        `Allowed tables are: ${VALID_TABLES.join(', ')}. ` +
        `This may indicate a configuration issue or a bug in the calling code.`
    );
  }
  // The whitelist check above is sufficient since VALID_TABLES is a const array
  // containing only safe, pre-validated table names
  return tableName;
}

// Error messages for DatabaseManager
const ERROR_MANAGER_CLOSING =
  'DatabaseManager is closing, cannot get database connection';
const ERROR_CLOSED_DURING_INIT = 'DatabaseManager closed during initialization';

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
  private initPromise: Promise<Database> | null = null;
  private isClosing = false;
  private abortController = new AbortController();

  private constructor() {
    // Register cleanup handlers with AbortController for proper cleanup
    const signal = this.abortController.signal;

    // Use pagehide for more reliable cleanup (works on mobile Safari and modern browsers)
    window.addEventListener(
      'pagehide',
      () => {
        this.cleanup();
      },
      { signal }
    );

    // Also register beforeunload as fallback for older browsers
    // Note: beforeunload cannot reliably complete async operations
    // The database connection will be cleaned up by Tauri/browser when the process ends
    window.addEventListener(
      'beforeunload',
      () => {
        this.cleanup();
      },
      { signal }
    );

    // Note: visibilitychange listener removed as it causes issues:
    // - Closes connection when user switches tabs, requiring reconnection
    // - Can interrupt ongoing operations
    // - Connection cleanup on page unload is sufficient for Tauri desktop apps
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
      throw new Error(ERROR_MANAGER_CLOSING);
    }

    // If already initialized, verify it's still valid and return
    if (this.db && !this.isClosing) {
      return this.db;
    }

    // If already initializing, wait for that promise
    if (this.initPromise) {
      const db = await this.initPromise;
      // Check again in case cleanup() started while we were waiting
      if (this.isClosing) {
        throw new Error(ERROR_MANAGER_CLOSING);
      }
      // Note: initPromise is cleared by the outer finally block below
      return db;
    }

    // Initialize database
    this.initPromise = (async () => {
      const appDataDirPath = await appDataDir();
      const dbPath = await join(appDataDirPath, 'paa_data.db');
      const db = await Database.load(`sqlite:${dbPath}`);

      // Check again if we started closing while initializing
      if (this.isClosing) {
        // Close the newly created connection and throw
        await db.close().catch((err) => {
          console.error(
            'Failed to close database during initialization cleanup:',
            err
          );
        });
        throw new Error(ERROR_CLOSED_DURING_INIT);
      }

      // Set instance variable only after successful initialization
      this.db = db;

      return db;
    })();

    try {
      const db = await this.initPromise;
      // Check again in case cleanup() started while we were waiting
      if (this.isClosing) {
        throw new Error(ERROR_MANAGER_CLOSING);
      }
      return db;
    } finally {
      // Clear initPromise after completion (success or failure)
      // This allows initialization to be retried if there was an error
      this.initPromise = null;
    }
  }

  /**
   * Cleanup the database connection
   * Called automatically on pagehide and beforeunload events
   * Can also be called manually
   *
   * IMPORTANT: After cleanup, the instance becomes permanently unusable.
   * The isClosing flag is never reset, preventing future database operations.
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
    // Set isClosing to true permanently - this instance cannot be reused
    this.isClosing = true;

    // Cancel any in-flight initialization by setting initPromise to null
    // This is intentional - we don't wait for completion during page unload
    this.initPromise = null;

    if (this.db) {
      const dbToClose = this.db;
      // Set to null immediately to prevent new operations
      this.db = null;

      // Close the database connection asynchronously
      // Best effort - may not complete if called during page unload
      dbToClose.close().catch((err) => {
        console.error('Error closing database:', err);
      });
    }

    // Remove event listeners to prevent memory leaks
    this.abortController.abort();
  }

  /**
   * Async version of cleanup for contexts where we can await
   * Use this when you need to ensure the connection is fully closed
   * before proceeding (e.g., in tests or programmatic cleanup)
   *
   * IMPORTANT: After cleanup completes, the instance becomes permanently unusable.
   * The isClosing flag is never reset to false, preventing any future database operations.
   * To reinitialize the connection, you must call DatabaseManager.resetAsync() to create
   * a new instance, rather than reusing the cleaned-up instance.
   */
  async cleanupAsync(): Promise<void> {
    // Set isClosing to true permanently - this instance cannot be reused
    // Any subsequent calls to getDatabase() will throw an error
    this.isClosing = true;

    // Capture references to current state
    const currentInitPromise = this.initPromise;
    const currentDb = this.db;

    // Immediately clear instance fields to prevent race conditions
    // This ensures getDatabase() cannot access these after cleanup starts
    this.initPromise = null;
    this.db = null;

    let initDb: Database | null = null;

    // Wait for any ongoing initialization to complete before closing
    if (currentInitPromise) {
      try {
        initDb = await currentInitPromise;
        // Always close the initialized connection during cleanup
        await initDb.close();
      } catch (err) {
        // Initialization failed or was cancelled; log for troubleshooting
        console.error('Error during database initialization cleanup:', err);
      }
    }

    // Close the current database if it exists and wasn't already closed above
    // Note: currentDb === initDb can occur when initialization just completed and
    // both fields point to the same Database instance. The !== check prevents double-closing.
    if (currentDb && currentDb !== initDb) {
      try {
        await currentDb.close();
      } catch (err) {
        console.error('Error closing database:', err);
      }
    }

    // Remove event listeners to prevent memory leaks
    this.abortController.abort();
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
