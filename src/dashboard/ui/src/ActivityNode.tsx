import { Handle, Position, type NodeProps } from "@xyflow/react";
import { KIND_COLOR, type ActivityNodeData } from "./flowDefs";

const PHASE_GLOW: Record<string, string> = {
  completed: "#7fe0a8",
  failed: "#ff8d8d",
  started: "#ffffff",
  progress: "#ffffff",
  held_back: "#ffcf7a",
};

/**
 * A flow node that flashes when activity arrives. Four sides each carry an
 * overlapping source+target handle (hidden) so edges can connect on any side.
 */
export function ActivityNode({ data }: NodeProps) {
  const d = data as ActivityNodeData;
  const color = KIND_COLOR[d.kind];
  const glow = d.active ? (PHASE_GLOW[d.phase ?? ""] ?? "#ffffff") : undefined;

  return (
    <div
      className={`anode${d.active ? " active" : ""}`}
      style={{
        borderColor: color,
        boxShadow: glow ? `0 0 0 2px ${glow}, 0 0 18px ${glow}` : undefined,
        background: d.active ? glow : undefined,
      }}
    >
      <Handle id="tt" type="target" position={Position.Top} />
      <Handle id="ts" type="source" position={Position.Top} />
      <Handle id="bt" type="target" position={Position.Bottom} />
      <Handle id="bs" type="source" position={Position.Bottom} />
      <Handle id="lt" type="target" position={Position.Left} />
      <Handle id="ls" type="source" position={Position.Left} />
      <Handle id="rt" type="target" position={Position.Right} />
      <Handle id="rs" type="source" position={Position.Right} />
      <div className="anode-label">{d.label}</div>
      {d.sub && <div className="anode-sub">{d.sub}</div>}
      <div className="anode-count">{d.count}</div>
    </div>
  );
}
