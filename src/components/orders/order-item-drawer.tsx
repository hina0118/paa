import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { StatusBadge } from './status-badge';
import { ImageSearchDialog } from './image-search-dialog';
import { useImageUrl } from '@/hooks/useImageUrl';
import type { OrderItemRow } from '@/lib/types';
import { CLIPBOARD_URL_DETECTED_EVENT } from '@/lib/tauri-events';
import { formatDate, formatPrice } from '@/lib/utils';
import { toastWarning, formatError } from '@/lib/toast';
import { Search, Pencil, Trash2, X } from 'lucide-react';

type OrderItemDrawerProps = {
  item: OrderItemRow | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImageUpdated?: () => void;
  onDataChanged?: () => void;
};

type EditForm = {
  itemName: string;
  price: string;
  quantity: string;
  brand: string;
  orderNumber: string;
  orderDate: string;
  shopName: string;
};

function normalizeToDateInput(value: string | null | undefined): string {
  if (!value) return '';
  // `type="date"` は YYYY-MM-DD 形式のみ許容
  return value.length >= 10 ? value.slice(0, 10) : value;
}

function parseRequiredInt(value: string, label: string): { value: number } {
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error(`${label}を入力してください`);
  }
  const n = Number(trimmed);
  if (!Number.isFinite(n) || !Number.isInteger(n)) {
    throw new Error(`${label}は整数で入力してください`);
  }
  return { value: n };
}

type ClipboardUrlDetectedPayload = {
  url: string;
  kind: 'image_url' | 'url';
  source: 'clipboard';
  detectedAt: string;
};

export function OrderItemDrawer({
  item,
  open,
  onOpenChange,
  onImageUpdated,
  onDataChanged,
}: OrderItemDrawerProps) {
  const getImageUrl = useImageUrl();
  const [imageSearchOpen, setImageSearchOpen] = useState(false);
  const [imageSearchInitialUrl, setImageSearchInitialUrl] = useState<
    string | undefined
  >(undefined);
  const [imageKey, setImageKey] = useState(0);
  const [isEditing, setIsEditing] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [excludeDialogOpen, setExcludeDialogOpen] = useState(false);
  const [validationError, setValidationError] = useState<string | null>(null);
  const [form, setForm] = useState<EditForm>({
    itemName: '',
    price: '',
    quantity: '',
    brand: '',
    orderNumber: '',
    orderDate: '',
    shopName: '',
  });

  const imageSrc = item ? getImageUrl(item.fileName) : null;
  const itemId = item?.id;

  // 編集モード開始時にフォームを初期化
  useEffect(() => {
    if (isEditing && item) {
      setForm({
        itemName: item.itemName ?? '',
        price: String(item.price ?? 0),
        quantity: String(item.quantity ?? 1),
        brand: item.brand ?? '',
        orderNumber: item.orderNumber ?? '',
        orderDate: normalizeToDateInput(item.orderDate),
        shopName: item.shopName ?? '',
      });
      setValidationError(null);
    }
  }, [isEditing, item]);

  // ドロワーが閉じたら編集モードを解除
  useEffect(() => {
    if (!open) {
      setIsEditing(false);
      setValidationError(null);
      setImageSearchInitialUrl(undefined);
    }
  }, [open]);

  const openImageSearchDialog = useCallback(
    (initialUrl?: string) => {
      const resolved =
        initialUrl?.trim() || imageSearchInitialUrl?.trim() || undefined;
      setImageSearchInitialUrl(resolved || undefined);
      setImageSearchOpen(true);
    },
    [imageSearchInitialUrl]
  );

  // drawerが開いている間だけ、クリップボード検知イベントを購読する
  useEffect(() => {
    if (!open || !itemId) return;

    let unlisten: null | (() => void) = null;
    let cancelled = false;

    void (async () => {
      try {
        const stop = await listen<ClipboardUrlDetectedPayload>(
          CLIPBOARD_URL_DETECTED_EVENT,
          (event) => {
            const payload = event.payload;
            if (!payload || payload.kind !== 'image_url') return;
            const url = payload.url.trim();

            // ダイアログに集約: 検知したら初期URLをセットして自動で開く
            setImageSearchInitialUrl(url);
            setImageSearchOpen(true);
          }
        );

        if (cancelled) {
          stop();
          return;
        }
        unlisten = stop;
      } catch (e) {
        toastWarning(
          `クリップボード監視イベントの購読に失敗しました: ${formatError(e)}`
        );
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [open, itemId]);

  const handleImageSaved = useCallback(() => {
    onImageUpdated?.();
    setImageKey((prev) => prev + 1);
  }, [onImageUpdated]);

  const handleSave = useCallback(async () => {
    if (!item || !item.shopDomain || !item.originalOrderNumber) return;
    setIsSaving(true);
    try {
      setValidationError(null);

      const { value: nextPrice } = parseRequiredInt(form.price, '価格');
      const { value: nextQuantity } = parseRequiredInt(form.quantity, '数量');
      if (nextPrice < 0) {
        throw new Error('価格は 0 以上で入力してください');
      }
      if (nextQuantity < 1) {
        throw new Error('数量は 1 以上で入力してください');
      }

      if (!item.originalItemName) {
        throw new Error('商品情報が不足しているため保存できません');
      }

      const baseItemName = item.originalItemName ?? item.itemName;
      const baseBrand = item.originalBrand ?? item.brand ?? '';
      const basePrice = item.originalPrice ?? item.price;
      const baseQuantity = item.originalQuantity ?? item.quantity;

      // 「元に戻す」判定のベースは必ず original を使う
      // original が NULL の場合は空文字へ正規化して比較する
      const baseOrderNumber = item.originalOrderNumber;
      const baseOrderDate = normalizeToDateInput(item.originalOrderDate);
      const baseShopName = item.originalShopName;

      // DBに保存する override 値（元値に戻した場合は NULL にしてクリア）
      const desiredItemName =
        form.itemName === baseItemName ? null : form.itemName;
      const desiredPrice = nextPrice === basePrice ? null : nextPrice;
      const desiredQuantity =
        nextQuantity === baseQuantity ? null : nextQuantity;
      const desiredBrand = form.brand === baseBrand ? null : form.brand;
      // UIでは category を編集しないため、既存の override 値があればそれを維持する
      // override が無い場合は NULL のまま（新規に category override を作らない）
      const desiredCategory = item.itemOverrideCategory ?? null;

      const shouldDeleteItemOverride =
        desiredItemName == null &&
        desiredPrice == null &&
        desiredQuantity == null &&
        desiredBrand == null &&
        desiredCategory == null;

      const desiredNewOrderNumber =
        form.orderNumber === baseOrderNumber ? null : form.orderNumber;
      const desiredOrderDate =
        form.orderDate === baseOrderDate ? null : form.orderDate;
      const desiredShopName =
        form.shopName === (baseShopName ?? '') ? null : form.shopName;

      const shouldDeleteOrderOverride =
        desiredNewOrderNumber == null &&
        desiredOrderDate == null &&
        desiredShopName == null;

      // アイテムレベルの変更を検出
      const itemChanged =
        form.itemName !== (item.itemName ?? '') ||
        nextPrice !== (item.price ?? 0) ||
        nextQuantity !== (item.quantity ?? 1) ||
        form.brand !== (item.brand ?? '');

      if (itemChanged) {
        if (shouldDeleteItemOverride) {
          await invoke('delete_item_override_by_key', {
            shopDomain: item.shopDomain,
            orderNumber: item.originalOrderNumber,
            originalItemName: item.originalItemName,
            originalBrand: item.originalBrand,
          });
        } else {
          await invoke('save_item_override', {
            shopDomain: item.shopDomain,
            orderNumber: item.originalOrderNumber,
            // JOIN に使用する元キーを渡す（表示値で送ると再編集時に一致しない）
            originalItemName: item.originalItemName,
            originalBrand: item.originalBrand,
            itemName: desiredItemName,
            price: desiredPrice,
            quantity: desiredQuantity,
            brand: desiredBrand,
            category: desiredCategory,
          });
        }
      }

      // 注文レベルの変更を検出
      const orderChanged =
        form.orderNumber !== (item.orderNumber ?? '') ||
        form.orderDate !== normalizeToDateInput(item.orderDate) ||
        form.shopName !== (item.shopName ?? '');

      if (orderChanged) {
        if (shouldDeleteOrderOverride) {
          await invoke('delete_order_override_by_key', {
            shopDomain: item.shopDomain,
            orderNumber: item.originalOrderNumber,
          });
        } else {
          await invoke('save_order_override', {
            shopDomain: item.shopDomain,
            // キーは補正前の order_number を使う（補正後表示値だと一致しない）
            orderNumber: item.originalOrderNumber,
            newOrderNumber: desiredNewOrderNumber,
            orderDate: desiredOrderDate,
            shopName: desiredShopName,
          });
        }
      }

      setIsEditing(false);
      onDataChanged?.();
    } catch (e) {
      console.error('Failed to save override:', e);
      setValidationError(e instanceof Error ? e.message : '保存に失敗しました');
    } finally {
      setIsSaving(false);
    }
  }, [item, form, onDataChanged]);

  const handleExclude = useCallback(async () => {
    if (!item || !item.shopDomain || !item.originalOrderNumber) return;
    try {
      setValidationError(null);
      await invoke('exclude_item', {
        shopDomain: item.shopDomain,
        // 除外も元キーで一致させる（表示値だと JOIN に一致しない）
        orderNumber: item.originalOrderNumber,
        itemName: item.originalItemName,
        brand: item.originalBrand,
        reason: null,
      });
      setExcludeDialogOpen(false);
      onOpenChange(false);
      onDataChanged?.();
    } catch (e) {
      console.error('Failed to exclude item:', e);
      setValidationError(
        e instanceof Error ? e.message : '商品を除外できませんでした'
      );
    }
  }, [item, onOpenChange, onDataChanged]);

  if (!item) return null;

  return (
    <>
      <Sheet open={open} onOpenChange={onOpenChange}>
        <SheetContent side="right" className="sm:max-w-md overflow-y-auto">
          <SheetHeader>
            <div className="flex items-center gap-2 pr-8">
              <SheetTitle className="text-left flex-1 min-w-0 truncate">
                {item.itemName}
              </SheetTitle>
              {item.hasOverride === 1 && (
                <span className="shrink-0 text-xs bg-blue-100 text-blue-800 px-1.5 py-0.5 rounded">
                  修正済
                </span>
              )}
              {!isEditing && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="shrink-0 h-8 w-8"
                  onClick={() => setIsEditing(true)}
                  title="編集"
                >
                  <Pencil className="h-4 w-4" />
                </Button>
              )}
              {isEditing && (
                <Button
                  variant="ghost"
                  size="icon"
                  className="shrink-0 h-8 w-8"
                  onClick={() => setIsEditing(false)}
                  title="キャンセル"
                >
                  <X className="h-4 w-4" />
                </Button>
              )}
            </div>
          </SheetHeader>
          <div className="mt-6 space-y-6">
            {/* 画像 */}
            <div className="space-y-2">
              <div
                key={imageKey}
                className="aspect-square max-h-64 bg-muted/50 rounded-lg overflow-hidden flex items-center justify-center cursor-pointer hover:opacity-80 transition-opacity"
                onClick={() => openImageSearchDialog()}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    openImageSearchDialog();
                  }
                }}
                aria-label="画像を検索"
                title="クリックして画像を検索"
              >
                {imageSrc ? (
                  <img
                    src={imageSrc}
                    alt={item.itemName}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <span className="text-muted-foreground text-sm">
                    画像なし
                  </span>
                )}
              </div>
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                onClick={() => openImageSearchDialog()}
              >
                <Search className="mr-2 h-4 w-4" />
                画像を検索
              </Button>
            </div>

            {isEditing ? (
              /* ─── 編集モード ─── */
              <>
                <div className="space-y-1.5">
                  <Label htmlFor="edit-itemName">商品名</Label>
                  <Input
                    id="edit-itemName"
                    value={form.itemName}
                    onChange={(e) =>
                      setForm((f) => ({ ...f, itemName: e.target.value }))
                    }
                  />
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-1.5">
                    <Label htmlFor="edit-price">価格</Label>
                    <Input
                      id="edit-price"
                      type="number"
                      value={form.price}
                      onChange={(e) =>
                        setForm((f) => ({ ...f, price: e.target.value }))
                      }
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="edit-quantity">数量</Label>
                    <Input
                      id="edit-quantity"
                      type="number"
                      min={1}
                      value={form.quantity}
                      onChange={(e) =>
                        setForm((f) => ({ ...f, quantity: e.target.value }))
                      }
                    />
                  </div>
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="edit-brand">メーカー</Label>
                  <Input
                    id="edit-brand"
                    value={form.brand}
                    onChange={(e) =>
                      setForm((f) => ({ ...f, brand: e.target.value }))
                    }
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="edit-shopName">ショップ名</Label>
                  <Input
                    id="edit-shopName"
                    value={form.shopName}
                    onChange={(e) =>
                      setForm((f) => ({ ...f, shopName: e.target.value }))
                    }
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="edit-orderNumber">注文番号</Label>
                  <Input
                    id="edit-orderNumber"
                    value={form.orderNumber}
                    onChange={(e) =>
                      setForm((f) => ({ ...f, orderNumber: e.target.value }))
                    }
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="edit-orderDate">注文日</Label>
                  <Input
                    id="edit-orderDate"
                    type="date"
                    value={form.orderDate}
                    onChange={(e) =>
                      setForm((f) => ({ ...f, orderDate: e.target.value }))
                    }
                  />
                </div>

                <div className="flex gap-2 pt-2">
                  <Button
                    className="flex-1"
                    onClick={handleSave}
                    disabled={isSaving}
                  >
                    {isSaving ? '保存中...' : '保存'}
                  </Button>
                  <Button
                    variant="outline"
                    className="flex-1"
                    onClick={() => setIsEditing(false)}
                    disabled={isSaving}
                  >
                    キャンセル
                  </Button>
                </div>

                {validationError && (
                  <p className="text-sm text-destructive">{validationError}</p>
                )}

                <Button
                  variant="destructive"
                  size="sm"
                  className="w-full"
                  onClick={() => setExcludeDialogOpen(true)}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  この商品を除外
                </Button>
              </>
            ) : (
              /* ─── 表示モード ─── */
              <>
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-1">
                    ステータス
                  </h4>
                  <StatusBadge status={item.deliveryStatus} />
                </div>
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-1">
                    価格
                  </h4>
                  <p className="text-lg font-semibold">
                    {formatPrice(item.price)}
                  </p>
                </div>
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-1">
                    数量
                  </h4>
                  <p>{item.quantity}</p>
                </div>
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-1">
                    ショップ
                  </h4>
                  <p>{item.shopName ?? item.shopDomain ?? '-'}</p>
                </div>
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-1">
                    注文番号
                  </h4>
                  <p>{item.orderNumber ?? '-'}</p>
                </div>
                <div>
                  <h4 className="text-sm font-medium text-muted-foreground mb-1">
                    注文日
                  </h4>
                  <p>{formatDate(item.orderDate)}</p>
                </div>
                {(item.brand || item.category) && (
                  <div>
                    <h4 className="text-sm font-medium text-muted-foreground mb-1">
                      メーカー / 作品名
                    </h4>
                    <p>
                      {[item.brand, item.category].filter(Boolean).join(' / ')}
                    </p>
                  </div>
                )}
              </>
            )}
          </div>
        </SheetContent>
      </Sheet>

      <ImageSearchDialog
        open={imageSearchOpen}
        onOpenChange={(next) => {
          setImageSearchOpen(next);
          if (!next) setImageSearchInitialUrl(undefined);
        }}
        itemId={item.id}
        itemName={item.itemName}
        initialUrl={imageSearchInitialUrl}
        onImageSaved={handleImageSaved}
      />

      {/* 除外確認ダイアログ */}
      <Dialog open={excludeDialogOpen} onOpenChange={setExcludeDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>商品を除外</DialogTitle>
            <DialogDescription>
              この商品を除外しますか？再パース後も表示されなくなります。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setExcludeDialogOpen(false)}
            >
              キャンセル
            </Button>
            <Button variant="destructive" onClick={handleExclude}>
              除外する
            </Button>
          </DialogFooter>

          {validationError && (
            <p className="text-sm text-destructive">{validationError}</p>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
