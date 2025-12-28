# Self-Improvement System Remediation Plan

## Problem Statement

The self-improvement system was implemented with simplified types that deviate from the Design Doc (Section 14). This plan restores the full specification.

---

## Phase R1: Core Types Restoration (types.rs)

### R1.1: Fix Severity Enum

**Current (WRONG):**
```rust
pub enum Severity { Low, Medium, High, Critical }
```

**Required (DESIGN.md 14.2):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info = 0,      // Minor deviation, no action needed
    Warning = 1,   // Moderate deviation, consider action
    High = 2,      // Significant deviation, action recommended
    Critical = 3,  // Severe deviation, immediate action required
}

impl Severity {
    pub fn from_deviation(deviation_pct: f64) -> Self {
        match deviation_pct {
            d if d >= 100.0 => Severity::Critical,
            d if d >= 50.0 => Severity::High,
            d if d >= 25.0 => Severity::Warning,
            _ => Severity::Info,
        }
    }
}
```

### R1.2: Add TriggerMetric Enum (NEW)

**Required (DESIGN.md 14.2):**
```rust
#[derive(Debug, Clone)]
pub enum TriggerMetric {
    ErrorRate { observed: f64, baseline: f64, threshold: f64 },
    Latency { observed_p95_ms: i64, baseline_ms: i64, threshold_ms: i64 },
    QualityScore { observed: f64, baseline: f64, minimum: f64 },
}

impl TriggerMetric {
    pub fn deviation_pct(&self) -> f64 {
        match self {
            TriggerMetric::ErrorRate { observed, baseline, .. } => {
                if *baseline == 0.0 {
                    if *observed > 0.0 { 100.0 } else { 0.0 }
                } else {
                    ((observed - baseline) / baseline) * 100.0
                }
            }
            TriggerMetric::Latency { observed_p95_ms, baseline_ms, .. } => {
                if *baseline_ms == 0 {
                    if *observed_p95_ms > 0 { 100.0 } else { 0.0 }
                } else {
                    ((*observed_p95_ms - *baseline_ms) as f64 / *baseline_ms as f64) * 100.0
                }
            }
            TriggerMetric::QualityScore { observed, baseline, .. } => {
                if *baseline == 0.0 {
                    if *observed < 1.0 { 100.0 } else { 0.0 }
                } else {
                    ((baseline - observed) / baseline) * 100.0  // Inverted: lower is worse
                }
            }
        }
    }

    pub fn severity(&self) -> Severity {
        Severity::from_deviation(self.deviation_pct())
    }
}
```

### R1.3: Add ParamValue Enum (NEW)

**Required (DESIGN.md 14.2):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamValue {
    Integer(i64),
    Float(f64),
    String(String),
    DurationMs(u64),
    Boolean(bool),
}
```

### R1.4: Add ConfigScope Enum (NEW)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigScope {
    Global,
    Mode(ReasoningMode),
    Tool(ToolName),
}
```

### R1.5: Add ResourceType Enum (NEW)

**Required (DESIGN.md 14.2):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    MaxConcurrentRequests,
    ConnectionPoolSize,
    CacheSize,
    TimeoutMs,
    MaxRetries,
    RetryDelayMs,
}
```

### R1.6: Add SuggestedAction Enum (NEW)

**Required (DESIGN.md 14.2):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestedAction {
    AdjustParam {
        key: String,
        old_value: ParamValue,
        new_value: ParamValue,
        scope: ConfigScope,
    },
    ScaleResource {
        resource: ResourceType,
        old_value: u32,
        new_value: u32,
    },
    NoOp {
        reason: String,
        revisit_after: std::time::Duration,
    },
}
```

### R1.7: Add SelfDiagnosis Struct (NEW)

**Required (DESIGN.md 14.2):**
```rust
pub type DiagnosisId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosisStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfDiagnosis {
    pub id: DiagnosisId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub trigger: TriggerMetric,
    pub severity: Severity,
    pub description: String,
    pub suspected_cause: Option<String>,
    pub suggested_action: SuggestedAction,
    pub action_rationale: Option<String>,
    pub status: DiagnosisStatus,
}
```

### R1.8: Add NormalizedReward (NEW)

**Required (DESIGN.md 14.2):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardBreakdown {
    pub error_rate_component: f64,
    pub latency_component: f64,
    pub quality_component: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedReward {
    pub value: f64,           // -1.0 to 1.0 (positive = improvement)
    pub breakdown: RewardBreakdown,
    pub confidence: f64,      // Based on sample size
}

impl NormalizedReward {
    pub fn calculate(
        trigger: &TriggerMetric,
        pre_metrics: &MetricsSnapshot,
        post_metrics: &MetricsSnapshot,
        baselines: &Baselines,
    ) -> Self;

    pub fn is_positive(&self) -> bool { self.value > 0.0 }
    pub fn is_negative(&self) -> bool { self.value < 0.0 }
}
```

---

## Phase R2: Monitor Restoration (monitor.rs)

### R2.1: Add InvocationEvent

**Required (DESIGN.md 14.3):**
```rust
#[derive(Debug, Clone)]
pub struct InvocationEvent {
    pub tool_name: String,
    pub latency_ms: i64,
    pub success: bool,
    pub quality_score: Option<f64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### R2.2: Add HealthReport

**Required (DESIGN.md 14.3):**
```rust
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub current_metrics: MetricsSnapshot,
    pub baselines: Baselines,
    pub triggers: Vec<TriggerMetric>,
    pub is_healthy: bool,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}
```

### R2.3: Refactor Monitor Struct

**Required (DESIGN.md 14.3):**
```rust
pub struct Monitor {
    config: MonitorConfig,
    baselines: RwLock<BaselineCollection>,
    raw_metrics: RwLock<RawMetrics>,
}

impl Monitor {
    /// Record invocation (called on EVERY request)
    pub async fn record_invocation(&self, event: InvocationEvent);

    /// Check health - returns report if enough samples
    pub async fn check_health(&self) -> Option<HealthReport>;

    /// Force health check regardless of timing
    pub async fn force_check(&self) -> Option<HealthReport>;

    /// Get current baselines
    pub async fn get_baselines(&self) -> Baselines;

    /// Get current aggregated metrics
    pub async fn get_current_metrics(&self) -> MetricsSnapshot;
}
```

---

## Phase R3: Analyzer Restoration (analyzer.rs)

### R3.1: Add AnalysisBlocked Enum

**Required (DESIGN.md 14.4):**
```rust
#[derive(Debug)]
pub enum AnalysisBlocked {
    CircuitOpen { remaining_secs: u64 },
    NoTriggers,
    SeverityTooLow { severity: Severity, minimum: Severity },
    MaxPendingReached { count: u32 },
}
```

### R3.2: Refactor AnalysisResult

**Required (DESIGN.md 14.4):**
```rust
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub diagnosis: SelfDiagnosis,
    pub analysis_stats: AnalyzerStats,
}

#[derive(Debug, Clone)]
pub struct AnalyzerStats {
    pub analysis_time_ms: u64,
    pub tokens_used: u32,
}
```

### R3.3: Refactor Analyzer

**Required (DESIGN.md 14.4):**
```rust
pub struct Analyzer {
    config: AnalyzerConfig,
    anthropic: Arc<AnthropicClient>,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
}

impl Analyzer {
    pub async fn analyze(&self, health: &HealthReport) -> Result<AnalysisResult, AnalysisBlocked>;
}
```

---

## Phase R4: Executor Restoration (executor.rs)

### R4.1: Add ActionOutcome Enum

**Required (DESIGN.md 14.5):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionOutcome {
    Pending,
    Success,
    Failed,
    RolledBack,
}
```

### R4.2: Add ExecutionBlocked Enum

**Required (DESIGN.md 14.5):**
```rust
#[derive(Debug)]
pub enum ExecutionBlocked {
    CircuitOpen { remaining_secs: u64 },
    CooldownActive { remaining_secs: u64 },
    RateLimitExceeded { count: u32, max: u32 },
    NotAllowed { reason: String },
    NoOpAction { reason: String },
}
```

### R4.3: Refactor ExecutionResult

**Required (DESIGN.md 14.5):**
```rust
pub type ActionId = String;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub action_id: ActionId,
    pub diagnosis_id: DiagnosisId,
    pub outcome: ActionOutcome,
    pub pre_metrics: MetricsSnapshot,
    pub execution_time_ms: u64,
}
```

### R4.4: Refactor Executor

**Required (DESIGN.md 14.5):**
```rust
pub struct Executor {
    config: ExecutorConfig,
    allowlist: ActionAllowlist,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    config_state: RwLock<ConfigState>,
}

impl Executor {
    pub async fn execute(
        &self,
        diagnosis: &SelfDiagnosis,
        current_metrics: &MetricsSnapshot,
    ) -> Result<ExecutionResult, ExecutionBlocked>;

    pub async fn rollback_by_id(&self, action_id: &str) -> Result<(), ExecutorError>;
}
```

---

## Phase R5: Learner Restoration (learner.rs)

### R5.1: Add Learning Types

**Required (DESIGN.md 14.6):**
```rust
#[derive(Debug, Clone)]
pub struct LearningOutcome {
    pub reward: NormalizedReward,
    pub action_effectiveness: ActionEffectiveness,
    pub learning_synthesis: Option<LearningSynthesis>,
}

#[derive(Debug, Clone)]
pub struct LearningSynthesis {
    pub lessons: Vec<String>,
    pub future_recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ActionEffectiveness {
    pub expected_improvement: f64,
    pub actual_improvement: f64,
    pub effectiveness_ratio: f64,
}

#[derive(Debug)]
pub enum LearningBlocked {
    ExecutionNotCompleted { status: ActionOutcome },
    InsufficientSamples { required: u64, actual: u64 },
}
```

### R5.2: Refactor Learner

**Required (DESIGN.md 14.6):**
```rust
pub struct Learner {
    config: LearnerConfig,
    anthropic: Arc<AnthropicClient>,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
}

impl Learner {
    pub async fn learn(
        &self,
        execution_result: &ExecutionResult,
        diagnosis: &SelfDiagnosis,
        post_metrics: &MetricsSnapshot,
        baselines: &Baselines,
    ) -> Result<LearningOutcome, LearningBlocked>;
}
```

---

## Phase R6: Allowlist Restoration (allowlist.rs)

### R6.1: Add ParamBounds and ResourceBounds

**Required (DESIGN.md 14.7):**
```rust
#[derive(Debug, Clone)]
pub struct ParamBounds {
    pub min: ParamValue,
    pub max: ParamValue,
    pub step: Option<ParamValue>,
}

#[derive(Debug, Clone)]
pub struct ResourceBounds {
    pub min: u32,
    pub max: u32,
    pub step: Option<u32>,
}
```

### R6.2: Refactor ActionAllowlist

**Required (DESIGN.md 14.7):**
```rust
pub struct ActionAllowlist {
    allowed_params: HashMap<String, ParamBounds>,
    allowed_resources: HashMap<ResourceType, ResourceBounds>,
}

impl ActionAllowlist {
    pub fn default_allowlist() -> Self;
    pub fn validate(&self, action: &SuggestedAction) -> Result<(), AllowlistError>;
}
```

---

## Phase R7: System Restoration (system.rs)

### R7.1: Add SelfImprovementError

**Required (DESIGN.md 14.9):**
```rust
#[derive(Debug)]
pub enum SelfImprovementError {
    CircuitBreakerOpen { consecutive_failures: u32 },
    InCooldown { until: chrono::DateTime<chrono::Utc> },
    RateLimitExceeded { count: u32, max: u32 },
    MonitorFailed { message: String },
    AnalyzerFailed { message: String },
    ExecutorFailed { message: String },
    LearnerFailed { message: String },
}
```

### R7.2: Refactor SelfImprovementSystem

**Required (DESIGN.md 14.9):**
```rust
pub struct SelfImprovementSystem {
    config: SelfImprovementConfig,
    monitor: Monitor,
    analyzer: Analyzer,
    executor: Executor,
    learner: Learner,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    allowlist: ActionAllowlist,
    state: Arc<RwLock<SystemState>>,
}

impl SelfImprovementSystem {
    /// Always returns true - system cannot be disabled
    pub fn is_enabled(&self) -> bool { true }

    /// Record invocation (called after EVERY tool use)
    pub async fn on_invocation(&self, event: InvocationEvent);

    /// Check health
    pub async fn check_health(&self) -> Option<HealthReport>;

    /// Run one improvement cycle
    pub async fn run_cycle(&self) -> Result<CycleResult, SelfImprovementError>;

    /// Get current system status
    pub async fn status(&self) -> SystemStatus;
}
```

---

## Phase R8: New Files

### R8.1: baseline.rs (NEW)

Baseline calculation with EMA and rolling average.

```rust
pub struct BaselineCollection {
    error_rate: Baseline,
    latency_p95: Baseline,
    quality_score: Baseline,
}

pub struct Baseline {
    ema: f64,
    rolling_avg: f64,
    sample_count: u64,
}

impl Baseline {
    pub fn update(&mut self, value: f64);
    pub fn value(&self) -> f64;
}
```

### R8.2: storage.rs (NEW)

Database operations for self-improvement tables.

### R8.3: cli.rs (NEW)

**Required (DESIGN.md 14.11):**
```rust
#[derive(Subcommand, Debug, Clone)]
pub enum SelfImproveCommands {
    Status,
    History { limit: usize, outcome: Option<String> },
    Diagnostics { verbose: bool },
    Config,
    CircuitBreaker,
    Baselines,
    Pause { duration: String },
    Rollback { action_id: String },
    Approve { diagnosis_id: String },
    Reject { diagnosis_id: String, reason: Option<String> },
}
```

### R8.4: anthropic_calls.rs (NEW)

**Required (DESIGN.md 14.12):**
```rust
pub async fn generate_diagnosis(health: &HealthReport) -> Result<DiagnosisContent, AnthropicError>;
pub async fn select_action(diagnosis: &DiagnosisContent) -> Result<SuggestedAction, AnthropicError>;
pub async fn validate_decision(action: &SuggestedAction) -> Result<ValidationResult, AnthropicError>;
pub async fn synthesize_learning(outcome: &LearningOutcome) -> Result<LearningSynthesis, AnthropicError>;
```

---

## Phase R9: Database Schema

Add to migrations (DESIGN.md 14.10):

```sql
-- Invocation records (fed by Monitor)
CREATE TABLE IF NOT EXISTS invocations (
    id TEXT PRIMARY KEY,
    tool_name TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    success INTEGER NOT NULL,
    quality_score REAL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_invocations_created_at ON invocations(created_at);
CREATE INDEX IF NOT EXISTS idx_invocations_tool ON invocations(tool_name);

-- Diagnosis records
CREATE TABLE IF NOT EXISTS diagnoses (
    id TEXT PRIMARY KEY,
    trigger_type TEXT NOT NULL,
    trigger_json TEXT NOT NULL,
    severity TEXT NOT NULL,
    description TEXT NOT NULL,
    suspected_cause TEXT,
    suggested_action_json TEXT NOT NULL,
    action_rationale TEXT,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_diagnoses_status ON diagnoses(status);

-- Action records (executed by Executor)
CREATE TABLE IF NOT EXISTS si_actions (
    id TEXT PRIMARY KEY,
    diagnosis_id TEXT NOT NULL REFERENCES diagnoses(id),
    action_type TEXT NOT NULL,
    action_json TEXT NOT NULL,
    outcome TEXT NOT NULL,
    pre_metrics_json TEXT NOT NULL,
    post_metrics_json TEXT,
    execution_time_ms INTEGER NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_si_actions_diagnosis ON si_actions(diagnosis_id);
CREATE INDEX IF NOT EXISTS idx_si_actions_outcome ON si_actions(outcome);

-- Learning records
CREATE TABLE IF NOT EXISTS learnings (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL REFERENCES si_actions(id),
    reward_value REAL NOT NULL,
    reward_breakdown_json TEXT NOT NULL,
    confidence REAL NOT NULL,
    lessons_json TEXT,
    recommendations_json TEXT,
    created_at TEXT NOT NULL
);

-- Config overrides (applied by Executor, read at startup)
CREATE TABLE IF NOT EXISTS config_overrides (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    applied_by_action TEXT REFERENCES si_actions(id),
    updated_at TEXT NOT NULL
);
```

---

## Phase R10: Integration

### R10.1: Wire Self-Improvement to Tool Handlers

Every tool handler must call `self_improvement.on_invocation()` after execution.

### R10.2: Update Tests

All existing tests must be updated to use new types. Add new tests for:
- TriggerMetric::deviation_pct()
- Severity::from_deviation()
- NormalizedReward::calculate()
- SuggestedAction validation
- Full cycle with new types

---

## Execution Order

```
R1 → R2 → R3 → R4 → R5 → R6 → R7 → R8 → R9 → R10
```

Each phase depends on the previous. Tests must pass at each checkpoint.

---

## Estimated Scope

| Phase | Files Modified | New Tests |
|-------|----------------|-----------|
| R1 | types.rs | ~25 |
| R2 | monitor.rs | ~15 |
| R3 | analyzer.rs | ~10 |
| R4 | executor.rs | ~15 |
| R5 | learner.rs | ~10 |
| R6 | allowlist.rs | ~10 |
| R7 | system.rs | ~15 |
| R8 | 4 new files | ~30 |
| R9 | migration | ~5 |
| R10 | handlers, integration | ~20 |
| **Total** | ~15 files | ~155 tests |
