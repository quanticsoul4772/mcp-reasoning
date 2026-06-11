import type { MetricsSnapshot } from "./useMetrics";

function pct(x: number): string {
  return `${Math.round(x * 100)}%`;
}

interface TransitionRow {
  from: string;
  to: string;
  count: number;
  success: number;
}

function topTransitions(snap: MetricsSnapshot, limit = 5): TransitionRow[] {
  const rows: TransitionRow[] = [];
  for (const [from, tos] of Object.entries(snap.chains.transitions)) {
    for (const [to, s] of Object.entries(tos)) {
      rows.push({ from, to, count: s.count, success: s.success_rate });
    }
  }
  rows.sort((a, b) => b.count - a.count);
  return rows.slice(0, limit);
}

export function Metrics({ snap }: { snap: MetricsSnapshot | null }) {
  if (!snap) {
    return (
      <section className="metrics">
        <h2>Metrics</h2>
        <div className="metrics-empty">waiting for /metrics…</div>
      </section>
    );
  }

  const modes = Object.entries(snap.usage.by_mode)
    .map(([name, m]) => ({ name, calls: m.total_invocations, latency: Math.round(m.avg_latency_ms), success: m.success_rate }))
    .sort((a, b) => b.calls - a.calls)
    .slice(0, 7);

  const si = snap.self_improvement;
  const transitions = topTransitions(snap);

  return (
    <section className="metrics">
      <h2>Metrics</h2>
      <div className="stat-row">
        <div className="stat">
          <div className="stat-num">{snap.usage.total_invocations}</div>
          <div className="stat-lbl">invocations</div>
        </div>
        <div className="stat">
          <div className="stat-num">{pct(snap.usage.overall_success_rate)}</div>
          <div className="stat-lbl">success</div>
        </div>
        <div className="stat">
          <div className="stat-num">{si.total_cycles}</div>
          <div className="stat-lbl">SI cycles</div>
        </div>
        <div className="stat">
          <div className="stat-num">{snap.heal.recurring_defects}</div>
          <div className="stat-lbl">heal defects</div>
        </div>
      </div>

      {modes.length > 0 && (
        <div className="panel">
          <div className="panel-title">avg latency by tool (ms)</div>
          <div className="bars">
            {modes.map((m) => {
              const max = Math.max(...modes.map((x) => x.latency), 1);
              return (
                <div className="bar-row" key={m.name} title={`${m.calls} calls · ${pct(m.success)} ok`}>
                  <span className="bar-name">{m.name}</span>
                  <span className="bar-track">
                    <span
                      className="bar-fill"
                      style={{
                        width: `${Math.max(4, (m.latency / max) * 100)}%`,
                        background: m.success >= 0.5 ? "var(--proc)" : "var(--guard)",
                      }}
                    />
                  </span>
                  <span className="bar-val">{m.latency}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      <div className="panel">
        <div className="panel-title">
          self-improvement · <span className={`cb cb-${si.circuit_state.toLowerCase()}`}>{si.circuit_state}</span>
        </div>
        <div className="kv">
          <span>cycles {si.successful_cycles}/{si.total_cycles} ok</span>
          <span>pending {si.pending_diagnoses}</span>
          <span>{si.running ? "running" : "idle"}</span>
        </div>
      </div>

      {transitions.length > 0 && (
        <div className="panel">
          <div className="panel-title">top tool transitions</div>
          <div className="transitions">
            {transitions.map((t) => (
              <div className="trow" key={`${t.from}->${t.to}`}>
                <span className="tlabel">{t.from} → {t.to}</span>
                <span className="tcount">{t.count}×</span>
                <span className={`tok ${t.success < 0.5 ? "bad" : ""}`}>{pct(t.success)}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
