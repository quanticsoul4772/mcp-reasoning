// Mirrors `src/dashboard/event.rs` — the JSON shape arriving over `/events`.

export type NodeId =
  | "client"
  | "registry"
  | "mode"
  | "anthropic"
  | "sqlite"
  | "voyage"
  | "worker"
  | "si"
  | "heal"
  | "github";

export type Phase = "started" | "progress" | "completed" | "failed" | "held_back";

export type EdgeId =
  | "client_to_registry"
  | "registry_to_mode"
  | "mode_to_sqlite"
  | "mode_to_anthropic"
  | "mode_to_client"
  | "worker_to_voyage"
  | "si_cycle"
  | "heal_to_github";

export interface ActivityEvent {
  id: number;
  ts_ms: number;
  session_id?: string;
  node: NodeId;
  edge?: EdgeId;
  phase: Phase;
  tool?: string;
  model?: string;
  duration_ms?: number;
  bytes?: number;
  note?: string;
}
