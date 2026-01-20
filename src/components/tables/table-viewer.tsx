import { useCallback, useEffect, useState } from 'react';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import { ChevronLeft, ChevronRight, RefreshCw } from 'lucide-react';
import { sanitizeTableName } from '@/lib/database';
import { useDatabase } from '@/hooks/useDatabase';

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

export function TableViewer({ tableName, title }: TableViewerProps) {
  const [data, setData] = useState<TableData[]>([]);
  const [columns, setColumns] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  // Page size of 50 rows - adequate for most tables
  // For tables with many columns or large text/blob fields, consider making this configurable
  const pageSize = 50;
  const { getDb } = useDatabase();

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // Sanitize and validate table name to prevent SQL injection
      const safeTableName = sanitizeTableName(tableName);

      // Get database connection (reused)
      const db = await getDb();

      // Get table schema
      // SECURITY NOTE: Table names cannot be parameterized in SQL (including PRAGMA statements)
      // We rely on sanitizeTableName() whitelist validation for SQL injection protection
      // This is a known limitation - no alternative parameterization exists for table identifiers
      // The whitelist approach provides strong security as only pre-defined tables are accessible
      const schemaRows = await db.select<SchemaColumn[]>(
        `PRAGMA table_info(${safeTableName})`
      );

      // Check if table exists and has columns
      // Note: This is different from sanitizeTableName() validation above:
      // - sanitizeTableName() checks if the table name is in the VALID_TABLES whitelist
      // - This check verifies the table actually exists in the database
      // A table could be in the whitelist but not yet created in the DB
      if (!schemaRows || schemaRows.length === 0) {
        throw new Error(
          `Table "${safeTableName}" does not exist or has no columns`
        );
      }

      const columnNames = schemaRows.map((row) => row.name);
      setColumns(columnNames);

      // Get total count for pagination
      const countResult = await db.select<Array<{ count: number }>>(
        `SELECT COUNT(*) as count FROM ${safeTableName}`
      );
      const total = countResult[0]?.count || 0;
      setTotalCount(total);

      // Get table data with pagination
      const offset = page * pageSize;

      // Table name validated by sanitizeTableName() above (see security note)
      // LIMIT and OFFSET values are properly parameterized
      const rows = await db.select<TableData[]>(
        `SELECT * FROM ${safeTableName} LIMIT ? OFFSET ?`,
        [pageSize, offset]
      );

      setData(rows);
    } catch (err) {
      console.error('Error loading table data:', err);
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
      return '-';
    }
    if (typeof value === 'object') {
      return JSON.stringify(value);
    }
    if (typeof value === 'boolean') {
      return value ? 'true' : 'false';
    }
    return String(value);
  };

  const handlePreviousPage = () => {
    if (page > 0) {
      setPage(page - 1);
    }
  };

  const handleNextPage = () => {
    const nextPageStart = (page + 1) * pageSize;
    if (nextPageStart < totalCount) {
      setPage(page + 1);
    }
  };

  const totalPages = Math.ceil(totalCount / pageSize);
  const hasNextPage = page + 1 < totalPages;

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
          <RefreshCw
            className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`}
          />
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
          {totalCount > 0
            ? `${page * pageSize + 1}〜${page * pageSize + data.length}件を表示 / 全${totalCount}件`
            : '0件'}
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
            ページ {page + 1} / {totalPages}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleNextPage}
            disabled={!hasNextPage || loading}
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
