import { useState, useMemo } from 'react';
import { ChevronLeft, ChevronRight, RefreshCw } from 'lucide-react';
import type { NewsClip } from '@/lib/news/clips';
import { cn } from '@/lib/utils';

interface NewsClipCalendarProps {
  clips: NewsClip[];
  selectedDate: string | null; // YYYY-MM-DD
  onSelectDate: (date: string | null) => void;
  onRefreshEvents: (clipId: number) => Promise<void>;
}

// -------------------------------------------------------
// 日付キー取得 (publishedAt 優先)
// -------------------------------------------------------
export function getClipDateKey(clip: NewsClip): string | null {
  const dateStr = clip.publishedAt || clip.clippedAt;
  const date = new Date(dateStr);
  if (isNaN(date.getTime())) return null;
  return toDateKey(date.getFullYear(), date.getMonth() + 1, date.getDate());
}

function toDateKey(y: number, m: number, d: number): string {
  return `${y}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
}

const WEEKDAY_LABELS = ['日', '月', '火', '水', '木', '金', '土'];

export function NewsClipCalendar({
  clips,
  selectedDate,
  onSelectDate,
  onRefreshEvents,
}: NewsClipCalendarProps) {
  const [refreshingId, setRefreshingId] = useState<number | null>(null);

  const initialDate = useMemo(() => {
    const dates = clips
      .map((c) => getClipDateKey(c))
      .filter((d): d is string => d !== null)
      .sort();
    const latest = dates[dates.length - 1];
    if (latest) {
      const [y, m] = latest.split('-').map(Number);
      return { year: y, month: m - 1 };
    }
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth() };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const [viewYear, setViewYear] = useState(initialDate.year);
  const [viewMonth, setViewMonth] = useState(initialDate.month);

  // クリップ公開日ごとの件数
  const dateCounts = useMemo(() => {
    const map = new Map<string, number>();
    for (const clip of clips) {
      const d = getClipDateKey(clip);
      if (d) map.set(d, (map.get(d) ?? 0) + 1);
    }
    return map;
  }, [clips]);

  // clip.events から日付ごとのイベントを集約
  const eventsByDate = useMemo(() => {
    const map = new Map<string, { label: string; clipId: number }[]>();
    for (const clip of clips) {
      for (const ev of clip.events) {
        if (!map.has(ev.date)) map.set(ev.date, []);
        map.get(ev.date)!.push({ label: ev.label, clipId: clip.id });
      }
    }
    return map;
  }, [clips]);

  // events が空のクリップ（summary があるもの）は backfill 候補
  const backfillCandidates = useMemo(
    () => clips.filter((c) => c.events.length === 0 && !!c.summary),
    [clips]
  );

  const prevMonth = () => {
    if (viewMonth === 0) {
      setViewMonth(11);
      setViewYear((y) => y - 1);
    } else setViewMonth((m) => m - 1);
  };
  const nextMonth = () => {
    if (viewMonth === 11) {
      setViewMonth(0);
      setViewYear((y) => y + 1);
    } else setViewMonth((m) => m + 1);
  };

  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const today = new Date();
  const todayKey = toDateKey(
    today.getFullYear(),
    today.getMonth() + 1,
    today.getDate()
  );
  const monthPrefix = `${viewYear}-${String(viewMonth + 1).padStart(2, '0')}`;

  const monthClipTotal = useMemo(
    () =>
      [...dateCounts.entries()]
        .filter(([k]) => k.startsWith(monthPrefix))
        .reduce((sum, [, c]) => sum + c, 0),
    [dateCounts, monthPrefix]
  );

  const monthEventTotal = useMemo(
    () =>
      [...eventsByDate.entries()]
        .filter(([k]) => k.startsWith(monthPrefix))
        .reduce((sum, [, evs]) => sum + evs.length, 0),
    [eventsByDate, monthPrefix]
  );

  const handleDayClick = (day: number) => {
    const dateKey = toDateKey(viewYear, viewMonth + 1, day);
    onSelectDate(selectedDate === dateKey ? null : dateKey);
  };

  const handleRefreshAll = async () => {
    for (const clip of backfillCandidates) {
      setRefreshingId(clip.id);
      try {
        await onRefreshEvents(clip.id);
      } catch {
        // エラーは個別にスキップ
      }
    }
    setRefreshingId(null);
  };

  return (
    <div className="flex flex-col select-none">
      {/* 月ナビゲーション */}
      <div className="flex items-center justify-between px-1 py-1.5 sticky top-0 bg-background z-10 border-b border-border/40">
        <button
          onClick={prevMonth}
          className="p-1 rounded hover:bg-muted transition-colors"
          aria-label="前月"
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </button>
        <div className="text-center">
          <p className="text-xs font-semibold leading-none">
            {viewYear}年{viewMonth + 1}月
          </p>
          <p className="text-[10px] text-muted-foreground mt-0.5">
            {monthClipTotal > 0 && `記事${monthClipTotal}件`}
            {monthClipTotal > 0 && monthEventTotal > 0 && ' / '}
            {monthEventTotal > 0 && `予定${monthEventTotal}件`}
          </p>
        </div>
        <button
          onClick={nextMonth}
          className="p-1 rounded hover:bg-muted transition-colors"
          aria-label="翌月"
        >
          <ChevronRight className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* 日付リスト */}
      <div className="flex flex-col py-0.5">
        {Array.from({ length: daysInMonth }, (_, i) => i + 1).map((day) => {
          const dateKey = toDateKey(viewYear, viewMonth + 1, day);
          const count = dateCounts.get(dateKey) ?? 0;
          const events = eventsByDate.get(dateKey) ?? [];
          const hasClips = count > 0;
          const hasAny = hasClips || events.length > 0;
          const isSelected = selectedDate === dateKey;
          const isToday = dateKey === todayKey;
          const dayOfWeek = new Date(viewYear, viewMonth, day).getDay();

          return (
            <div key={day} className={cn(!hasAny && 'opacity-25')}>
              {/* 日付行 */}
              <button
                onClick={() => hasClips && handleDayClick(day)}
                disabled={!hasClips}
                className={cn(
                  'w-full flex items-center gap-1.5 px-2 py-1 rounded transition-colors text-xs',
                  hasClips ? 'cursor-pointer hover:bg-muted' : 'cursor-default',
                  isSelected &&
                    'bg-primary text-primary-foreground hover:bg-primary/90'
                )}
              >
                <span
                  className={cn(
                    'w-5 text-right font-mono leading-none shrink-0',
                    isToday && !isSelected && 'font-bold'
                  )}
                >
                  {day}
                </span>
                <span
                  className={cn(
                    'text-[10px] leading-none shrink-0',
                    isSelected
                      ? 'opacity-70'
                      : dayOfWeek === 0
                        ? 'text-red-500'
                        : dayOfWeek === 6
                          ? 'text-blue-500'
                          : 'text-muted-foreground'
                  )}
                >
                  {WEEKDAY_LABELS[dayOfWeek]}
                </span>
                {isToday && !isSelected && (
                  <span className="text-[9px] text-primary leading-none">
                    ●
                  </span>
                )}
                {hasClips && (
                  <span
                    className={cn(
                      'ml-auto text-[10px] leading-none px-1 py-0.5 rounded shrink-0',
                      isSelected
                        ? 'bg-primary-foreground/20 text-primary-foreground'
                        : 'bg-primary/10 text-primary'
                    )}
                  >
                    {count}
                  </span>
                )}
              </button>

              {/* AI 抽出イベントラベル */}
              {events.map((ev, idx) => (
                <div
                  key={`${ev.clipId}-${idx}`}
                  className="flex items-start gap-1 pl-8 pr-2 py-0.5"
                  title={ev.label}
                >
                  <span className="mt-[3px] w-1.5 h-1.5 rounded-full bg-amber-500 shrink-0" />
                  <span className="text-[10px] text-amber-600 dark:text-amber-400 leading-snug line-clamp-2">
                    {ev.label}
                  </span>
                </div>
              ))}
            </div>
          );
        })}
      </div>

      {/* フィルター解除 */}
      {selectedDate && (
        <div className="px-2 py-1.5 border-t border-border/40 sticky bottom-0 bg-background">
          <button
            onClick={() => onSelectDate(null)}
            className="text-[10px] text-muted-foreground hover:text-foreground underline transition-colors w-full text-center"
          >
            フィルター解除
          </button>
        </div>
      )}

      {/* backfill ボタン（events 未取得クリップがある場合のみ表示） */}
      {backfillCandidates.length > 0 && (
        <div className="px-2 py-2 border-t border-border/40">
          <button
            onClick={handleRefreshAll}
            disabled={refreshingId !== null}
            className="w-full flex items-center justify-center gap-1.5 text-[10px] text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50"
          >
            <RefreshCw
              className={cn('h-3 w-3', refreshingId !== null && 'animate-spin')}
            />
            {refreshingId !== null
              ? 'イベント日付を取得中...'
              : `イベント日付を取得 (${backfillCandidates.length}件)`}
          </button>
        </div>
      )}
    </div>
  );
}
