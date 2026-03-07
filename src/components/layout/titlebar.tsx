import { useState, useEffect, useRef } from 'react';
import { Minus, Square, X, Minimize2 } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { cn } from '@/lib/utils';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  const isActiveRef = useRef(true);

  useEffect(() => {
    isActiveRef.current = true;
    const win = getCurrentWindow();

    win
      .isMaximized()
      .then(setIsMaximized)
      .catch(() => {});

    let cleanup: (() => void) | undefined;
    win
      .onResized(async () => {
        const maximized = await win.isMaximized().catch(() => false);
        setIsMaximized(maximized);
      })
      .then((unlisten) => {
        if (!isActiveRef.current) {
          unlisten();
        } else {
          cleanup = unlisten;
        }
      })
      .catch(() => {});

    return () => {
      isActiveRef.current = false;
      cleanup?.();
    };
  }, []);

  const handleMinimize = () =>
    getCurrentWindow()
      .minimize()
      .catch(() => {});
  const handleMaximize = () =>
    getCurrentWindow()
      .toggleMaximize()
      .catch(() => {});
  const handleClose = () =>
    getCurrentWindow()
      .close()
      .catch(() => {});

  return (
    <div
      className="flex h-9 shrink-0 items-center border-b bg-background select-none"
      data-tauri-drag-region
    >
      {/* ドラッグ領域（フレックスで残りスペースを占有） */}
      <div className="flex-1" data-tauri-drag-region />

      {/* ウィンドウコントロール */}
      <div className="flex">
        <button
          type="button"
          className={cn(
            'flex h-9 w-12 items-center justify-center',
            'text-muted-foreground transition-colors hover:bg-muted hover:text-foreground'
          )}
          aria-label="最小化"
          onClick={handleMinimize}
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          className={cn(
            'flex h-9 w-12 items-center justify-center',
            'text-muted-foreground transition-colors hover:bg-muted hover:text-foreground'
          )}
          aria-label={isMaximized ? '元のサイズに戻す' : '最大化'}
          onClick={handleMaximize}
        >
          {isMaximized ? (
            <Minimize2 className="h-3.5 w-3.5" />
          ) : (
            <Square className="h-3.5 w-3.5" />
          )}
        </button>
        <button
          type="button"
          className={cn(
            'flex h-9 w-12 items-center justify-center',
            'text-muted-foreground transition-colors',
            'hover:bg-destructive hover:text-destructive-foreground'
          )}
          aria-label="閉じる"
          onClick={handleClose}
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  );
}
