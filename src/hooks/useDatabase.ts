import { useCallback } from "react";
import { DatabaseManager } from "@/lib/database";

/**
 * React hook for accessing the SQLite database
 *
 * This hook provides a safe way to access the singleton DatabaseManager instance
 * from React components. It manages the database connection lifecycle and ensures
 * proper cleanup.
 *
 * Key features:
 * - Provides access to the singleton DatabaseManager instance
 * - Returns a stable callback that won't cause unnecessary re-renders
 * - Handles database initialization and connection pooling automatically
 *
 * Singleton behavior:
 * - The DatabaseManager is a singleton that persists across component lifecycle
 * - The same database connection is shared across all components using this hook
 * - Connection is automatically cleaned up on page unload
 *
 * Caveats:
 * - The database connection is shared globally; be mindful of concurrent operations
 * - Connection cleanup happens on page unload, not on component unmount
 * - Database errors should be handled by the calling component
 *
 * @returns An object containing:
 *   - getDb: Async function that returns the Database instance
 *
 * @example
 * ```typescript
 * function MyComponent() {
 *   const { getDb } = useDatabase();
 *
 *   useEffect(() => {
 *     async function loadData() {
 *       const db = await getDb();
 *       const results = await db.select("SELECT * FROM emails");
 *       // ... use results
 *     }
 *     loadData();
 *   }, [getDb]);
 *
 *   return <div>...</div>;
 * }
 * ```
 */
export function useDatabase() {
  // Always get the current instance instead of caching in a ref
  // This ensures we get a new instance if DatabaseManager.reset() was called
  const getDb = useCallback(async () => {
    const manager = DatabaseManager.getInstance();
    return manager.getDatabase();
  }, []);

  return { getDb };
}
