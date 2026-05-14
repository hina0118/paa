import { useState, useEffect, useRef, useLayoutEffect } from 'react';
import { getNewsClips } from '@/lib/news/clips';
import { cn } from '@/lib/utils';

interface TickerEvent {
  date: string; // YYYY-MM-DD
  label: string;
  isToday: boolean;
}

function getTodayStr(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function formatDate(dateStr: string): string {
  const [year, month, day] = dateStr.split('-');
  return `${year}/${month}/${day}`;
}

const TICKER_DAYS = 7;

function collectUpcomingEvents(
  clips: Awaited<ReturnType<typeof getNewsClips>>
): TickerEvent[] {
  const todayStr = getTodayStr();
  const today = new Date(todayStr);
  const limit = new Date(today);
  limit.setDate(limit.getDate() + TICKER_DAYS);

  const events: TickerEvent[] = [];
  for (const clip of clips) {
    for (const ev of clip.events) {
      const evDate = new Date(ev.date);
      if (evDate >= today && evDate <= limit) {
        events.push({
          date: ev.date,
          label: ev.label,
          isToday: ev.date === todayStr,
        });
      }
    }
  }

  events.sort((a, b) => a.date.localeCompare(b.date));
  return events;
}

const REFRESH_INTERVAL_MS = 5 * 60 * 1000; // 5分
const PX_PER_SECOND = 80; // スクロール速度(px/秒)

function TickerItem({ ev }: { ev: TickerEvent }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1',
        ev.isToday ? 'text-primary font-semibold' : 'text-muted-foreground'
      )}
    >
      {ev.isToday && (
        <span className="inline-block rounded bg-primary px-1 py-px text-[10px] leading-none text-primary-foreground">
          TODAY
        </span>
      )}
      <span>{formatDate(ev.date)}</span>
      <span>{ev.label}</span>
    </span>
  );
}

function TickerList({ events }: { events: TickerEvent[] }) {
  return (
    <>
      {events.map((ev, i) => (
        <span key={`${ev.date}-${i}`}>
          <TickerItem ev={ev} />
          <span className="mx-4 text-muted-foreground/50" aria-hidden>
            ／
          </span>
        </span>
      ))}
    </>
  );
}

export function EventTicker() {
  const [events, setEvents] = useState<TickerEvent[]>([]);
  const [duration, setDuration] = useState<number>(40);
  const trackRef = useRef<HTMLDivElement>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        const clips = await getNewsClips();
        if (active) setEvents(collectUpcomingEvents(clips));
      } catch {
        /* noop */
      }
    };

    load();
    intervalRef.current = setInterval(load, REFRESH_INTERVAL_MS);

    return () => {
      active = false;
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, []);

  // コンテンツ幅に応じてアニメーション時間を計算 (1周分 = 幅の半分)
  useLayoutEffect(() => {
    if (!trackRef.current) return;
    const halfWidth = trackRef.current.scrollWidth / 2;
    if (halfWidth > 0) {
      setDuration(halfWidth / PX_PER_SECOND);
    }
  }, [events]);

  if (events.length === 0) return null;

  return (
    <div className="flex h-6 shrink-0 items-center overflow-hidden border-b bg-muted/40 text-xs select-none">
      {/* ラベル */}
      <div className="flex shrink-0 items-center gap-1 border-r bg-muted px-2 h-full text-muted-foreground">
        <span className="text-[10px] leading-none">📅</span>
        <span className="font-medium text-[11px]">イベント</span>
      </div>

      {/* スクロール領域 */}
      <div className="flex-1 overflow-hidden">
        {/* 2周分を並べてシームレスループ */}
        <div
          ref={trackRef}
          className="ticker-track inline-flex items-center whitespace-nowrap"
          style={{ animationDuration: `${duration}s` }}
        >
          <TickerList events={events} />
          <TickerList events={events} />
        </div>
      </div>
    </div>
  );
}
