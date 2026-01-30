import { ShoppingCart } from 'lucide-react';

export function Orders() {
  return (
    <div className="container mx-auto py-10 px-6">
      <div className="mb-8 space-y-2">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-primary/10">
            <ShoppingCart className="h-6 w-6 text-primary" />
          </div>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">商品一覧</h1>
            <p className="text-sm text-muted-foreground mt-1">
              注文商品を閲覧・管理
            </p>
          </div>
        </div>
      </div>
      <div className="text-muted-foreground py-12 text-center">
        読み込み中...
      </div>
    </div>
  );
}
