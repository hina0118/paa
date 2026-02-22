import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { BookOpen, ChevronLeft, ChevronRight, X } from 'lucide-react';
import { PageHeader } from '@/components/ui/page-header';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { toastSuccess, toastError } from '@/lib/toast';

// ---- 型定義 ----

interface ProductMaster {
  id: number;
  raw_name: string;
  normalized_name: string;
  maker: string | null;
  series: string | null;
  product_name: string | null;
  scale: string | null;
  is_reissue: boolean;
  platform_hint: string | null;
  created_at: string;
  updated_at: string;
}

interface FilterParams {
  raw_name: string;
  maker: string;
  series: string;
  product_name: string;
  scale: string;
  is_reissue: '' | 'true' | 'false';
}

interface EditForm {
  maker: string;
  series: string;
  product_name: string;
  scale: string;
  is_reissue: boolean;
}

interface ProductMasterListResponse {
  items: ProductMaster[];
  total: number;
}

const EMPTY_FILTER: FilterParams = {
  raw_name: '',
  maker: '',
  series: '',
  product_name: '',
  scale: '',
  is_reissue: '',
};

const PAGE_SIZE = 50;

// ---- フック ----

function useProductMasterList(filter: FilterParams, page: number) {
  const [items, setItems] = useState<ProductMaster[]>([]);
  const [total, setTotal] = useState(0);
  const [isLoading, setIsLoading] = useState(false);

  const load = useCallback(async () => {
    setIsLoading(true);
    try {
      const filterArg = {
        raw_name: filter.raw_name || null,
        maker: filter.maker || null,
        series: filter.series || null,
        product_name: filter.product_name || null,
        scale: filter.scale || null,
        is_reissue:
          filter.is_reissue === 'true'
            ? true
            : filter.is_reissue === 'false'
              ? false
              : null,
      };
      const res = await invoke<ProductMasterListResponse>(
        'get_product_master_list',
        {
          filter: filterArg,
          limit: PAGE_SIZE,
          offset: (page - 1) * PAGE_SIZE,
        }
      );
      setItems(res.items);
      setTotal(res.total);
    } catch (e) {
      toastError(`読み込みに失敗しました: ${e}`);
    } finally {
      setIsLoading(false);
    }
  }, [filter, page]);

  useEffect(() => {
    load();
  }, [load]);

  return { items, total, isLoading, reload: load };
}

// ---- メインコンポーネント ----

export function ProductMasterEdit() {
  const [filter, setFilter] = useState<FilterParams>(EMPTY_FILTER);
  const [debouncedFilter, setDebouncedFilter] =
    useState<FilterParams>(EMPTY_FILTER);
  const [page, setPage] = useState(1);

  const { items, total, isLoading, reload } = useProductMasterList(
    debouncedFilter,
    page
  );

  // フィルター変更時: page をリセットして 300ms デバウンス
  useEffect(() => {
    setPage(1);
    const timer = setTimeout(() => setDebouncedFilter(filter), 300);
    return () => clearTimeout(timer);
  }, [filter]);

  // 編集状態
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editForm, setEditForm] = useState<EditForm>({
    maker: '',
    series: '',
    product_name: '',
    scale: '',
    is_reissue: false,
  });

  const startEdit = (row: ProductMaster) => {
    setEditingId(row.id);
    setEditForm({
      maker: row.maker ?? '',
      series: row.series ?? '',
      product_name: row.product_name ?? '',
      scale: row.scale ?? '',
      is_reissue: row.is_reissue,
    });
  };

  const cancelEdit = () => setEditingId(null);

  const saveEdit = async (id: number) => {
    try {
      await invoke('update_product_master', {
        id,
        maker: editForm.maker || null,
        series: editForm.series || null,
        productName: editForm.product_name,
        scale: editForm.scale || null,
        isReissue: editForm.is_reissue,
      });
      toastSuccess('保存しました');
      setEditingId(null);
      reload();
    } catch (e) {
      toastError(`保存に失敗しました: ${e}`);
    }
  };

  const updateFilter = (key: keyof FilterParams, value: string) => {
    setFilter((prev) => ({ ...prev, [key]: value }));
  };

  const clearFilter = () => setFilter(EMPTY_FILTER);

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const hasFilter = Object.entries(filter).some(([, v]) => v !== '');

  return (
    <div className="h-full flex flex-col">
      <PageHeader title="商品マスタ編集" icon={BookOpen} />

      {/* スクロール領域: table-viewer と同じ構造 */}
      <div className="flex-1 min-h-0 container mx-auto px-6 py-4">
        <div className="h-full rounded-lg border shadow-sm bg-card overflow-auto">
          <table className="w-full caption-bottom text-sm">
            <TableHeader className="sticky top-0 z-10 bg-card">
              {/* カラム名行 */}
              <TableRow>
                <TableHead className="w-64">元の商品名</TableHead>
                <TableHead className="w-32">メーカー</TableHead>
                <TableHead className="w-32">シリーズ</TableHead>
                <TableHead className="w-48">商品名</TableHead>
                <TableHead className="w-24">スケール</TableHead>
                <TableHead className="w-16 text-center">再販</TableHead>
                <TableHead className="w-24" />
              </TableRow>

              {/* フィルター行 */}
              <TableRow className="bg-muted/30 hover:bg-muted/30">
                <TableHead className="py-1.5">
                  <Input
                    placeholder="絞り込み…"
                    value={filter.raw_name}
                    onChange={(e) => updateFilter('raw_name', e.target.value)}
                    className="h-7 text-xs"
                  />
                </TableHead>
                <TableHead className="py-1.5">
                  <Input
                    placeholder="絞り込み…"
                    value={filter.maker}
                    onChange={(e) => updateFilter('maker', e.target.value)}
                    className="h-7 text-xs"
                  />
                </TableHead>
                <TableHead className="py-1.5">
                  <Input
                    placeholder="絞り込み…"
                    value={filter.series}
                    onChange={(e) => updateFilter('series', e.target.value)}
                    className="h-7 text-xs"
                  />
                </TableHead>
                <TableHead className="py-1.5">
                  <Input
                    placeholder="絞り込み…"
                    value={filter.product_name}
                    onChange={(e) =>
                      updateFilter('product_name', e.target.value)
                    }
                    className="h-7 text-xs"
                  />
                </TableHead>
                <TableHead className="py-1.5">
                  <Input
                    placeholder="絞り込み…"
                    value={filter.scale}
                    onChange={(e) => updateFilter('scale', e.target.value)}
                    className="h-7 text-xs"
                  />
                </TableHead>
                <TableHead className="py-1.5 text-center">
                  <select
                    value={filter.is_reissue}
                    onChange={(e) =>
                      updateFilter(
                        'is_reissue',
                        e.target.value as FilterParams['is_reissue']
                      )
                    }
                    className="w-full h-7 rounded-md border border-input bg-background px-1 text-xs"
                  >
                    <option value="">全て</option>
                    <option value="true">再販</option>
                    <option value="false">通常</option>
                  </select>
                </TableHead>
                <TableHead className="py-1.5 text-right">
                  {hasFilter && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={clearFilter}
                      className="h-7 px-2 text-xs text-muted-foreground"
                    >
                      <X className="h-3 w-3 mr-1" />
                      クリア
                    </Button>
                  )}
                </TableHead>
              </TableRow>
            </TableHeader>

            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell
                    colSpan={7}
                    className="h-24 text-center text-muted-foreground"
                  >
                    読み込み中…
                  </TableCell>
                </TableRow>
              ) : items.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={7}
                    className="h-24 text-center text-muted-foreground"
                  >
                    データがありません
                  </TableCell>
                </TableRow>
              ) : (
                items.map((row) =>
                  editingId === row.id ? (
                    // ---- 編集行 ----
                    <TableRow key={row.id} className="bg-primary/5">
                      <TableCell className="text-xs text-muted-foreground truncate max-w-64">
                        {row.raw_name}
                      </TableCell>
                      <TableCell className="py-1.5">
                        <Input
                          value={editForm.maker}
                          onChange={(e) =>
                            setEditForm((f) => ({
                              ...f,
                              maker: e.target.value,
                            }))
                          }
                          className="h-7 text-xs"
                        />
                      </TableCell>
                      <TableCell className="py-1.5">
                        <Input
                          value={editForm.series}
                          onChange={(e) =>
                            setEditForm((f) => ({
                              ...f,
                              series: e.target.value,
                            }))
                          }
                          className="h-7 text-xs"
                        />
                      </TableCell>
                      <TableCell className="py-1.5">
                        <Input
                          value={editForm.product_name}
                          onChange={(e) =>
                            setEditForm((f) => ({
                              ...f,
                              product_name: e.target.value,
                            }))
                          }
                          className="h-7 text-xs"
                        />
                      </TableCell>
                      <TableCell className="py-1.5">
                        <Input
                          value={editForm.scale}
                          onChange={(e) =>
                            setEditForm((f) => ({
                              ...f,
                              scale: e.target.value,
                            }))
                          }
                          className="h-7 text-xs"
                        />
                      </TableCell>
                      <TableCell className="text-center py-1.5">
                        <Checkbox
                          checked={editForm.is_reissue}
                          onCheckedChange={(checked) =>
                            setEditForm((f) => ({
                              ...f,
                              is_reissue: checked === true,
                            }))
                          }
                        />
                      </TableCell>
                      <TableCell className="py-1.5">
                        <div className="flex gap-1 justify-end">
                          <Button
                            size="sm"
                            className="h-7 px-2 text-xs"
                            onClick={() => saveEdit(row.id)}
                          >
                            保存
                          </Button>
                          <Button
                            size="sm"
                            variant="ghost"
                            className="h-7 px-2 text-xs"
                            onClick={cancelEdit}
                          >
                            キャンセル
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ) : (
                    // ---- 通常行 ----
                    <TableRow key={row.id}>
                      <TableCell
                        className="text-xs truncate max-w-64"
                        title={row.raw_name}
                      >
                        {row.raw_name}
                      </TableCell>
                      <TableCell className="text-xs">
                        {row.maker ?? '—'}
                      </TableCell>
                      <TableCell className="text-xs">
                        {row.series ?? '—'}
                      </TableCell>
                      <TableCell className="text-xs">
                        {row.product_name ?? '—'}
                      </TableCell>
                      <TableCell className="text-xs">
                        {row.scale ?? '—'}
                      </TableCell>
                      <TableCell className="text-center">
                        <Checkbox checked={row.is_reissue} disabled />
                      </TableCell>
                      <TableCell className="text-right">
                        <Button
                          size="sm"
                          variant="outline"
                          className="h-7 px-2 text-xs"
                          onClick={() => startEdit(row)}
                        >
                          編集
                        </Button>
                      </TableCell>
                    </TableRow>
                  )
                )
              )}
            </TableBody>
          </table>
        </div>
      </div>

      {/* ページネーション: table-viewer と同じ sticky フッター */}
      <div className="sticky bottom-0 z-10 bg-background/95 backdrop-blur border-t flex-shrink-0">
        <div className="container mx-auto px-6 py-3 flex items-center justify-between">
          <div className="text-sm text-muted-foreground">
            {total > 0
              ? `${(page - 1) * PAGE_SIZE + 1}〜${Math.min(page * PAGE_SIZE, total)}件を表示 / 全${total}件`
              : '0件'}
          </div>
          <div className="flex items-center space-x-2">
            <Button
              variant="outline"
              size="sm"
              disabled={page <= 1 || isLoading}
              onClick={() => setPage((p) => p - 1)}
              className="gap-1"
            >
              <ChevronLeft className="h-4 w-4" />
              前へ
            </Button>
            <div className="text-sm text-muted-foreground px-2">
              ページ {page} / {totalPages}
            </div>
            <Button
              variant="outline"
              size="sm"
              disabled={page >= totalPages || isLoading}
              onClick={() => setPage((p) => p + 1)}
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
