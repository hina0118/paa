import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
import { formatDate, formatPrice } from '@/lib/utils';
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

export function OrderItemDrawer({
  item,
  open,
  onOpenChange,
  onImageUpdated,
  onDataChanged,
}: OrderItemDrawerProps) {
  const getImageUrl = useImageUrl();
  const [imageSearchOpen, setImageSearchOpen] = useState(false);
  const [imageKey, setImageKey] = useState(0);
  const [isEditing, setIsEditing] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [excludeDialogOpen, setExcludeDialogOpen] = useState(false);
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

  // 編集モード開始時にフォームを初期化
  useEffect(() => {
    if (isEditing && item) {
      setForm({
        itemName: item.itemName ?? '',
        price: String(item.price ?? 0),
        quantity: String(item.quantity ?? 1),
        brand: item.brand ?? '',
        orderNumber: item.orderNumber ?? '',
        orderDate: item.orderDate ?? '',
        shopName: item.shopName ?? '',
      });
    }
  }, [isEditing, item]);

  // ドロワーが閉じたら編集モードを解除
  useEffect(() => {
    if (!open) {
      setIsEditing(false);
    }
  }, [open]);

  const handleImageSaved = useCallback(() => {
    onImageUpdated?.();
    setImageKey((prev) => prev + 1);
  }, [onImageUpdated]);

  const handleSave = useCallback(async () => {
    if (!item || !item.shopDomain || !item.orderNumber) return;
    setIsSaving(true);
    try {
      // アイテムレベルの変更を検出
      const itemChanged =
        form.itemName !== (item.itemName ?? '') ||
        form.price !== String(item.price ?? 0) ||
        form.quantity !== String(item.quantity ?? 1) ||
        form.brand !== (item.brand ?? '');

      if (itemChanged) {
        await invoke('save_item_override', {
          shopDomain: item.shopDomain,
          orderNumber: item.orderNumber,
          originalItemName: item.itemName,
          originalBrand: item.brand ?? '',
          itemName:
            form.itemName !== (item.itemName ?? '') ? form.itemName : null,
          price:
            form.price !== String(item.price ?? 0) ? Number(form.price) : null,
          quantity:
            form.quantity !== String(item.quantity ?? 1)
              ? Number(form.quantity)
              : null,
          brand: form.brand !== (item.brand ?? '') ? form.brand : null,
          category: null,
        });
      }

      // 注文レベルの変更を検出
      const orderChanged =
        form.orderNumber !== (item.orderNumber ?? '') ||
        form.orderDate !== (item.orderDate ?? '') ||
        form.shopName !== (item.shopName ?? '');

      if (orderChanged) {
        await invoke('save_order_override', {
          shopDomain: item.shopDomain,
          orderNumber: item.orderNumber,
          newOrderNumber:
            form.orderNumber !== (item.orderNumber ?? '')
              ? form.orderNumber
              : null,
          orderDate:
            form.orderDate !== (item.orderDate ?? '') ? form.orderDate : null,
          shopName:
            form.shopName !== (item.shopName ?? '') ? form.shopName : null,
        });
      }

      setIsEditing(false);
      onDataChanged?.();
    } catch (e) {
      console.error('Failed to save override:', e);
    } finally {
      setIsSaving(false);
    }
  }, [item, form, onDataChanged]);

  const handleExclude = useCallback(async () => {
    if (!item || !item.shopDomain || !item.orderNumber) return;
    try {
      await invoke('exclude_item', {
        shopDomain: item.shopDomain,
        orderNumber: item.orderNumber,
        itemName: item.itemName,
        brand: item.brand ?? '',
        reason: null,
      });
      setExcludeDialogOpen(false);
      onOpenChange(false);
      onDataChanged?.();
    } catch (e) {
      console.error('Failed to exclude item:', e);
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
                onClick={() => setImageSearchOpen(true)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    setImageSearchOpen(true);
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
                onClick={() => setImageSearchOpen(true)}
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
        onOpenChange={setImageSearchOpen}
        itemId={item.id}
        itemName={item.itemName}
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
        </DialogContent>
      </Dialog>
    </>
  );
}
