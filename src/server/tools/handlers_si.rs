use crate::metrics::{MetricEvent, Timer};
use crate::server::requests::{
    SiApproveRequest, SiDiagnosesRequest, SiRejectRequest, SiRollbackRequest, SiStatusRequest,
    SiTriggerRequest,
};
use crate::server::responses::{
    SiApproveResponse, SiDiagnosesResponse, SiExecutionSummary, SiLearningSummary,
    SiPendingDiagnosis, SiRejectResponse, SiRollbackResponse, SiStatusResponse, SiTriggerResponse,
};

impl super::ReasoningServer {
    pub(super) async fn handle_si_status(&self, _req: SiStatusRequest) -> SiStatusResponse {
        let timer = Timer::start();
        let status = self.state.self_improvement.status().await;

        self.state
            .metrics
            .record(MetricEvent::new("si_status", timer.elapsed_ms(), true));

        SiStatusResponse {
            running: status.running,
            circuit_state: status.circuit_state,
            total_cycles: status.total_cycles,
            successful_cycles: status.successful_cycles,
            failed_cycles: status.failed_cycles,
            pending_diagnoses: status.pending_diagnoses,
            total_actions_executed: status.total_actions_executed,
            total_actions_rolled_back: status.total_actions_rolled_back,
            last_cycle_at: status.last_cycle_at,
            average_reward: status.learning_summary.average_reward,
        }
    }

    pub(super) async fn handle_si_diagnoses(&self, req: SiDiagnosesRequest) -> SiDiagnosesResponse {
        let timer = Timer::start();
        let diagnoses = self
            .state
            .self_improvement
            .pending_diagnoses(req.limit)
            .await;
        let total = diagnoses.len();

        self.state
            .metrics
            .record(MetricEvent::new("si_diagnoses", timer.elapsed_ms(), true));

        SiDiagnosesResponse {
            diagnoses: diagnoses
                .into_iter()
                .map(|d| SiPendingDiagnosis {
                    id: d.id,
                    action_type: d.action_type,
                    description: d.description,
                    rationale: d.rationale,
                    expected_improvement: d.expected_improvement,
                    created_at: d.created_at,
                })
                .collect(),
            total,
        }
    }

    pub(super) async fn handle_si_approve(&self, req: SiApproveRequest) -> SiApproveResponse {
        let timer = Timer::start();
        let result = self.state.self_improvement.approve(req.diagnosis_id).await;

        let (response, success) = match result {
            Ok(approve_result) => (
                SiApproveResponse {
                    success: true,
                    actions_executed: approve_result.execution_results.len(),
                    lessons_learned: approve_result.learning_results.len(),
                    execution_results: approve_result
                        .execution_results
                        .into_iter()
                        .map(|r| SiExecutionSummary {
                            action_id: r.action_id,
                            success: r.success,
                            error: if r.success { None } else { Some(r.message) },
                        })
                        .collect(),
                    learning_results: approve_result
                        .learning_results
                        .into_iter()
                        .map(|r| SiLearningSummary {
                            action_id: r.action_id,
                            insight: r.lesson,
                            reward: r.reward,
                        })
                        .collect(),
                    error: None,
                },
                true,
            ),
            Err(e) => (
                SiApproveResponse {
                    success: false,
                    actions_executed: 0,
                    lessons_learned: 0,
                    execution_results: vec![],
                    learning_results: vec![],
                    error: Some(e),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_approve", timer.elapsed_ms(), success));

        response
    }

    pub(super) async fn handle_si_reject(&self, req: SiRejectRequest) -> SiRejectResponse {
        let timer = Timer::start();
        let result = self
            .state
            .self_improvement
            .reject(req.diagnosis_id, req.reason)
            .await;

        let (response, success) = match result {
            Ok(()) => (
                SiRejectResponse {
                    success: true,
                    error: None,
                },
                true,
            ),
            Err(e) => (
                SiRejectResponse {
                    success: false,
                    error: Some(e),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_reject", timer.elapsed_ms(), success));

        response
    }

    pub(super) async fn handle_si_trigger(&self, _req: SiTriggerRequest) -> SiTriggerResponse {
        let timer = Timer::start();
        let result = self.state.self_improvement.trigger_cycle().await;

        let (response, success) = match result {
            Ok(cycle_result) => {
                let actions_proposed = cycle_result
                    .analysis_result
                    .as_ref()
                    .map_or(0, |a| a.actions.len());
                (
                    SiTriggerResponse {
                        success: true,
                        actions_proposed,
                        actions_executed: cycle_result.execution_results.len(),
                        analysis_skipped: cycle_result.analysis_result.is_none(),
                        error: None,
                    },
                    true,
                )
            }
            Err(e) => (
                SiTriggerResponse {
                    success: false,
                    actions_proposed: 0,
                    actions_executed: 0,
                    analysis_skipped: false,
                    error: Some(format!("{e}")),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_trigger", timer.elapsed_ms(), success));

        response
    }

    pub(super) async fn handle_si_rollback(&self, req: SiRollbackRequest) -> SiRollbackResponse {
        let timer = Timer::start();
        let result = self.state.self_improvement.rollback(req.action_id).await;

        let (response, success) = match result {
            Ok(()) => (
                SiRollbackResponse {
                    success: true,
                    error: None,
                },
                true,
            ),
            Err(e) => (
                SiRollbackResponse {
                    success: false,
                    error: Some(e),
                },
                false,
            ),
        };

        self.state
            .metrics
            .record(MetricEvent::new("si_rollback", timer.elapsed_ms(), success));

        response
    }
}
