import { useEffect, useState } from "react";

export interface ModeSummary {
  total_invocations: number;
  successful: number;
  failed: number;
  avg_latency_ms: number;
  min_latency_ms: number;
  max_latency_ms: number;
  success_rate: number;
}

export interface MetricsSnapshot {
  usage: {
    total_invocations: number;
    overall_success_rate: number;
    by_mode: Record<string, ModeSummary>;
  };
  chains: {
    transitions: Record<string, Record<string, { count: number; success_rate: number }>>;
    anti_patterns: { from_tool: string; to_tool: string; success_rate: number; occurrences: number }[];
  };
  self_improvement: {
    running: boolean;
    circuit_state: string;
    total_cycles: number;
    successful_cycles: number;
    failed_cycles: number;
    pending_diagnoses: number;
  };
  heal: { recurring_defects: number };
}

/** Poll `/metrics` on an interval. Returns the latest snapshot (or null). */
export function useMetrics(intervalMs = 3000): MetricsSnapshot | null {
  const [data, setData] = useState<MetricsSnapshot | null>(null);

  useEffect(() => {
    let alive = true;
    const fetchOnce = async () => {
      try {
        const r = await fetch("/metrics");
        if (!r.ok) return;
        const json = (await r.json()) as MetricsSnapshot;
        if (alive) setData(json);
      } catch {
        /* transient; keep last snapshot */
      }
    };
    fetchOnce();
    const t = setInterval(fetchOnce, intervalMs);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [intervalMs]);

  return data;
}
