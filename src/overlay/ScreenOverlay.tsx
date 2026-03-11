import {
  useState,
  useRef,
  useEffect,
  useCallback,
  type MouseEvent,
  type CSSProperties,
} from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface SelectionRect {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

type OverlayState = 'selecting' | 'processing' | 'error';

function normalizeRect(rect: SelectionRect) {
  return {
    x: Math.min(rect.startX, rect.endX),
    y: Math.min(rect.startY, rect.endY),
    width: Math.abs(rect.endX - rect.startX),
    height: Math.abs(rect.endY - rect.startY),
  };
}

export function ScreenOverlay() {
  const [state, setState] = useState<OverlayState>('selecting');
  const [errorMsg, setErrorMsg] = useState('');
  const [selection, setSelection] = useState<SelectionRect | null>(null);
  const isDragging = useRef(false);
  const startPos = useRef({ x: 0, y: 0 });

  const closeOverlay = useCallback(async () => {
    try {
      await getCurrentWindow().close();
    } catch {
      await invoke('close_screen_overlay');
    }
  }, []);

  // Escキーで閉じる
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeOverlay();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [closeOverlay]);

  const handleMouseDown = (e: MouseEvent) => {
    if (state !== 'selecting') return;
    e.preventDefault();
    isDragging.current = true;
    startPos.current = { x: e.clientX, y: e.clientY };
    setSelection({
      startX: e.clientX,
      startY: e.clientY,
      endX: e.clientX,
      endY: e.clientY,
    });
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!isDragging.current || state !== 'selecting') return;
    setSelection((prev) =>
      prev ? { ...prev, endX: e.clientX, endY: e.clientY } : null
    );
  };

  const handleMouseUp = async (e: MouseEvent) => {
    if (!isDragging.current || state !== 'selecting') return;
    isDragging.current = false;

    const rect = {
      startX: startPos.current.x,
      startY: startPos.current.y,
      endX: e.clientX,
      endY: e.clientY,
    };
    const { x, y, width, height } = normalizeRect(rect);

    // 小さすぎる選択は無視
    if (width < 10 || height < 10) {
      setSelection(null);
      return;
    }

    setState('processing');

    try {
      // 物理ピクセル座標に変換するためウィンドウのスクリーン座標を取得
      const win = getCurrentWindow();
      const outerPos = await win.outerPosition();
      const factor = await win.scaleFactor();

      // 論理ピクセル → 物理ピクセル変換
      // outerPosition() は物理ピクセルで返る
      const physX = Math.round(outerPos.x + x * factor);
      const physY = Math.round(outerPos.y + y * factor);
      const physW = Math.round(width * factor);
      const physH = Math.round(height * factor);

      // キャプチャ前にオーバーレイを非表示にしてスクリーンショットへの写り込みを防ぐ
      await win.hide();
      // OSが再描画するまで待機
      await new Promise((resolve) => setTimeout(resolve, 150));

      await invoke('capture_and_ocr', {
        x: physX,
        y: physY,
        width: physW,
        height: physH,
      });

      // OCR完了後にウィンドウを閉じる（メインウィンドウがocr-resultイベントを受信する）
      await closeOverlay();
    } catch (err) {
      setState('error');
      setErrorMsg(String(err));
    }
  };

  const selectionStyle = (): CSSProperties => {
    if (!selection) return { display: 'none' };
    const { x, y, width, height } = normalizeRect(selection);
    return {
      position: 'fixed',
      left: x,
      top: y,
      width,
      height,
      border: '2px solid #3b82f6',
      backgroundColor: 'rgba(59, 130, 246, 0.1)',
      pointerEvents: 'none',
    };
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor:
          state === 'selecting' ? 'rgba(0, 0, 0, 0.35)' : 'rgba(0, 0, 0, 0.6)',
        cursor: state === 'selecting' ? 'crosshair' : 'default',
        userSelect: 'none',
        WebkitUserSelect: 'none',
      }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {state === 'selecting' && (
        <div
          style={{
            position: 'fixed',
            top: 24,
            left: '50%',
            transform: 'translateX(-50%)',
            backgroundColor: 'rgba(0, 0, 0, 0.75)',
            color: '#fff',
            padding: '8px 20px',
            borderRadius: 8,
            fontSize: 14,
            pointerEvents: 'none',
            whiteSpace: 'nowrap',
          }}
        >
          ドラッグして範囲を選択 ／ Esc でキャンセル
        </div>
      )}

      {/* 選択矩形 */}
      <div style={selectionStyle()} />

      {state === 'processing' && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 16,
          }}
        >
          <div
            style={{
              width: 48,
              height: 48,
              border: '4px solid rgba(255,255,255,0.3)',
              borderTop: '4px solid #fff',
              borderRadius: '50%',
              animation: 'spin 0.8s linear infinite',
            }}
          />
          <p style={{ color: '#fff', fontSize: 16 }}>OCR処理中...</p>
          <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
        </div>
      )}

      {state === 'error' && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 16,
            padding: 32,
          }}
        >
          <p style={{ color: '#f87171', fontSize: 16, textAlign: 'center' }}>
            エラーが発生しました
          </p>
          <p
            style={{
              color: 'rgba(255,255,255,0.7)',
              fontSize: 13,
              textAlign: 'center',
              maxWidth: 480,
            }}
          >
            {errorMsg}
          </p>
          <button
            onClick={closeOverlay}
            style={{
              marginTop: 8,
              padding: '8px 24px',
              backgroundColor: '#3b82f6',
              color: '#fff',
              border: 'none',
              borderRadius: 6,
              cursor: 'pointer',
              fontSize: 14,
            }}
          >
            閉じる
          </button>
        </div>
      )}
    </div>
  );
}
