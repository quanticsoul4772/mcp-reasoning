import type { ActivityEvent } from "./types";

function fmtTime(ms: number): string {
  if (!ms) return "--:--:--";
  return new Date(ms).toLocaleTimeString([], { hour12: false });
}

function detail(ev: ActivityEvent): string {
  return [ev.phase, ev.duration_ms != null ? `${ev.duration_ms}ms` : null, ev.note]
    .filter(Boolean)
    .join(" · ");
}

export function Timeline({ events }: { events: ActivityEvent[] }) {
  return (
    <aside className="timeline">
      <h2>Timeline</h2>
      <div className="timeline-rows">
        {events.map((ev) => (
          <div key={ev.id || ev.ts_ms + ev.node} className={`row phase-${ev.phase}`}>
            <span className="t">{fmtTime(ev.ts_ms)}</span>
            <span className="n">{ev.tool ?? ev.node}</span>
            <span className="d">{detail(ev)}</span>
          </div>
        ))}
        {events.length === 0 && <div className="row empty">waiting for activity…</div>}
      </div>
    </aside>
  );
}
