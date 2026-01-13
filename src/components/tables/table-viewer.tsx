import { useCallback, useEffect, useState, useRef } from "react";
import Database from "@tauri-apps/plugin-sql";
import { path } from "@tauri-apps/api";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight, RefreshCw } from "lucide-react";

type TableViewerProps = {
  tableName: string;
  title: string;
};

type TableData = Record<string, unknown>;

type SchemaColumn = {
  cid: number;
  name: string;
  type: string;
  notnull: number;
  dflt_value: unknown;
  pk: number;
};

// Valid table names - used to prevent SQL injection
const VALID_TABLES = [
  "emails",
  "orders",
  "items",
  "images",
  "deliveries",
  "htmls",
  "order_emails",
  "order_htmls",
] as const;

type ValidTableName = typeof VALID_TABLES[number];

function isValidTableName(name: string): name is ValidTableName {
  return VALID_TABLES.includes(name as ValidTableName);
}

function sanitizeTableName(tableName: string): string {
  if (!isValidTableName(tableName)) {
    throw new Error(`Invalid table name: ${tableName}`);
  }
  // Additional safeguard: ensure table name contains only alphanumeric and underscore
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(tableName)) {
    throw new Error(`Table name contains invalid characters: ${tableName}`);
  }
  return tableName;
}

// Singleton database manager
class DatabaseManager {
  private static instance: DatabaseManager | null = null;
  private db: Database | null = null;
  private dbPath: string | null = null;
  private initPromise: Promise<Database> | null = null;

  private constructor() {
    // Register cleanup on beforeunload
    if (typeof window !== 'undefined') {
      window.addEventListener('beforeunload', () => {
        this.cleanup();
      });
    }
  }

  static getInstance(): DatabaseManager {
    if (!DatabaseManager.instance) {
      DatabaseManager.instance = new DatabaseManager();
    }
    return DatabaseManager.instance;
  }

  async getDatabase(): Promise<Database> {
    // If already initializing, wait for that promise
    if (this.initPromise) {
      return this.initPromise;
    }

    // If already initialized, return the instance
    if (this.db && this.dbPath) {
      return this.db;
    }

    // Initialize database
    this.initPromise = (async () => {
      const appDataDir = await path.appDataDir();
      this.dbPath = await path.join(appDataDir, "paa_data.db");
      this.db = await Database.load(`sqlite:${this.dbPath}`);
      return this.db;
    })();

    try {
      const db = await this.initPromise;
      return db;
    } finally {
      this.initPromise = null;
    }
  }

  /**
   * Cleanup the database connection
   * Called automatically on window beforeunload or can be called manually
   */
  cleanup(): void {
    if (this.db) {
      // Close the database connection
      // Note: close() is async but we can't await in beforeunload
      // The connection will be cleaned up by the browser/Tauri
      this.db.close().catch((err) => {
        console.error('Error closing database:', err);
      });
      this.db = null;
      this.dbPath = null;
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
}

function useDatabase() {
  const managerRef = useRef<DatabaseManager>(DatabaseManager.getInstance());

  const getDb = useCallback(async () => {
    return managerRef.current.getDatabase();
  }, []);

  return { getDb };
}

export function TableViewer({ tableName, title }: TableViewerProps) {
  const [data, setData] = useState<TableData[]>([]);
  const [columns, setColumns] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const pageSize = 50;
  const { getDb } = useDatabase();

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      console.log(`Loading table: ${tableName}`);

      // Sanitize and validate table name to prevent SQL injection
      const safeTableName = sanitizeTableName(tableName);

      // Get database connection (reused)
      const db = await getDb();

      // Get table schema
      // Note: SQLite PRAGMA statements don't support parameterized table names
      // We use sanitizeTableName above to ensure safety
      const schemaRows = await db.select<SchemaColumn[]>(
        `PRAGMA table_info(${safeTableName})`
      );
      console.log("Schema rows:", schemaRows);

      const columnNames = schemaRows.map(row => row.name);
      setColumns(columnNames);

      // Get table data with pagination
      const offset = page * pageSize;
      console.log(`Fetching data: LIMIT ${pageSize} OFFSET ${offset}`);

      // SECURITY NOTE: Table names cannot be parameterized in SQL
      // We rely on sanitizeTableName() above for validation and sanitization
      // The LIMIT and OFFSET values are properly parameterized
      const rows = await db.select<TableData[]>(
        `SELECT * FROM ${safeTableName} LIMIT ? OFFSET ?`,
        [pageSize, offset]
      );
      console.log(`Fetched ${rows.length} rows`);

      setData(rows);
    } catch (err) {
      console.error("Error loading table data:", err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [tableName, page, pageSize, getDb]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const formatValue = (value: unknown): string => {
    if (value === null || value === undefined) {
      return "-";
    }
    if (typeof value === "object") {
      return JSON.stringify(value);
    }
    if (typeof value === "boolean") {
      return value ? "true" : "false";
    }
    return String(value);
  };

  const handlePreviousPage = () => {
    if (page > 0) {
      setPage(page - 1);
    }
  };

  const handleNextPage = () => {
    if (data.length === pageSize) {
      setPage(page + 1);
    }
  };

  if (loading && data.length === 0) {
    return (
      <div className="p-8">
        <h1 className="text-3xl font-bold mb-6">{title}</h1>
        <div className="flex items-center justify-center h-64">
          <div className="text-muted-foreground">読み込み中...</div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-8">
        <h1 className="text-3xl font-bold mb-6">{title}</h1>
        <div className="flex items-center justify-center h-64">
          <div className="text-destructive">エラー: {error}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-3xl font-bold">{title}</h1>
        <Button
          variant="outline"
          size="sm"
          onClick={loadData}
          disabled={loading}
        >
          <RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
          更新
        </Button>
      </div>

      <div className="rounded-lg border shadow-sm bg-card">
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                {columns.map((column) => (
                  <TableHead key={column} className="font-semibold">
                    {column}
                  </TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.length > 0 ? (
                data.map((row, index) => (
                  <TableRow key={index}>
                    {columns.map((column) => (
                      <TableCell key={column} className="max-w-xs truncate">
                        {formatValue(row[column])}
                      </TableCell>
                    ))}
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell
                    colSpan={columns.length}
                    className="h-24 text-center text-muted-foreground"
                  >
                    データがありません
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      </div>

      <div className="flex items-center justify-between mt-4">
        <div className="text-sm text-muted-foreground">
          {data.length > 0
            ? `${page * pageSize + 1}〜${page * pageSize + data.length}件を表示`
            : "0件"}
        </div>
        <div className="flex items-center space-x-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handlePreviousPage}
            disabled={page === 0 || loading}
            className="gap-1"
          >
            <ChevronLeft className="h-4 w-4" />
            前へ
          </Button>
          <div className="text-sm text-muted-foreground px-2">
            ページ {page + 1}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleNextPage}
            disabled={data.length < pageSize || loading}
            className="gap-1"
          >
            次へ
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
