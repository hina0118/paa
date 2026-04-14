import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Ban, Trash2, GripVertical, Minus, Plus } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { toastSuccess, toastError, formatError } from '@/lib/toast';

interface ExclusionPattern {
  id: number;
  keyword: string;
  match_type: string;
  shop_domain: string | null;
  note: string | null;
}

const MATCH_TYPE_LABELS: Record<string, string> = {
  contains: '部分',
  starts_with: '前方',
  exact: '完全',
};

interface Position {
  x: number;
  y: number;
}

export function ExclusionKeywordFloat() {
  const [isOpen, setIsOpen] = useState(false);
  const [isMinimized, setIsMinimized] = useState(false);
  const [position, setPosition] = useState<Position>({
    x: window.innerWidth - 320,
    y: 80,
  });
  const [patterns, setPatterns] = useState<ExclusionPattern[]>([]);
  const [keyword, setKeyword] = useState('');
  const [isAdding, setIsAdding] = useState(false);

  const isDragging = useRef(false);
  const dragOffset = useRef<Position>({ x: 0, y: 0 });
  const panelRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const loadPatterns = useCallback(async () => {
    try {
      const result = await invoke<ExclusionPattern[]>(
        'list_exclusion_patterns'
      );
      setPatterns(result);
    } catch (e) {
      toastError(`除外キーワードの取得に失敗しました: ${formatError(e)}`);
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      loadPatterns();
      if (!isMinimized) {
        setTimeout(() => inputRef.current?.focus(), 50);
      }
    }
  }, [isOpen, isMinimized, loadPatterns]);

  const handleAdd = async () => {
    if (!keyword.trim()) return;
    setIsAdding(true);
    try {
      await invoke('add_exclusion_pattern', {
        shopDomain: null,
        keyword: keyword.trim(),
        matchType: 'contains',
        note: null,
      });
      toastSuccess('除外キーワードを追加しました');
      setKeyword('');
      await loadPatterns();
      inputRef.current?.focus();
    } catch (e) {
      toastError(`追加に失敗しました: ${formatError(e)}`);
    } finally {
      setIsAdding(false);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await invoke('delete_exclusion_pattern', { id });
      toastSuccess('削除しました');
      setPatterns((prev) => prev.filter((p) => p.id !== id));
    } catch (e) {
      toastError(`削除に失敗しました: ${formatError(e)}`);
    }
  };

  const onMouseDown = (e: React.MouseEvent) => {
    if (!panelRef.current) return;
    isDragging.current = true;
    dragOffset.current = {
      x: e.clientX - position.x,
      y: e.clientY - position.y,
    };
    e.preventDefault();
  };

  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      if (!isDragging.current) return;
      const panelW = panelRef.current?.offsetWidth ?? 288;
      const panelH = panelRef.current?.offsetHeight ?? 400;
      setPosition({
        x: Math.max(
          0,
          Math.min(window.innerWidth - panelW, e.clientX - dragOffset.current.x)
        ),
        y: Math.max(
          0,
          Math.min(
            window.innerHeight - panelH,
            e.clientY - dragOffset.current.y
          )
        ),
      });
    };
    const onMouseUp = () => {
      isDragging.current = false;
    };
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
    return () => {
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, []);

  if (!isOpen) {
    return (
      <button
        className="fixed bottom-6 right-6 z-50 h-12 w-12 rounded-full bg-primary text-primary-foreground shadow-lg flex items-center justify-center hover:bg-primary/90 transition-colors"
        onClick={() => setIsOpen(true)}
        title="除外キーワードを管理"
      >
        <Ban className="h-5 w-5" />
      </button>
    );
  }

  return (
    <div
      ref={panelRef}
      className="fixed z-50 w-72 rounded-xl border bg-background shadow-2xl overflow-hidden select-none"
      style={{ left: position.x, top: position.y }}
    >
      {/* タイトルバー（ドラッグハンドル） */}
      <div
        className="flex items-center gap-2 px-3 py-2 bg-muted/60 border-b cursor-grab active:cursor-grabbing"
        onMouseDown={onMouseDown}
      >
        <GripVertical className="h-4 w-4 text-muted-foreground shrink-0" />
        <Ban className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
        <span className="text-xs font-semibold flex-1">除外キーワード</span>
        <button
          className="text-muted-foreground hover:text-foreground transition-colors p-0.5 rounded"
          onClick={() => setIsMinimized((v) => !v)}
          title={isMinimized ? '展開' : '最小化'}
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          className="text-muted-foreground hover:text-foreground transition-colors p-0.5 rounded"
          onClick={() => setIsOpen(false)}
          title="閉じる"
        >
          <Plus className="h-3.5 w-3.5 rotate-45" />
        </button>
      </div>

      {!isMinimized && (
        <>
          {/* キーワード入力 */}
          <div className="p-3 border-b space-y-2">
            <div className="flex gap-2">
              <Input
                ref={inputRef}
                className="h-8 text-sm"
                placeholder="キーワードを入力..."
                value={keyword}
                onChange={(e) => setKeyword(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleAdd();
                }}
              />
              <Button
                size="sm"
                className="h-8 px-3 shrink-0"
                onClick={handleAdd}
                disabled={isAdding || !keyword.trim()}
              >
                追加
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              部分一致・全ショップ対象で登録
            </p>
          </div>

          {/* キーワード一覧 */}
          <div className="max-h-64 overflow-y-auto">
            {patterns.length === 0 ? (
              <p className="text-xs text-muted-foreground text-center py-4">
                登録済みキーワードなし
              </p>
            ) : (
              <ul className="divide-y">
                {patterns.map((p) => (
                  <li
                    key={p.id}
                    className="flex items-center justify-between px-3 py-1.5 gap-2"
                  >
                    <div className="flex items-center gap-1.5 min-w-0">
                      <span className="text-sm truncate">{p.keyword}</span>
                      <span className="text-xs px-1 py-0.5 rounded bg-muted text-muted-foreground shrink-0">
                        {MATCH_TYPE_LABELS[p.match_type] ?? p.match_type}
                      </span>
                    </div>
                    <button
                      className="text-muted-foreground hover:text-destructive transition-colors shrink-0"
                      onClick={() => handleDelete(p.id)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      )}
    </div>
  );
}
