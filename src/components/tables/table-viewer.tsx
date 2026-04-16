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

const COLUMN_LABELS: Record<string, Record<string, string>> = {
  emails: {
    id: 'ID',
    message_id: 'メッセージID',
    body_plain: '本文（プレーン）',
    body_html: '本文（HTML）',
    analysis_status: '解析ステータス',
    internal_date: '内部日時',
    from_address: '送信元アドレス',
    subject: '件名',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  orders: {
    id: 'ID',
    shop_domain: 'ショップドメイン',
    shop_name: 'ショップ名',
    order_number: '注文番号',
    order_date: '注文日',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  items: {
    id: 'ID',
    order_id: '注文ID',
    item_name: '商品名',
    item_name_normalized: '正規化商品名',
    price: '価格',
    quantity: '数量',
    category: 'カテゴリ',
    brand: 'ブランド',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  images: {
    id: 'ID',
    item_name_normalized: '正規化商品名',
    file_name: 'ファイル名',
    created_at: '作成日時',
  },
  deliveries: {
    id: 'ID',
    order_id: '注文ID',
    tracking_number: '追跡番号',
    carrier: '配送業者',
    delivery_status: '配送ステータス',
    estimated_delivery: '配送予定日',
    actual_delivery: '実際の配送日',
    last_checked_at: '最終確認日時',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  htmls: {
    id: 'ID',
    url: 'URL',
    html_content: 'HTML内容',
    analysis_status: '解析ステータス',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  order_emails: {
    id: 'ID',
    order_id: '注文ID',
    email_id: 'メールID',
    created_at: '作成日時',
  },
  order_htmls: {
    id: 'ID',
    order_id: '注文ID',
    html_id: 'HTML ID',
    created_at: '作成日時',
  },
  shop_settings: {
    id: 'ID',
    shop_name: 'ショップ名',
    sender_address: '送信元アドレス',
    parser_type: 'パーサー種別',
    is_enabled: '有効フラグ',
    subject_filters: '件名フィルター',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  product_master: {
    id: 'ID',
    raw_name: '元の名称',
    normalized_name: '正規化名称',
    maker: 'メーカー',
    series: 'シリーズ',
    product_name: '商品名',
    scale: 'スケール',
    is_reissue: '再版フラグ',
    platform_hint: 'プラットフォームヒント',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  item_overrides: {
    id: 'ID',
    shop_domain: 'ショップドメイン',
    order_number: '注文番号',
    original_item_name: '元の商品名',
    original_brand: '元のブランド',
    item_name: '商品名',
    price: '価格',
    quantity: '数量',
    brand: 'ブランド',
    category: 'カテゴリ',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  order_overrides: {
    id: 'ID',
    shop_domain: 'ショップドメイン',
    order_number: '注文番号',
    new_order_number: '新注文番号',
    order_date: '注文日',
    shop_name: 'ショップ名',
    created_at: '作成日時',
    updated_at: '更新日時',
  },
  excluded_items: {
    id: 'ID',
    shop_domain: 'ショップドメイン',
    order_number: '注文番号',
    item_name: '商品名',
    brand: 'ブランド',
    reason: '除外理由',
    created_at: '作成日時',
  },
  excluded_orders: {
    id: 'ID',
    shop_domain: 'ショップドメイン',
    order_number: '注文番号',
    reason: '除外理由',
    created_at: '作成日時',
  },
  tracking_check_logs: {
    id: 'ID',
    tracking_number: '追跡番号',
    checked_at: '確認日時',
    check_status: 'チェック結果',
    delivery_status: '配送ステータス',
    description: '説明',
    location: '場所',
    error_message: 'エラーメッセージ',
    created_at: '作成日時',
  },
  news_clips: {
    id: 'ID',
    title: 'タイトル',
    url: 'URL',
    source_name: '情報源',
    published_at: '公開日時',
    summary: '要約',
    tags: 'タグ',
    clipped_at: 'クリップ日時',
  },
  item_exclusion_patterns: {
    id: 'ID',
    shop_domain: 'ショップドメイン',
    keyword: 'キーワード',
    match_type: 'マッチ種別',
    note: 'メモ',
    created_at: '作成日時',
  },
};

function getColumnLabel(tableName: string, column: string): string {
  const labels = COLUMN_LABELS[tableName];
  if (labels && column in labels) {
    return labels[column];
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
      toastError(`テーブルデータの読み込みに失敗しました: ${formatError(err)}`);
      console.error('Error loading table data:', err);
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
                {columns.map((column) => (
                  <TableHead
                    key={column}
                    className="font-semibold p-1 whitespace-nowrap"
                  >
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
