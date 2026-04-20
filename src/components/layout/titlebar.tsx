import { useState, useEffect, useRef } from 'react';
import { Minus, Square, X, Minimize2 } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { cn } from '@/lib/utils';
import appIcon from '@/assets/app-icon.png';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  const isActiveRef = useRef(true);

  useEffect(() => {
    isActiveRef.current = true;
    let cleanup: (() => void) | undefined;

    const init = async () => {
      const win = getCurrentWindow();
      const maximized = await win.isMaximized();
      setIsMaximized(maximized);

      const unlisten = await win.onResized(async () => {
        const maximized = await win.isMaximized().catch(() => false);
        setIsMaximized(maximized);
      });

      if (!isActiveRef.current) {
        unlisten();
      } else {
        cleanup = unlisten;
      }
    };

    init().catch(() => {});

    return () => {
      isActiveRef.current = false;
      cleanup?.();
    };
  }, []);

  const handleMinimize = async () => {
    try {
      await getCurrentWindow().minimize();
    } catch {
      /* noop */
    }
  };
  const handleMaximize = async () => {
    try {
      await getCurrentWindow().toggleMaximize();
    } catch {
      /* noop */
    }
  };
  const handleClose = async () => {
    try {
      await getCurrentWindow().close();
    } catch {
      /* noop */
    }
  };

  return (
    <div
      className="flex h-9 shrink-0 items-center border-b bg-background select-none"
      data-tauri-drag-region
    >
      {/* アプリアイコン */}
      <div className="flex items-center pl-2" data-tauri-drag-region>
        <img src={appIcon} alt="paa" className="h-5 w-5" draggable={false} />
      </div>

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
