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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  RefreshCw,
} from 'lucide-react';
import { Input } from '@/components/ui/input';
import { sanitizeTableName } from '@/lib/database';
import { formatDateTime } from '@/lib/utils';
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

const IMAGES_COLUMN_LABELS: Record<string, string> = {
  id: 'ID',
  item_id: '商品ID',
  file_name: 'ファイル名',
  created_at: '作成日時',
};

function getColumnLabel(tableName: string, column: string): string {
  if (tableName === 'images' && column in IMAGES_COLUMN_LABELS) {
    return IMAGES_COLUMN_LABELS[column];
  }
  return column;
}

function quoteColumn(name: string): string {
  return `"${name.replace(/"/g, '""')}"`;
}

export function TableViewer({ tableName, title }: TableViewerProps) {
  const [data, setData] = useState<TableData[]>([]);
  const [columns, setColumns] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  const [selectedCell, setSelectedCell] = useState<{
    column: string;
    value: unknown;
  } | null>(null);
  const [filters, setFilters] = useState<Record<string, string>>({});
  const [sort, setSort] = useState<{
    column: string | null;
    direction: 'asc' | 'desc';
  }>({ column: null, direction: 'asc' });
  // Page size of 50 rows - adequate for most tables
  const pageSize = 50;
  const { getDb } = useDatabase();

  const updateFilter = useCallback((column: string, value: string) => {
    setFilters((prev) => {
      const next = { ...prev };
      if (value.trim() === '') {
        delete next[column];
      } else {
        next[column] = value;
      }
      return next;
    });
    setPage(0);
  }, []);

  const handleSort = useCallback((column: string) => {
    setSort((prev) =>
      prev.column === column
        ? { column, direction: prev.direction === 'asc' ? 'desc' : 'asc' }
        : { column, direction: 'asc' }
    );
    setPage(0);
  }, []);

  const clearFiltersAndSort = useCallback(() => {
    setFilters({});
    setSort({ column: null, direction: 'asc' });
    setPage(0);
  }, []);

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

      // Build WHERE clause (column names from schema whitelist only)
      const whereParts: string[] = [];
      const countArgs: unknown[] = [];
      const filterColumns = Object.keys(filters).filter(
        (col) => columnNames.includes(col) && filters[col]?.trim()
      );
      for (const col of filterColumns) {
        // SQLite LIKE: ESCAPE '!' 指定時はエスケープ文字は ! のみ。
        // % _ ! を !% !_ !! にエスケープ。バックスラッシュはエスケープ文字でないため常にリテラル（https://sqlite.org/lang_expr.html#like）
        whereParts.push(`${quoteColumn(col)} LIKE ? ESCAPE '!'`);
        const escaped = String(filters[col])
          .replace(/!/g, '!!')
          .replace(/%/g, '!%')
          .replace(/_/g, '!_');
        countArgs.push(`%${escaped}%`);
      }
      const whereClause =
        whereParts.length > 0 ? `WHERE ${whereParts.join(' AND ')}` : '';

      // Get total count for pagination (with filters)
      const countSql = `SELECT COUNT(*) as count FROM ${safeTableName} ${whereClause}`;
      const countResult =
        countArgs.length > 0
          ? await db.select<Array<{ count: number }>>(countSql, countArgs)
          : await db.select<Array<{ count: number }>>(countSql);
      const total = countResult[0]?.count || 0;
      setTotalCount(total);

      // Build ORDER BY (column from schema whitelist only)
      let orderClause = '';
      if (sort.column && columnNames.includes(sort.column)) {
        orderClause = `ORDER BY ${quoteColumn(sort.column)} ${sort.direction.toUpperCase()}`;
      }

      // Get table data with pagination
      const offset = page * pageSize;
      const dataArgs = [...countArgs, pageSize, offset];
      const dataSql = `SELECT * FROM ${safeTableName} ${whereClause} ${orderClause} LIMIT ? OFFSET ?`;
      const rows = await db.select<TableData[]>(dataSql, dataArgs);

      setData(rows);
    } catch (err) {
      console.error('Error loading table data:', err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [tableName, page, pageSize, filters, sort.column, sort.direction, getDb]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    setFilters({});
    setSort({ column: null, direction: 'asc' });
    setPage(0);
  }, [tableName]);

  const sortColumn = sort.column;
  const sortDirection = sort.direction;

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

  const formatFullValue = (value: unknown): string => {
    if (value === null || value === undefined) {
      return '(null)';
    }
    if (typeof value === 'object') {
      return JSON.stringify(value, null, 2);
    }
    if (typeof value === 'boolean') {
      return value ? 'true' : 'false';
    }
    const s = String(value);
    // ISO 8601 日付/日時のみ対象。T/Z/空白/終端のいずれかで区切られる（"2024-01-01-backup" 等は除外）
    if (/^\d{4}-\d{2}-\d{2}(?:[T\sZ]|$)/.test(s)) {
      return formatDateTime(s);
    }
    return s;
  };

  const handleCellClick = (column: string, value: unknown) => {
    setSelectedCell({ column, value });
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
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={clearFiltersAndSort}
            disabled={
              loading ||
              (Object.keys(filters).length === 0 && sortColumn === null)
            }
          >
            フィルター・ソートをクリア
          </Button>
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
      </div>

      <div className="rounded-lg border shadow-sm bg-card">
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                {columns.map((column) => (
                  <TableHead key={column} className="font-semibold p-1">
                    <button
                      type="button"
                      onClick={() => handleSort(column)}
                      className="flex items-center gap-1 w-full text-left hover:bg-muted/50 rounded px-1 py-0.5 min-w-0"
                      title="クリックでソート"
                    >
                      {getColumnLabel(tableName, column)}
                      {sortColumn === column ? (
                        sortDirection === 'asc' ? (
                          <ChevronUp className="h-4 w-4 shrink-0" />
                        ) : (
                          <ChevronDown className="h-4 w-4 shrink-0" />
                        )
                      ) : null}
                    </button>
                  </TableHead>
                ))}
              </TableRow>
              <TableRow className="border-b bg-muted/30">
                {columns.map((column) => (
                  <TableCell key={column} className="p-1">
                    <Input
                      placeholder={`${getColumnLabel(tableName, column)}で絞り込み`}
                      value={filters[column] ?? ''}
                      onChange={(e) => updateFilter(column, e.target.value)}
                      className="h-8 text-sm"
                      type="text"
                    />
                  </TableCell>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.length > 0 ? (
                data.map((row, index) => (
                  <TableRow
                    key={
                      columns.includes('id') && row.id != null
                        ? String(row.id)
                        : index
                    }
                  >
                    {columns.map((column) => (
                      <TableCell
                        key={column}
                        className="max-w-xs truncate cursor-pointer hover:bg-muted/50"
                        onClick={() => handleCellClick(column, row[column])}
                        title="クリックして全文表示"
                      >
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

      {/* Cell content dialog */}
      <Dialog
        open={selectedCell !== null}
        onOpenChange={() => setSelectedCell(null)}
      >
        <DialogContent className="max-w-3xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>
              {selectedCell && getColumnLabel(tableName, selectedCell.column)}
            </DialogTitle>
            <DialogDescription>セルの全内容</DialogDescription>
          </DialogHeader>
          <div className="mt-4 overflow-auto max-h-[60vh]">
            <pre className="whitespace-pre-wrap break-words text-sm font-mono bg-muted p-4 rounded-md">
              {selectedCell && formatFullValue(selectedCell.value)}
            </pre>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
