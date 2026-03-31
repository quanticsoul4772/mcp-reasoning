use crate::metrics::{MetricEvent, Timer};
use crate::server::requests::{MetricsRequest, PresetRequest};
use crate::server::responses::{
    Invocation, MetricsResponse, MetricsSummary, ModeStats, PresetExecution, PresetInfo,
    PresetResponse,
};

impl super::ReasoningServer {
    pub(super) async fn handle_preset(&self, req: PresetRequest) -> PresetResponse {
        let timer = Timer::start();
        let operation = req.operation.clone();

        let (response, success) = match operation.as_str() {
            "list" => {
                // List available presets, optionally filtered by category
                let presets: Vec<PresetInfo> = self
                    .state
                    .presets
                    .list()
                    .iter()
                    .filter(|p| {
                        req.category
                            .as_ref()
                            .is_none_or(|cat| p.category.to_string() == *cat)
                    })
                    .map(|p| PresetInfo {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        description: p.description.clone(),
                        category: p.category.to_string(),
                        required_inputs: p
                            .steps
                            .iter()
                            .filter_map(|s| s.config.as_ref().map(|_| s.mode.clone()))
                            .collect(),
                    })
                    .collect();

                (
                    PresetResponse {
                        presets: Some(presets),
                        execution_result: None,
                        session_id: None,
                        metadata: None,
                        next_call: None,
                    },
                    true,
                )
            }
            "run" => {
                // Run a specific preset
                let Some(preset_id) = req.preset_id.clone() else {
                    self.state.metrics.record(
                        MetricEvent::new("preset", timer.elapsed_ms(), false)
                            .with_operation(&operation),
                    );
                    return PresetResponse {
                        presets: None,
                        execution_result: Some(PresetExecution {
                            preset_id: "unknown".to_string(),
                            steps_completed: 0,
                            total_steps: 0,
                            step_results: vec![],
                            final_output: serde_json::json!({
                                "error": "preset_id is required for run operation"
                            }),
                        }),
                        session_id: req.session_id,
                        metadata: None,
                        next_call: Some(
                            serde_json::json!({"tool": "reasoning_preset", "args": {"operation": "list"}}),
                        ),
                    };
                };

                let Some(preset) = self.state.presets.get(&preset_id) else {
                    self.state.metrics.record(
                        MetricEvent::new("preset", timer.elapsed_ms(), false)
                            .with_operation(&operation),
                    );
                    return PresetResponse {
                        presets: None,
                        execution_result: Some(PresetExecution {
                            preset_id: preset_id.clone(),
                            steps_completed: 0,
                            total_steps: 0,
                            step_results: vec![],
                            final_output: serde_json::json!({"error": format!("Preset '{}' not found. Use operation='list' to see available presets.", preset_id)}),
                        }),
                        session_id: req.session_id,
                        metadata: None,
                        next_call: Some(
                            serde_json::json!({"tool": "reasoning_preset", "args": {"operation": "list"}}),
                        ),
                    };
                };

                // Return preset info - actual execution would require running each step
                let total_steps = preset.steps.len() as u32;
                let step_results: Vec<serde_json::Value> = preset
                    .steps
                    .iter()
                    .enumerate()
                    .map(|(i, step)| {
                        serde_json::json!({
                            "step": i,
                            "mode": step.mode,
                            "operation": step.operation,
                            "description": step.description,
                            "status": "pending"
                        })
                    })
                    .collect();

                (
                    PresetResponse {
                        presets: None,
                        execution_result: Some(PresetExecution {
                            preset_id: preset.id.clone(),
                            steps_completed: 0,
                            total_steps,
                            step_results,
                            final_output: serde_json::json!({
                                "name": preset.name,
                                "description": preset.description,
                                "category": preset.category.to_string(),
                                "message": "Preset workflow ready for execution"
                            }),
                        }),
                        session_id: req.session_id,
                        metadata: None,
                        next_call: None,
                    },
                    true,
                )
            }
            _ => (
                PresetResponse {
                    presets: None,
                    execution_result: Some(PresetExecution {
                        preset_id: "unknown".to_string(),
                        steps_completed: 0,
                        total_steps: 0,
                        step_results: vec![],
                        final_output: serde_json::json!({
                            "error": format!(
                                "Unknown operation: {}. Use 'list' or 'run'.",
                                operation
                            )
                        }),
                    }),
                    session_id: req.session_id,
                    metadata: None,
                    next_call: Some(
                        serde_json::json!({"tool": "reasoning_preset", "args": {"operation": "list"}}),
                    ),
                },
                false,
            ),
        };

        self.state.metrics.record(
            MetricEvent::new("preset", timer.elapsed_ms(), success).with_operation(&operation),
        );

        response
    }

    pub(super) async fn handle_metrics(&self, req: MetricsRequest) -> MetricsResponse {
        let timer = Timer::start();
        let query = req.query.clone();

        let (response, success) = match query.as_str() {
            "summary" => {
                let summary = self.state.metrics.summary();
                (
                    MetricsResponse {
                        summary: Some(MetricsSummary {
                            total_calls: summary.total_invocations,
                            success_rate: summary.overall_success_rate,
                            avg_latency_ms: summary
                                .by_mode
                                .values()
                                .map(|m| m.avg_latency_ms)
                                .sum::<f64>()
                                / summary.by_mode.len().max(1) as f64,
                            by_mode: serde_json::to_value(&summary.by_mode).unwrap_or_default(),
                        }),
                        mode_stats: None,
                        invocations: None,
                        config: None,
                    },
                    true,
                )
            }
            "by_mode" => {
                let mode_name = req.mode_name.clone().unwrap_or_default();

                // If mode_name is empty, return summary with all modes instead
                if mode_name.is_empty() {
                    let summary = self.state.metrics.summary();
                    (
                        MetricsResponse {
                            summary: Some(MetricsSummary {
                                total_calls: summary.total_invocations,
                                success_rate: summary.overall_success_rate,
                                avg_latency_ms: summary
                                    .by_mode
                                    .values()
                                    .map(|m| m.avg_latency_ms)
                                    .sum::<f64>()
                                    / summary.by_mode.len().max(1) as f64,
                                by_mode: serde_json::to_value(&summary.by_mode).unwrap_or_default(),
                            }),
                            mode_stats: None,
                            invocations: None,
                            config: None,
                        },
                        true,
                    )
                } else {
                    let events = self.state.metrics.invocations_by_mode(&mode_name);
                    let total = events.len() as u64;
                    let success_count = events.iter().filter(|e| e.success).count() as u64;
                    let failure_count = total - success_count;
                    let success_rate = if total > 0 {
                        success_count as f64 / total as f64
                    } else {
                        0.0
                    };

                    // Calculate latency percentiles
                    let mut latencies: Vec<u64> = events.iter().map(|e| e.latency_ms).collect();
                    latencies.sort_unstable();
                    let p50 = latencies.get(latencies.len() / 2).copied().unwrap_or(0) as f64;
                    let p95 = latencies
                        .get(latencies.len() * 95 / 100)
                        .copied()
                        .unwrap_or(0) as f64;
                    let p99 = latencies
                        .get(latencies.len() * 99 / 100)
                        .copied()
                        .unwrap_or(0) as f64;

                    (
                        MetricsResponse {
                            summary: None,
                            mode_stats: Some(ModeStats {
                                mode_name,
                                call_count: total,
                                success_count,
                                failure_count,
                                success_rate,
                                latency_p50_ms: p50,
                                latency_p95_ms: p95,
                                latency_p99_ms: p99,
                            }),
                            invocations: None,
                            config: None,
                        },
                        true,
                    )
                }
            }
            "invocations" => {
                let events = req.mode_name.as_ref().map_or_else(
                    || {
                        self.state
                            .metrics
                            .summary()
                            .by_mode
                            .keys()
                            .flat_map(|mode| self.state.metrics.invocations_by_mode(mode))
                            .collect()
                    },
                    |mode| self.state.metrics.invocations_by_mode(mode),
                );

                let limit = req.limit.unwrap_or(100).min(1000) as usize;
                let invocations: Vec<Invocation> = events
                    .into_iter()
                    .filter(|e| req.success_only.is_none_or(|s| !s || e.success))
                    .take(limit)
                    .enumerate()
                    .map(|(i, e)| {
                        #[allow(clippy::cast_possible_wrap)]
                        let created_at = chrono::DateTime::from_timestamp(e.timestamp as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default();
                        Invocation {
                            id: format!("inv-{i}"),
                            tool_name: e.mode.clone(),
                            session_id: req.session_id.clone(),
                            success: e.success,
                            latency_ms: e.latency_ms,
                            created_at,
                        }
                    })
                    .collect();

                (
                    MetricsResponse {
                        summary: None,
                        mode_stats: None,
                        invocations: Some(invocations),
                        config: None,
                    },
                    true,
                )
            }
            "fallbacks" => {
                let fallbacks = self.state.metrics.fallbacks();
                (
                    MetricsResponse {
                        summary: None,
                        mode_stats: None,
                        invocations: Some(
                            fallbacks
                                .into_iter()
                                .enumerate()
                                .map(|(i, f)| {
                                    #[allow(clippy::cast_possible_wrap)]
                                    let created_at =
                                        chrono::DateTime::from_timestamp(f.timestamp as i64, 0)
                                            .map(|dt| dt.to_rfc3339())
                                            .unwrap_or_default();
                                    Invocation {
                                        id: format!("fallback-{i}"),
                                        tool_name: format!("{} -> {}", f.from_mode, f.to_mode),
                                        session_id: Some(f.reason),
                                        success: false,
                                        latency_ms: 0,
                                        created_at,
                                    }
                                })
                                .collect(),
                        ),
                        config: None,
                    },
                    true,
                )
            }
            "config" => (
                MetricsResponse {
                    summary: None,
                    mode_stats: None,
                    invocations: None,
                    config: Some(serde_json::json!({
                        "model": self.state.config.model,
                        "request_timeout_ms": self.state.config.request_timeout_ms,
                        "max_retries": self.state.config.max_retries,
                        "log_level": self.state.config.log_level,
                    })),
                },
                true,
            ),
            _ => (
                MetricsResponse {
                    summary: None,
                    mode_stats: None,
                    invocations: None,
                    config: Some(serde_json::json!({
                        "error": format!(
                            "Unknown query: {}. Use 'summary', 'by_mode', 'invocations', 'fallbacks', or 'config'.",
                            query
                        )
                    })),
                },
                false,
            ),
        };

        self.state.metrics.record(
            MetricEvent::new("metrics", timer.elapsed_ms(), success).with_operation(&query),
        );

        response
    }
}
