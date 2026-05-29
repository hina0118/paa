import { useCallback, useEffect, useState } from 'react';
import {
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import { toastError, formatError } from '@/lib/toast';
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
  Database,
} from 'lucide-react';
import { Input } from '@/components/ui/input';
import { invoke } from '@tauri-apps/api/core';
import { formatDateTime } from '@/lib/utils';

type TableViewerProps = {
  tableName: string;
  title: string;
};

type ColumnInfo = {
  name: string;
  label: string;
  data_type: string;
  is_pk: boolean;
};

type TableDataResponse = {
  columns: ColumnInfo[];
  rows: Record<string, unknown>[];
  total_count: number;
  page: number;
  page_size: number;
  total_pages: number;
};

export function TableViewer({ tableName, title }: TableViewerProps) {
  const [response, setResponse] = useState<TableDataResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(0);
  const [selectedCell, setSelectedCell] = useState<{
    column: ColumnInfo;
    value: unknown;
  } | null>(null);
  const [filters, setFilters] = useState<Record<string, string>>({});
  const [sort, setSort] = useState<{
    column: string | null;
    direction: 'asc' | 'desc';
  }>({ column: null, direction: 'asc' });
  const pageSize = 50;

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
    try {
      const result = await invoke<TableDataResponse>('query_table_data', {
        params: {
          table_name: tableName,
          page,
          page_size: pageSize,
          filters: Object.entries(filters).map(([column, value]) => ({
            column,
            value,
          })),
          sort_column: sort.column ?? null,
          sort_direction: sort.direction,
        },
      });
      setResponse(result);
    } catch (err) {
      toastError(`テーブルデータの読み込みに失敗しました: ${formatError(err)}`);
      console.error('Error loading table data:', err);
    } finally {
      setLoading(false);
    }
  }, [tableName, page, pageSize, filters, sort.column, sort.direction]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  useEffect(() => {
    setFilters({});
    setSort({ column: null, direction: 'asc' });
    setPage(0);
  }, [tableName]);

  const columns = response?.columns ?? [];
  const data = response?.rows ?? [];
  const totalCount = response?.total_count ?? 0;
  const totalPages = response?.total_pages ?? 0;
  const sortColumn = sort.column;
  const sortDirection = sort.direction;
  const hasNextPage = page + 1 < totalPages;

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

  const handleCellClick = (column: ColumnInfo, value: unknown) => {
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

  if (loading && data.length === 0) {
    return (
      <div className="h-full flex flex-col">
        <div className="sticky top-0 z-10 bg-background/95 backdrop-blur border-b flex-shrink-0">
          <div className="container mx-auto px-6 py-4 flex items-center gap-3">
            <div className="p-2 rounded-lg bg-primary/10">
              <Database className="h-6 w-6 text-primary" />
            </div>
            <h1 className="text-3xl font-bold tracking-tight">{title}</h1>
          </div>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-muted-foreground">読み込み中...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <div className="sticky top-0 z-10 bg-background/95 backdrop-blur border-b flex-shrink-0">
        <div className="container mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-primary/10">
              <Database className="h-6 w-6 text-primary" />
            </div>
            <h1 className="text-3xl font-bold tracking-tight">{title}</h1>
          </div>
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
      </div>

      <div className="flex-1 min-h-0 container mx-auto px-6 py-4">
        <div className="h-full rounded-lg border shadow-sm bg-card overflow-auto">
          <table className="w-full caption-bottom text-sm">
            <TableHeader className="sticky top-0 z-10 bg-card">
              <TableRow>
                {columns.map((col) => (
                  <TableHead
                    key={col.name}
                    className="font-semibold p-1 whitespace-nowrap"
                  >
                    <button
                      type="button"
                      onClick={() => handleSort(col.name)}
                      className="flex items-center gap-1 w-full text-left hover:bg-muted/50 rounded px-1 py-0.5 min-w-0"
                      title="クリックでソート"
                    >
                      {col.label}
                      {sortColumn === col.name ? (
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
                {columns.map((col) => (
                  <TableCell key={col.name} className="p-1">
                    <Input
                      placeholder={`${col.label}で絞り込み`}
                      value={filters[col.name] ?? ''}
                      onChange={(e) => updateFilter(col.name, e.target.value)}
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
                      columns.some((c) => c.name === 'id') && row.id != null
                        ? String(row.id)
                        : index
                    }
                  >
                    {columns.map((col) => (
                      <TableCell
                        key={col.name}
                        className="max-w-xs truncate cursor-pointer hover:bg-muted/50"
                        onClick={() => handleCellClick(col, row[col.name])}
                        title="クリックして全文表示"
                      >
                        {formatValue(row[col.name])}
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
          </table>
        </div>
      </div>

      {/* Cell content dialog */}
      <Dialog
        open={selectedCell !== null}
        onOpenChange={() => setSelectedCell(null)}
      >
        <DialogContent className="max-w-3xl max-h-[80vh]">
          <DialogHeader>
            <DialogTitle>{selectedCell?.column.label}</DialogTitle>
            <DialogDescription>セルの全内容</DialogDescription>
          </DialogHeader>
          <div className="mt-4 overflow-auto max-h-[60vh]">
            <pre className="whitespace-pre-wrap break-words text-sm font-mono bg-muted p-4 rounded-md">
              {selectedCell && formatFullValue(selectedCell.value)}
            </pre>
          </div>
        </DialogContent>
      </Dialog>

      {/* Sticky pagination footer */}
      <div className="sticky bottom-0 z-10 bg-background/95 backdrop-blur border-t flex-shrink-0">
        <div className="container mx-auto px-6 py-3 flex items-center justify-between">
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
    </div>
  );
}
