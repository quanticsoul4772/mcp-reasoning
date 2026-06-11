import type { Edge, Node } from "@xyflow/react";
import { MarkerType } from "@xyflow/react";

/** Visual role of a node — drives its color. */
export type NodeKind = "client" | "proc" | "store" | "ext" | "guard";

export interface ActivityNodeData {
  label: string;
  sub?: string;
  kind: NodeKind;
  count: number;
  active: boolean;
  phase?: string;
  [key: string]: unknown;
}

export const KIND_COLOR: Record<NodeKind, string> = {
  client: "#7b4fd0",
  proc: "#2f6fed",
  store: "#cf8a17",
  ext: "#1f9d63",
  guard: "#d64545",
};

const n = (
  id: string,
  x: number,
  y: number,
  label: string,
  kind: NodeKind,
  sub?: string,
): Node<ActivityNodeData> => ({
  id,
  type: "activity",
  position: { x, y },
  data: { label, sub, kind, count: 0, active: false },
});

// Layout: request spine down the left (client → registry → mode), datastore +
// external API on the right, and a non-overlapping bottom band for the three
// background loops. Matches docs/reference/FLOW_OVERVIEW.md.
export const initialNodes: Node<ActivityNodeData>[] = [
  n("client", 60, 0, "MCP Client", "client", "Claude Code · Desktop"),
  n("registry", 60, 110, "Tool Registry", "proc", "35 tools → handler"),
  n("mode", 60, 250, "Reasoning Mode", "proc", "ModeCore"),
  n("sqlite", 400, 110, "SQLite", "store", "sessions · thoughts · graph"),
  n("anthropic", 400, 250, "Anthropic API", "ext", "reasoning + thinking"),
  n("heal", 40, 400, "Self-heal", "guard", "off by default"),
  n("si", 230, 400, "Self-improvement", "proc", "tune thresholds"),
  n("worker", 430, 400, "Embedding worker", "proc", "drains queue"),
  n("voyage", 640, 400, "Voyage AI", "ext", "embed + rerank"),
  n("github", 40, 510, "GitHub PR", "ext", "never merged"),
];

interface EdgeSpec {
  id: string;
  source: string;
  target: string;
  sourceHandle: string;
  targetHandle: string;
  label?: string;
  async?: boolean;
}

// `EdgeId`s from the schema get stable ids so events can pulse them; the
// structural interval-trigger edges (db → loop) use descriptive ids.
const edgeSpecs: EdgeSpec[] = [
  { id: "client_to_registry", source: "client", target: "registry", sourceHandle: "bs", targetHandle: "tt" },
  { id: "registry_to_mode", source: "registry", target: "mode", sourceHandle: "bs", targetHandle: "tt" },
  { id: "mode_to_sqlite", source: "mode", target: "sqlite", sourceHandle: "rs", targetHandle: "lt", label: "①④ load · persist" },
  { id: "mode_to_anthropic", source: "mode", target: "anthropic", sourceHandle: "rs", targetHandle: "lt", label: "②③ prompt · completion" },
  { id: "mode_to_client", source: "mode", target: "client", sourceHandle: "ls", targetHandle: "lt", label: "⑤" },
  { id: "db_to_worker", source: "sqlite", target: "worker", sourceHandle: "bs", targetHandle: "tt", async: true },
  { id: "db_to_si", source: "sqlite", target: "si", sourceHandle: "bs", targetHandle: "tt", async: true },
  { id: "db_to_heal", source: "sqlite", target: "heal", sourceHandle: "bs", targetHandle: "tt", async: true },
  { id: "worker_to_voyage", source: "worker", target: "voyage", sourceHandle: "rs", targetHandle: "lt", async: true },
  { id: "heal_to_github", source: "heal", target: "github", sourceHandle: "bs", targetHandle: "tt", async: true },
];

export const initialEdges: Edge[] = edgeSpecs.map((e) => ({
  id: e.id,
  source: e.source,
  target: e.target,
  sourceHandle: e.sourceHandle,
  targetHandle: e.targetHandle,
  label: e.label,
  animated: e.async ?? false,
  data: { async: e.async ?? false, base: e.async ? "#5a6a8e" : "#3a4a6e" },
  style: {
    stroke: e.async ? "#5a6a8e" : "#3a4a6e",
    strokeWidth: 2,
    strokeDasharray: e.async ? "5 5" : undefined,
  },
  labelStyle: { fill: "#8ea2c7", fontSize: 10.5 },
  labelBgStyle: { fill: "#15203a", fillOpacity: 0.85 },
  markerEnd: { type: MarkerType.ArrowClosed, color: e.async ? "#5a6a8e" : "#3a4a6e", width: 14, height: 14 },
}));
