# Self-Improvement System Integration Design

## Executive Summary

This document outlines the design for integrating the existing self-improvement system into the MCP reasoning server. The system is fully implemented but not wired into the main server loop.

**Design Principle**: Self-improvement is a **core feature**, not optional. It runs automatically whenever the server runs.

## Current State Analysis

### Existing Components (Fully Implemented)

```
src/self_improvement/
├── mod.rs              # Re-exports
├── system.rs           # SelfImprovementSystem orchestrator
├── monitor.rs          # Phase 1: Metric collection
├── analyzer.rs         # Phase 2: LLM-based diagnosis
├── executor.rs         # Phase 3: Action execution
├── learner.rs          # Phase 4: Lesson extraction
├── circuit_breaker.rs  # Safety: halt on failures
├── allowlist.rs        # Safety: validate actions
├── types/              # Type definitions
├── storage/            # Database operations
├── anthropic_calls/    # LLM integration
└── cli/                # CLI commands
```

### Missing Integration Points

1. **No instantiation in server startup** (`mcp.rs`)
2. **No background task/trigger** for improvement cycles
3. **No MCP tools** for self-improvement interaction
4. **No configuration options** for tuning cycle behavior

## Proposed Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                         McpServer                                    │
│  ┌───────────────┐   ┌──────────────────────────────────────────┐  │
│  │  AppState     │   │        SelfImprovementManager            │  │
│  │  ├─storage    │   │  ┌────────────────────────────────────┐  │  │
│  │  ├─client     │◀──┼──│  SelfImprovementSystem             │  │  │
│  │  ├─config     │   │  │  ├─Monitor (metrics -> triggers)    │  │  │
│  │  ├─metrics ◀──┼───┼──│  ├─Analyzer (LLM diagnosis)        │  │  │
│  │  └─presets    │   │  │  ├─Executor (action execution)     │  │  │
│  └───────────────┘   │  │  └─Learner (reward calculation)    │  │  │
│         │            │  └────────────────────────────────────┘  │  │
│         │            │  ┌────────────────────────────────────┐  │  │
│         ▼            │  │  Background Task                   │  │  │
│  ┌───────────────┐   │  │  └─Periodic cycle trigger          │  │  │
│  │ ReasoningTools│   │  └────────────────────────────────────┘  │  │
│  │ (15 tools)    │   └──────────────────────────────────────────┘  │
│  └───────────────┘                      │                          │
│         │                               │                          │
│         ▼                               ▼                          │
│  ┌───────────────┐            ┌─────────────────┐                  │
│  │ MCP Protocol  │            │ SI MCP Tools    │                  │
│  │ Handler       │            │ (new tools)     │                  │
│  └───────────────┘            └─────────────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
```

### Integration Options

#### Option A: Background Task with Periodic Trigger (Recommended)

- Spawns a background Tokio task that runs improvement cycles periodically
- Non-blocking, doesn't interfere with MCP tool handling
- Configurable interval (default: every 100 invocations or 5 minutes)

#### Option B: Event-Driven on Invocation Threshold

- Triggers after N invocations or when metrics cross thresholds
- Lower latency for detecting issues
- Slightly more complex to implement

#### Option C: Manual Trigger via MCP Tool

- User explicitly triggers improvement cycles
- Simplest to implement
- Least autonomous

**Recommendation**: Implement Options A + C (background task + manual trigger)

## Detailed Design

### 1. Configuration Extensions

```rust
// src/config/mod.rs - Add to Config struct

/// Self-improvement system configuration.
///
/// NOTE: Self-improvement is ALWAYS enabled. It is a core feature, not optional.
pub struct SelfImprovementConfig {
    /// Require human approval before executing actions (default: true)
    pub require_approval: bool,
    /// Minimum invocations before analysis (default: 50)
    pub min_invocations_for_analysis: u64,
    /// Interval between automatic cycles in seconds (default: 300)
    pub cycle_interval_secs: u64,
    /// Maximum actions per cycle (default: 3)
    pub max_actions_per_cycle: u32,
    /// Circuit breaker failure threshold (default: 3)
    pub circuit_breaker_threshold: u32,
}

impl Default for SelfImprovementConfig {
    fn default() -> Self {
        Self {
            // No "enabled" flag - always on!
            require_approval: true,
            min_invocations_for_analysis: 50,
            cycle_interval_secs: 300,
            max_actions_per_cycle: 3,
            circuit_breaker_threshold: 3,
        }
    }
}
```

**Environment Variables:**
```bash
# Self-improvement is ALWAYS enabled - no toggle!
SELF_IMPROVEMENT_REQUIRE_APPROVAL=true|false
SELF_IMPROVEMENT_MIN_INVOCATIONS=50
SELF_IMPROVEMENT_CYCLE_INTERVAL_SECS=300
SELF_IMPROVEMENT_MAX_ACTIONS=3
SELF_IMPROVEMENT_CIRCUIT_BREAKER_THRESHOLD=3
```

### 2. SelfImprovementManager

New module: `src/self_improvement/manager.rs`

```rust
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub struct SelfImprovementManager {
    system: Arc<RwLock<SelfImprovementSystem>>,
    config: SelfImprovementConfig,
    metrics: Arc<MetricsCollector>,
    client: Arc<AnthropicClient>,
    storage: Arc<SelfImprovementStorage>,
    /// Channel for receiving approval/rejection
    approval_rx: mpsc::Receiver<ApprovalMessage>,
    /// Channel for sending pending diagnoses to UI
    diagnosis_tx: mpsc::Sender<PendingDiagnosis>,
    /// Shutdown signal
    shutdown: tokio::sync::watch::Receiver<bool>,
}

impl SelfImprovementManager {
    pub fn new(
        config: SelfImprovementConfig,
        metrics: Arc<MetricsCollector>,
        client: Arc<AnthropicClient>,
        storage: Arc<SelfImprovementStorage>,
    ) -> (Self, ManagerHandle) {
        // Returns the manager and a handle for MCP tools to interact
    }

    /// Run the background improvement loop
    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(
            Duration::from_secs(self.config.cycle_interval_secs)
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if self.should_run_cycle() {
                        self.run_cycle().await;
                    }
                }
                Some(approval) = self.approval_rx.recv() => {
                    self.handle_approval(approval).await;
                }
                _ = self.shutdown.changed() => {
                    break;
                }
            }
        }
    }

    fn should_run_cycle(&self) -> bool {
        let summary = self.metrics.summary();
        summary.total_invocations >= self.config.min_invocations_for_analysis
    }

    async fn run_cycle(&mut self) {
        let system = self.system.write().await;

        // Phase 1: Monitor - collect metrics and detect issues
        let triggers = system.monitor.check_triggers(&self.metrics.summary());

        if triggers.is_empty() {
            return; // Nothing to improve
        }

        // Phase 2: Analyze - diagnose issues
        let diagnoses = system.analyzer.analyze(triggers).await;

        for diagnosis in diagnoses {
            if self.config.require_approval {
                // Queue for approval
                self.storage.insert_diagnosis(&diagnosis).await;
                self.diagnosis_tx.send(diagnosis.into()).await;
            } else {
                // Auto-execute
                self.execute_diagnosis(diagnosis).await;
            }
        }
    }
}
```

### 3. AppState Extension

```rust
// src/server/types.rs

pub struct AppState {
    pub storage: Arc<SqliteStorage>,
    pub client: Arc<AnthropicClient>,
    pub config: Arc<Config>,
    pub metrics: Arc<MetricsCollector>,
    pub presets: Arc<PresetRegistry>,
    // Self-improvement handle (ALWAYS present - not optional)
    pub self_improvement: Arc<ManagerHandle>,
}
```

### 4. New MCP Tools

Add 4 new tools for self-improvement interaction:

```rust
// src/server/tools_si.rs

/// Tool: reasoning_si_status
/// Get current self-improvement system status
#[tool(description = "Get self-improvement system status including pending diagnoses, recent actions, and learning summary")]
pub async fn reasoning_si_status(&self) -> SiStatusResponse;

/// Tool: reasoning_si_diagnoses
/// List pending diagnoses awaiting approval
#[tool(description = "List pending diagnoses from the self-improvement system")]
pub async fn reasoning_si_diagnoses(&self, limit: Option<u32>) -> SiDiagnosesResponse;

/// Tool: reasoning_si_approve
/// Approve a pending diagnosis for execution
#[tool(description = "Approve a pending diagnosis for execution")]
pub async fn reasoning_si_approve(&self, diagnosis_id: String) -> SiApproveResponse;

/// Tool: reasoning_si_reject
/// Reject a pending diagnosis
#[tool(description = "Reject a pending diagnosis with optional reason")]
pub async fn reasoning_si_reject(&self, diagnosis_id: String, reason: Option<String>) -> SiRejectResponse;

/// Tool: reasoning_si_trigger
/// Manually trigger an improvement cycle
#[tool(description = "Manually trigger a self-improvement analysis cycle")]
pub async fn reasoning_si_trigger(&self) -> SiTriggerResponse;

/// Tool: reasoning_si_rollback
/// Rollback a previously executed action
#[tool(description = "Rollback a previously executed self-improvement action")]
pub async fn reasoning_si_rollback(&self, action_id: String) -> SiRollbackResponse;
```

### 5. Server Startup Integration

```rust
// src/server/mcp.rs

pub async fn run_stdio(&self) -> Result<(), AppError> {
    // ... existing initialization ...

    // Initialize self-improvement system (ALWAYS enabled - core feature)
    let si_storage = SelfImprovementStorage::new(pool.clone());

    let (manager, handle) = SelfImprovementManager::new(
        self.config.self_improvement.clone(),
        state.metrics.clone(),
        state.client.clone(),
        Arc::new(si_storage),
    );

    // Spawn background task - always runs
    let shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        tracing::info!("Self-improvement system started");
        manager.run(shutdown_rx).await;
        tracing::info!("Self-improvement system stopped");
    });

    // Add to AppState
    let state = AppState {
        // ... existing fields ...
        self_improvement: Arc::new(handle),  // Not Option - always present
    };

    // ... rest of server startup ...
}
```

## Implementation Plan

### Phase 1: Configuration & Foundation (2-3 hours)

1. **Extend Config** - Add `SelfImprovementConfig` struct
2. **Add environment variable parsing** - Parse SI-related env vars
3. **Create `ManagerHandle`** - Define the interface between MCP tools and manager

Files to modify:
- `src/config/mod.rs` - Add config struct
- `src/config/validation.rs` - Add validation

### Phase 2: Manager Implementation (3-4 hours)

1. **Create `SelfImprovementManager`** - Main orchestration struct
2. **Implement background task** - Tokio task with interval trigger
3. **Add approval channel** - mpsc channel for approval flow

Files to create:
- `src/self_improvement/manager.rs` - New file

### Phase 3: Server Integration (2-3 hours)

1. **Modify AppState** - Add optional SI handle
2. **Update McpServer** - Initialize and spawn manager
3. **Add graceful shutdown** - Ensure manager shuts down cleanly

Files to modify:
- `src/server/types.rs` - Extend AppState
- `src/server/mcp.rs` - Add initialization

### Phase 4: MCP Tools (3-4 hours)

1. **Define tool schemas** - Add rmcp macro definitions
2. **Implement handlers** - Connect to manager handle
3. **Add to tool registry** - Register new tools

Files to create:
- `src/server/tools_si.rs` - New file
- `src/server/params_si.rs` - Parameter types
- `src/server/responses_si.rs` - Response types

### Phase 5: Testing & Documentation (2-3 hours)

1. **Integration tests** - Test full cycle with mock metrics
2. **Update README** - Document new configuration
3. **Add usage examples** - Show how to enable and use

Files to modify:
- `tests/integration_si.rs` - New integration tests
- `README.md` - Documentation updates
- `docs/DESIGN.md` - Architecture updates

## Risk Mitigation

### Safety Guardrails (Already Implemented)

1. **Circuit Breaker** - Halts after N consecutive failures
2. **Allowlist** - Only allows approved action types
3. **Approval Gate** - Human approval required by default
4. **Rollback Capability** - All actions can be rolled back
5. **Rate Limiting** - Max N actions per cycle

### Additional Safety Measures

1. **Always On** - Self-improvement is a core feature, not optional
2. **Require Approval Default** - `require_approval: true` is the default (can be disabled)
3. **Minimum Data Threshold** - Won't analyze until 50+ invocations (configurable)
4. **Logging** - All actions logged to database for audit

## Success Metrics

After integration, validate that:

1. Server starts and self-improvement background task runs automatically
2. Metrics are collected from reasoning tool invocations
3. Diagnoses are generated when metrics cross thresholds
4. Approval flow works via MCP tools (when require_approval=true)
5. Actions auto-execute when require_approval=false
6. Actions are recorded in database with pre/post metrics
7. Rollback works correctly
8. Circuit breaker trips on consecutive failures
9. Learnings are extracted and stored
10. Graceful shutdown completes cleanly

## File Summary

### New Files
- `src/self_improvement/manager.rs` - Manager struct and background task
- `src/server/tools_si.rs` - MCP tool implementations
- `src/server/params_si.rs` - Tool parameter types
- `src/server/responses_si.rs` - Tool response types
- `tests/integration_si.rs` - Integration tests

### Modified Files
- `src/config/mod.rs` - Add SelfImprovementConfig
- `src/config/validation.rs` - Add SI config validation
- `src/server/types.rs` - Extend AppState
- `src/server/mcp.rs` - Add SI initialization
- `src/server/mod.rs` - Export new modules
- `src/lib.rs` - Export SI types
- `README.md` - Documentation
- `docs/DESIGN.md` - Architecture updates
