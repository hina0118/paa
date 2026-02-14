import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Loader2, Search, Check, ExternalLink } from 'lucide-react';
import {
  toastSuccess,
  toastError,
  toastWarning,
  formatError,
} from '@/lib/toast';

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
  initialUrl?: string;
  onImageSaved?: () => void;
};

export function ImageSearchDialog({
  open,
  onOpenChange,
  itemId,
  itemName,
  initialUrl,
  onImageSaved,
}: ImageSearchDialogProps) {
  const [isSearching, setIsSearching] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [searchResults, setSearchResults] = useState<ImageSearchResult[]>([]);
  const [selectedUrl, setSelectedUrl] = useState<string | null>(null);
  const [savedSuccess, setSavedSuccess] = useState(false);
  const [apiSearchFailed, setApiSearchFailed] = useState(false);
  const [manualUrlInput, setManualUrlInput] = useState('');

  // 初期URLが指定されている場合、入力欄に自動反映
  useEffect(() => {
    if (!open) return;
    const url = initialUrl?.trim();
    if (!url) return;
    setSelectedUrl(null);
    setSavedSuccess(false);
    setApiSearchFailed(false);
    setSearchResults([]);
    setManualUrlInput(url);
  }, [open, initialUrl]);

  const handleSearch = useCallback(async () => {
    setIsSearching(true);
    setSearchResults([]);
    setSelectedUrl(null);
    setSavedSuccess(false);
    setApiSearchFailed(false);
    setManualUrlInput('');

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
        toastWarning('画像が見つかりませんでした。');
        setApiSearchFailed(true);
      }
    } catch (e) {
      toastError(`画像検索に失敗しました: ${formatError(e)}`);
      setApiSearchFailed(true);
    } finally {
      setIsSearching(false);
    }
  }, [itemName]);

  const handleOpenBrowserSearch = useCallback(() => {
    const searchUrl = `https://www.google.com/search?q=${encodeURIComponent(itemName)}&tbm=isch`;
    const uniqueSuffix =
      typeof crypto !== 'undefined' && 'randomUUID' in crypto
        ? crypto.randomUUID()
        : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const label = `image-search-${uniqueSuffix}`;
    try {
      const webview = new WebviewWindow(label, {
        url: searchUrl,
        title: `Google画像検索: ${itemName}`,
        width: 900,
        height: 700,
      });
      webview.once('tauri://error', (e) => {
        toastError(`サブウィンドウを開けませんでした: ${formatError(e)}`);
      });
    } catch (err) {
      toastError(`サブウィンドウを開けませんでした: ${formatError(err)}`);
    }
  }, [itemName]);

  const urlToSave =
    selectedUrl || (manualUrlInput.trim() ? manualUrlInput.trim() : null);

  // URL validation - parse URL and check protocol is https:
  const urlValidation = (() => {
    if (!urlToSave) return { isValid: false, isHttp: false, parsed: null };
    try {
      const parsed = new URL(urlToSave);
      const isValid = parsed.protocol === 'https:';
      const isHttp = parsed.protocol === 'http:';
      return { isValid, isHttp, parsed };
    } catch {
      return { isValid: false, isHttp: false, parsed: null };
    }
  })();
  const isInvalidOrNonHttpsUrl = Boolean(urlToSave) && !urlValidation.isValid;

  const handleSaveImage = useCallback(async () => {
    if (!urlToSave) return;

    setIsSaving(true);

    try {
      await invoke('save_image_from_url', {
        itemId,
        imageUrl: urlToSave,
      });
      toastSuccess('画像を保存しました');
      setSavedSuccess(true);
      onImageSaved?.();
    } catch (e) {
      toastError(`画像の保存に失敗しました: ${formatError(e)}`);
    } finally {
      setIsSaving(false);
    }
  }, [itemId, urlToSave, onImageSaved]);

  const handleOpenChange = useCallback(
    (newOpen: boolean) => {
      if (!newOpen) {
        // ダイアログを閉じるときにステートをリセット
        setSearchResults([]);
        setSelectedUrl(null);
        setSavedSuccess(false);
        setApiSearchFailed(false);
        setManualUrlInput('');
      }
      onOpenChange(newOpen);
    },
    [onOpenChange]
  );

  // 成功後、少し待ってからダイアログを閉じる（クリーンアップでメモリリーク防止）
  useEffect(() => {
    if (savedSuccess) {
      const timer = setTimeout(() => {
        // close時のステートリセットを確実に通す
        handleOpenChange(false);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [savedSuccess, handleOpenChange]);

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
          {urlToSave && (
            <div className="p-3 bg-muted/50 rounded-lg">
              <p className="text-sm text-muted-foreground mb-2">
                選択中の画像:
              </p>
              <p className="text-xs text-muted-foreground truncate mb-2">
                {urlToSave}
              </p>
              {isInvalidOrNonHttpsUrl ? (
                <div className="p-4 bg-destructive/10 border border-destructive/20 rounded-md">
                  <p className="text-sm text-destructive font-medium">
                    {urlValidation.isHttp
                      ? 'HTTPのURLは使用できません'
                      : 'このURLは使用できません'}
                  </p>
                  <p className="text-xs text-destructive/80 mt-1">
                    セキュリティ上の理由により、HTTPSのURLのみ対応しています。
                  </p>
                </div>
              ) : (
                <div className="rounded-md overflow-hidden border bg-background">
                  <img
                    src={urlToSave}
                    alt="selected preview"
                    className="w-full max-h-48 object-cover"
                    loading="lazy"
                    onError={(e) => {
                      const img = e.target as HTMLImageElement;
                      img.style.display = 'none';
                    }}
                  />
                </div>
              )}
            </div>
          )}

          {/* ブラウザ検索フォールバック（API失敗時または手動登録用） */}
          <div className="space-y-3 pt-3 border-t">
            {apiSearchFailed && (
              <p className="text-sm text-muted-foreground">
                結果が見つからない場合もサブウィンドウでGoogle画像検索を開き、画像を選択してURLを貼り付けて登録できます。
              </p>
            )}
            <div className="flex flex-col sm:flex-row gap-2">
              <Button
                variant="outline"
                onClick={handleOpenBrowserSearch}
                className="flex-shrink-0"
              >
                <ExternalLink className="mr-2 h-4 w-4" />
                サブウィンドウでGoogle画像検索を開く
              </Button>
              <div className="flex-1 min-w-0">
                <Input
                  type="url"
                  placeholder="画像のURLをここに貼り付け"
                  value={manualUrlInput}
                  onChange={(e) => setManualUrlInput(e.target.value)}
                  className="h-9"
                />
              </div>
            </div>
          </div>
        </div>

        {/* フッター */}
        <div className="flex justify-end gap-2 pt-4 border-t">
          <Button variant="outline" onClick={() => handleOpenChange(false)}>
            キャンセル
          </Button>
          <Button
            onClick={handleSaveImage}
            disabled={!urlValidation.isValid || isSaving || savedSuccess}
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
