import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Loader2, Search, AlertCircle, Check } from 'lucide-react';

/** 画像検索結果の型 */
type ImageSearchResult = {
  url: string;
  thumbnail_url: string | null;
  width: number | null;
  height: number | null;
  title: string | null;
  mime_type: string | null;
};

type ImageSearchDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  itemId: number;
  itemName: string;
  onImageSaved?: () => void;
};

export function ImageSearchDialog({
  open,
  onOpenChange,
  itemId,
  itemName,
  onImageSaved,
}: ImageSearchDialogProps) {
  const [isSearching, setIsSearching] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [searchResults, setSearchResults] = useState<ImageSearchResult[]>([]);
  const [selectedUrl, setSelectedUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [savedSuccess, setSavedSuccess] = useState(false);

  const handleSearch = useCallback(async () => {
    setIsSearching(true);
    setError(null);
    setSearchResults([]);
    setSelectedUrl(null);
    setSavedSuccess(false);

    try {
      const results = await invoke<ImageSearchResult[]>(
        'search_product_images',
        {
          query: itemName,
          numResults: 10,
        }
      );
      const safeResults = Array.isArray(results) ? results : [];
      setSearchResults(safeResults);
      if (safeResults.length === 0) {
        setError('画像が見つかりませんでした。');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsSearching(false);
    }
  }, [itemName]);

  const handleSaveImage = useCallback(async () => {
    if (!selectedUrl) return;

    setIsSaving(true);
    setError(null);

    try {
      await invoke('save_image_from_url', {
        itemId,
        imageUrl: selectedUrl,
      });
      setSavedSuccess(true);
      onImageSaved?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsSaving(false);
    }
  }, [itemId, selectedUrl, onImageSaved]);

  // 成功後、少し待ってからダイアログを閉じる（クリーンアップでメモリリーク防止）
  useEffect(() => {
    if (savedSuccess) {
      const timer = setTimeout(() => {
        onOpenChange(false);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [savedSuccess, onOpenChange]);

  const handleOpenChange = useCallback(
    (newOpen: boolean) => {
      if (!newOpen) {
        // ダイアログを閉じるときにステートをリセット
        setSearchResults([]);
        setSelectedUrl(null);
        setError(null);
        setSavedSuccess(false);
      }
      onOpenChange(newOpen);
    },
    [onOpenChange]
  );

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[80vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle>画像を検索</DialogTitle>
          <DialogDescription className="truncate">
            「{itemName}」の画像を検索します
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto space-y-4">
          {/* 検索ボタン */}
          <div className="flex gap-2">
            <Button
              onClick={handleSearch}
              disabled={isSearching}
              className="flex-1"
            >
              {isSearching ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  検索中...
                </>
              ) : (
                <>
                  <Search className="mr-2 h-4 w-4" />
                  画像を検索
                </>
              )}
            </Button>
          </div>

          {/* エラー表示 */}
          {error && (
            <div className="flex items-center gap-2 p-3 bg-red-50 border border-red-200 rounded text-sm text-red-800">
              <AlertCircle className="h-4 w-4 flex-shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {/* 成功表示 */}
          {savedSuccess && (
            <div className="flex items-center gap-2 p-3 bg-green-50 border border-green-200 rounded text-sm text-green-800">
              <Check className="h-4 w-4 flex-shrink-0" />
              <span>画像を保存しました</span>
            </div>
          )}

          {/* 検索結果グリッド */}
          {searchResults.length > 0 && (
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
              {searchResults.map((result, index) => (
                <button
                  key={`${result.url}-${index}`}
                  type="button"
                  onClick={() => setSelectedUrl(result.url)}
                  className={`
                    relative aspect-square rounded-lg overflow-hidden border-2 transition-all
                    hover:opacity-80 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2
                    ${
                      selectedUrl === result.url
                        ? 'border-primary ring-2 ring-primary'
                        : 'border-muted'
                    }
                  `}
                >
                  <img
                    src={result.thumbnail_url || result.url}
                    alt={result.title || `検索結果 ${index + 1}`}
                    className="w-full h-full object-cover"
                    loading="lazy"
                    onError={(e) => {
                      const img = e.target as HTMLImageElement;
                      // サムネイルが読み込めない場合は一度だけ元のURLを試す
                      if (img.src !== result.url && !img.dataset.errorHandled) {
                        img.dataset.errorHandled = 'true';
                        img.src = result.url;
                      } else {
                        img.style.display = 'none';
                      }
                    }}
                  />
                  {selectedUrl === result.url && (
                    <div className="absolute inset-0 bg-primary/20 flex items-center justify-center">
                      <Check className="h-8 w-8 text-primary" />
                    </div>
                  )}
                </button>
              ))}
            </div>
          )}

          {/* 選択された画像のプレビュー */}
          {selectedUrl && (
            <div className="p-3 bg-muted/50 rounded-lg">
              <p className="text-sm text-muted-foreground mb-2">
                選択中の画像:
              </p>
              <p className="text-xs text-muted-foreground truncate">
                {selectedUrl}
              </p>
            </div>
          )}
        </div>

        {/* フッター */}
        <div className="flex justify-end gap-2 pt-4 border-t">
          <Button variant="outline" onClick={() => handleOpenChange(false)}>
            キャンセル
          </Button>
          <Button
            onClick={handleSaveImage}
            disabled={!selectedUrl || isSaving || savedSuccess}
          >
            {isSaving ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                保存中...
              </>
            ) : savedSuccess ? (
              <>
                <Check className="mr-2 h-4 w-4" />
                保存完了
              </>
            ) : (
              '選択した画像を保存'
            )}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
