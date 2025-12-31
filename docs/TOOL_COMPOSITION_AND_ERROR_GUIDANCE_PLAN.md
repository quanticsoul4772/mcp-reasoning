# Tool Composition Guidance & Enhanced Error Messages - Implementation Plan

**Status**: Ready for Implementation
**Created**: 2025-12-31
**Updated**: 2025-12-31 (Addressed review gaps)
**Priority**: Medium (Post-metadata enrichment improvements)

---

## Executive Summary

This plan implements two complementary features to improve AI agent experience:

1. **Priority 2: Tool Composition Guidance** - Help agents discover optimal tool sequences through metrics analysis and workflow patterns
2. **Priority 3: Better Error Messages** - Provide actionable alternatives when operations fail, especially for timeouts and API errors

Both features leverage existing metadata enrichment infrastructure and are designed to be additive (no breaking changes).

---

## Current State Analysis

### What We Have

#### Metadata Enrichment (Completed)
- `TimingDatabase` tracks execution history
- `SuggestionEngine` provides rule-based next-tool recommendations
- `PresetIndex` with 5 workflow presets
- `MetadataBuilder` enriches all tool responses

#### Metrics Collection (Basic)
- `MetricsCollector` tracks:
  - Per-mode invocations (count)
  - Success/failure rates
  - Latency (avg/min/max)
  - Fallback events
- **Missing**: Tool chain patterns, transition frequencies, success correlations

#### Error Handling (Minimal)
- Structured error types (thiserror)
- Basic error messages: `"Request timeout after 30000ms"`
- **Missing**: Contextual alternatives, recovery suggestions, agent-friendly guidance

#### Tool Suggestions (Static Rules)
- Hard-coded next-tool logic per tool
- Based on result context (complexity, branches, outputs)
- **Missing**: Dynamic learning from actual usage patterns

### What We Need

#### Tool Composition Guidance
1. **Chain Discovery**: Identify common tool sequences from metrics
2. **Transition Matrix**: Track which tools follow which (with success rates)
3. **Dynamic Suggestions**: Use historical data to improve recommendations
4. **Chain Visualization**: Show common paths in reasoning_metrics output

#### Enhanced Error Messages
1. **Alternative Suggestions**: Offer faster/alternative tools on failure
2. **Recovery Strategies**: Break down complex operations into smaller steps
3. **Context-Aware**: Use timing metadata to suggest appropriate timeout tiers
4. **Structured Format**: Machine-readable alternatives for agent parsing

---

## Priority 2: Tool Composition Guidance

### Design

#### 2.1: Chain Tracking in MetricsCollector

**New Data Structure:**
```rust
/// A tool transition event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTransition {
    pub from_tool: String,
    pub to_tool: String,
    pub session_id: String,
    pub success: bool,
    pub timestamp: u64,
}

/// Summary of tool chain patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSummary {
    /// Most common tool sequences (min 3 tools, min 5 occurrences)
    pub common_chains: Vec<ToolChain>,
    /// Transition matrix: tool A -> tool B (frequency %)
    pub transitions: HashMap<String, HashMap<String, TransitionStats>>,
    /// Tools that are frequently starting points
    pub entry_tools: Vec<String>,
    /// Tools that are frequently ending points
    pub terminal_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChain {
    pub tools: Vec<String>,
    pub occurrences: u32,
    pub avg_success_rate: f64,
    pub avg_total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionStats {
    pub count: u32,
    pub success_rate: f64,
    pub avg_time_between_ms: u64,
}
```

**Changes to MetricsCollector:**
```rust
impl MetricsCollector {
    // NEW: Record tool transitions
    pub fn record_transition(&self, transition: ToolTransition);
    
    // NEW: Get chain analysis
    pub fn chain_summary(&self) -> ChainSummary;
    
    // NEW: Get transitions for a specific tool
    pub fn transitions_from(&self, tool: &str) -> HashMap<String, TransitionStats>;
}
```

#### 2.2: Complete Static Tool Suggestions

**Gap Identified**: Current `SuggestionEngine` only covers 8 of 15 tools. Add handlers for missing tools:

```rust
// Add to src/metadata/suggestions.rs

impl SuggestionEngine {
    pub fn suggest_next_tools(
        &self,
        current_tool: &str,
        result_context: &ResultContext,
    ) -> Vec<ToolSuggestion> {
        match current_tool {
            // Existing handlers (8 tools)
            "reasoning_divergent" => self.suggest_after_divergent(result_context),
            "reasoning_tree" => self.suggest_after_tree(result_context),
            "reasoning_linear" => self.suggest_after_linear(result_context),
            "reasoning_decision" => self.suggest_after_decision(result_context),
            "reasoning_graph" => self.suggest_after_graph(result_context),
            "reasoning_reflection" => self.suggest_after_reflection(result_context),
            "reasoning_mcts" => self.suggest_after_mcts(result_context),
            "reasoning_evidence" => self.suggest_after_evidence(result_context),

            // NEW: Missing handlers (7 tools)
            "reasoning_auto" => self.suggest_after_auto(result_context),
            "reasoning_detect" => self.suggest_after_detect(result_context),
            "reasoning_timeline" => self.suggest_after_timeline(result_context),
            "reasoning_counterfactual" => self.suggest_after_counterfactual(result_context),
            "reasoning_checkpoint" => self.suggest_after_checkpoint(result_context),
            "reasoning_preset" => self.suggest_after_preset(result_context),
            "reasoning_metrics" => vec![], // Terminal tool, no suggestions
            _ => vec![],
        }
    }

    // NEW: Add these handler methods

    fn suggest_after_auto(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save auto-selected analysis results".into(),
                estimated_duration_ms: 100,
            },
        ]
    }

    fn suggest_after_detect(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        let mut suggestions = vec![];
        if ctx.num_outputs > 0 {
            suggestions.push(ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Reflect on detected biases/fallacies".into(),
                estimated_duration_ms: 25_000,
            });
        }
        suggestions.push(ToolSuggestion {
            tool: "reasoning_linear".into(),
            reason: "Re-analyze with detected issues in mind".into(),
            estimated_duration_ms: 12_000,
        });
        suggestions
    }

    fn suggest_after_timeline(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: "Compare timeline branches for decision".into(),
                estimated_duration_ms: 18_000,
            },
            ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save timeline state".into(),
                estimated_duration_ms: 100,
            },
        ]
    }

    fn suggest_after_counterfactual(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        vec![
            ToolSuggestion {
                tool: "reasoning_decision".into(),
                reason: "Use causal insights for decision-making".into(),
                estimated_duration_ms: 18_000,
            },
            ToolSuggestion {
                tool: "reasoning_evidence".into(),
                reason: "Evaluate evidence for causal claims".into(),
                estimated_duration_ms: 20_000,
            },
        ]
    }

    fn suggest_after_checkpoint(&self, _ctx: &ResultContext) -> Vec<ToolSuggestion> {
        // Checkpoint is typically terminal, but can continue
        vec![
            ToolSuggestion {
                tool: "reasoning_linear".into(),
                reason: "Continue analysis from saved state".into(),
                estimated_duration_ms: 12_000,
            },
        ]
    }

    fn suggest_after_preset(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
        // Preset execution may suggest continuation
        if ctx.complexity == "complex" {
            vec![ToolSuggestion {
                tool: "reasoning_reflection".into(),
                reason: "Review preset workflow results".into(),
                estimated_duration_ms: 25_000,
            }]
        } else {
            vec![ToolSuggestion {
                tool: "reasoning_checkpoint".into(),
                reason: "Save preset execution results".into(),
                estimated_duration_ms: 100,
            }]
        }
    }
}
```

#### 2.3: Dynamic Tool Suggestions

**Enhance SuggestionEngine with historical metrics:**
```rust
impl SuggestionEngine {
    // NEW: Constructor with metrics
    pub fn with_metrics(
        preset_index: Arc<PresetIndex>,
        metrics: Arc<MetricsCollector>,
    ) -> Self;
    
    // ENHANCED: Use historical data
    pub fn suggest_next_tools(
        &self,
        current_tool: &str,
        result_context: &ResultContext,
    ) -> Vec<ToolSuggestion> {
        // 1. Get static rule-based suggestions (existing)
        let mut suggestions = self.suggest_static(current_tool, result_context);
        
        // 2. Get historical transitions (NEW)
        if let Some(metrics) = &self.metrics {
            let historical = metrics.transitions_from(current_tool);
            
            // Add high-frequency transitions not in static rules
            for (to_tool, stats) in historical {
                if stats.count >= 5 && stats.success_rate > 0.7 {
                    if !suggestions.iter().any(|s| s.tool == to_tool) {
                        suggestions.push(ToolSuggestion {
                            tool: to_tool,
                            reason: format!(
                                "Historically used {}% of the time (success rate: {:.0}%)",
                                (stats.count as f64 / metrics.total_invocations() as f64) * 100.0,
                                stats.success_rate * 100.0
                            ),
                            estimated_duration_ms: stats.avg_time_between_ms,
                        });
                    }
                }
            }
        }
        
        suggestions
    }
}
```

#### 2.3: reasoning_metrics Enhancement

**Add New Query Types:**
```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MetricsRequest {
    pub query: String, // existing: summary, by_mode, invocations, fallbacks, config
                       // NEW: chains, transitions, entry_points, terminal_points
    // ... existing fields ...
}
```

**Response Changes:**
```rust
#[derive(Debug, Serialize, JsonSchema)]
pub struct MetricsResponse {
    // ... existing fields ...
    
    // NEW: Tool chain data (if query == "chains")
    pub common_chains: Option<Vec<ToolChain>>,
    pub transitions: Option<HashMap<String, HashMap<String, TransitionStats>>>,
    pub entry_tools: Option<Vec<String>>,
    pub terminal_tools: Option<Vec<String>>,
}
```

**Implementation:**
```rust
async fn reasoning_metrics(&self, req: Parameters<MetricsRequest>) -> MetricsResponse {
    match req.0.query.as_str() {
        "summary" => { /* existing */ },
        "by_mode" => { /* existing */ },
        "invocations" => { /* existing */ },
        "fallbacks" => { /* existing */ },
        "config" => { /* existing */ },
        
        // NEW
        "chains" => {
            let chain_summary = self.state.metrics.chain_summary();
            MetricsResponse {
                common_chains: Some(chain_summary.common_chains),
                transitions: Some(chain_summary.transitions),
                entry_tools: Some(chain_summary.entry_tools),
                terminal_tools: Some(chain_summary.terminal_tools),
                ..Default::default()
            }
        },
        
        // NEW
        "transitions" => {
            let tool = req.0.tool_name.as_deref().unwrap_or("");
            let transitions = self.state.metrics.transitions_from(tool);
            MetricsResponse {
                transitions: Some(hashmap! { tool.to_string() => transitions }),
                ..Default::default()
            }
        },
        
        _ => MetricsResponse::default(),
    }
}
```

### Implementation Roadmap

#### Phase 1: Chain Tracking
1. Add `ToolTransition` struct to metrics/mod.rs
2. Add `transitions` field to `MetricsCollector`
3. Implement `record_transition()` method
4. Track transitions in server request handler (see Section 2.5 below)
5. Add unit tests for transition recording

#### Phase 2: Chain Analysis
1. Implement `ChainSummary` analysis logic
2. Create `chain_summary()` method with:
   - Sliding window pattern detection (3-5 tool sequences)
   - Transition matrix calculation
   - Entry/terminal tool identification
3. Add unit tests for chain detection

#### Phase 3: Dynamic Suggestions
1. Update `SuggestionEngine` constructor to accept metrics
2. Enhance `suggest_next_tools()` with historical data
3. Update `MetadataBuilder` to pass metrics to engine
4. Add integration tests

#### Phase 4: Metrics Query Enhancement
1. Add "chains" and "transitions" query types to MetricsRequest
2. Update MetricsResponse with new fields
3. Implement query handlers in reasoning_metrics
4. Update TOOL_REFERENCE.md documentation

#### 2.5: Transition Recording Hook Location

**Where to record transitions**: In the MCP server's tool call dispatcher (typically `src/server/tools.rs` or `src/server/handlers.rs`).

```rust
// In src/server/tools.rs - tool call dispatcher

/// Session context tracking for transition recording.
struct SessionContext {
    session_id: String,
    last_tool: Option<String>,
}

impl McpServer {
    async fn handle_tool_call(
        &self,
        tool_name: &str,
        params: Value,
        session_ctx: &mut SessionContext,
    ) -> Result<Value, McpError> {
        // BEFORE executing the tool: record transition from previous tool
        if let Some(ref last_tool) = session_ctx.last_tool {
            self.state.metrics.record_transition(ToolTransition {
                from_tool: last_tool.clone(),
                to_tool: tool_name.to_string(),
                session_id: session_ctx.session_id.clone(),
                success: true,  // Optimistic; updated on failure
                timestamp: now_millis(),
            });
        }

        // Execute the tool
        let result = self.dispatch_tool(tool_name, params).await;

        // Update transition success based on result
        if result.is_err() {
            // Update last transition to mark as failed
            self.state.metrics.mark_last_transition_failed(&session_ctx.session_id);
        }

        // Track this tool as last_tool for next transition
        session_ctx.last_tool = Some(tool_name.to_string());

        result
    }
}

// Helper function for timestamps
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
```

**Key Integration Points**:
1. **Session ID extraction**: Get from MCP request context or create per-connection
2. **Last tool tracking**: Maintain per-session state (in-memory HashMap or session struct)
3. **Transition recording**: Call `record_transition()` BEFORE executing each tool
4. **Failure handling**: Update transition success status if tool execution fails

---

## Priority 3: Better Error Messages

### Design

#### 3.1: Error Context Structure

**New Types (add to `src/error/enhanced.rs`):**

```rust
/// Metrics about request complexity for error context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Length of content in bytes.
    pub content_length: usize,
    /// Depth of operation (e.g., graph depth, iteration count).
    pub operation_depth: Option<u32>,
    /// Branching factor (e.g., num_perspectives, num_branches).
    pub branching_factor: Option<u32>,
}
```

```rust
/// Enhanced error with recovery suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedError {
    /// Original error message
    pub error: String,
    /// Error category for machine parsing
    pub category: ErrorCategory,
    /// Suggested alternatives
    pub alternatives: Vec<Alternative>,
    /// Context that helps with recovery
    pub context: Option<ErrorContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCategory {
    Timeout,
    RateLimit,
    Authentication,
    InvalidRequest,
    ApiUnavailable,
    Storage,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    /// Alternative tool or approach
    pub suggestion: String,
    /// Why this might work better
    pub reason: String,
    /// Estimated duration if applicable
    pub estimated_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Tool that failed
    pub failed_tool: String,
    /// Operation that failed
    pub failed_operation: Option<String>,
    /// Request complexity metrics
    pub complexity: ComplexityMetrics,
    /// Timeout used
    pub timeout_ms: u64,
}
```

#### 3.2: ServerState Integration

**Update `src/server/mod.rs` or `src/server/types.rs`:**

The `ErrorEnhancer` must be integrated into the server's shared state so all tool handlers can access it.

```rust
use crate::error::enhanced::ErrorEnhancer;

/// Shared server state accessible by all tool handlers.
pub struct ServerState {
    pub storage: Arc<SqliteStorage>,
    pub client: Arc<AnthropicClient>,
    pub config: Config,
    pub metrics: Arc<MetricsCollector>,
    pub error_enhancer: Arc<ErrorEnhancer>,  // NEW: Add this field
}

impl ServerState {
    pub fn new(
        storage: Arc<SqliteStorage>,
        client: Arc<AnthropicClient>,
        config: Config,
    ) -> Self {
        let metrics = Arc::new(MetricsCollector::new());
        let metadata_builder = Arc::new(MetadataBuilder::new(/* ... */));

        // Create ErrorEnhancer with dependencies
        let error_enhancer = Arc::new(ErrorEnhancer::new(
            Arc::clone(&metadata_builder),
            Arc::clone(&metrics),
        ));

        Self {
            storage,
            client,
            config,
            metrics,
            error_enhancer,
        }
    }
}
```

#### 3.3: Error Enhancement Logic

**Create `src/error/enhanced.rs`:**
```rust
/// Enhance errors with contextual alternatives.
pub struct ErrorEnhancer {
    metadata_builder: Arc<MetadataBuilder>,
    metrics: Arc<MetricsCollector>,
}

impl ErrorEnhancer {
    /// Create a new error enhancer with dependencies.
    pub fn new(
        metadata_builder: Arc<MetadataBuilder>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            metadata_builder,
            metrics,
        }
    }
}

impl ErrorEnhancer {
    pub fn enhance(
        &self,
        error: &AppError,
        context: ErrorContext,
    ) -> EnhancedError {
        let category = self.categorize_error(error);
        let alternatives = self.generate_alternatives(&category, &context);
        
        EnhancedError {
            error: error.to_string(),
            category,
            alternatives,
            context: Some(context),
        }
    }
    
    fn categorize_error(&self, error: &AppError) -> ErrorCategory {
        match error {
            AppError::Anthropic(ae) => match ae {
                AnthropicError::Timeout { .. } => ErrorCategory::Timeout,
                AnthropicError::RateLimited { .. } => ErrorCategory::RateLimit,
                AnthropicError::AuthenticationFailed => ErrorCategory::Authentication,
                AnthropicError::InvalidRequest { .. } => ErrorCategory::InvalidRequest,
                _ => ErrorCategory::ApiUnavailable,
            },
            AppError::Storage(_) => ErrorCategory::Storage,
            _ => ErrorCategory::Other,
        }
    }
    
    fn generate_alternatives(
        &self,
        category: &ErrorCategory,
        context: &ErrorContext,
    ) -> Vec<Alternative> {
        match category {
            ErrorCategory::Timeout => self.timeout_alternatives(context),
            ErrorCategory::RateLimit => self.rate_limit_alternatives(context),
            ErrorCategory::ApiUnavailable => self.unavailable_alternatives(context),
            _ => vec![],
        }
    }
    
    fn timeout_alternatives(&self, ctx: &ErrorContext) -> Vec<Alternative> {
        let mut alts = vec![];
        
        // Suggest faster tool if available
        if ctx.failed_tool == "reasoning_divergent" {
            alts.push(Alternative {
                suggestion: "Use reasoning_linear instead".into(),
                reason: "Completes in ~12s vs 45s for divergent mode".into(),
                estimated_duration_ms: Some(12_000),
            });
        }
        
        if ctx.failed_tool == "reasoning_graph" {
            alts.push(Alternative {
                suggestion: "Use reasoning_tree with 2-3 branches".into(),
                reason: "Similar exploration but faster (18s vs 60s)".into(),
                estimated_duration_ms: Some(18_000),
            });
        }
        
        // Suggest breaking down if complex
        if ctx.complexity.content_length > 10_000 {
            alts.push(Alternative {
                suggestion: "Break content into 2-3 smaller reasoning_linear calls".into(),
                reason: format!(
                    "Content length {} is high. Splitting may help.",
                    ctx.complexity.content_length
                ),
                estimated_duration_ms: Some(8_000 * 3),
            });
        }
        
        // Suggest mode auto-selection
        alts.push(Alternative {
            suggestion: "Use reasoning_auto to select faster mode".into(),
            reason: "Automatically routes to optimal mode for complexity".into(),
            estimated_duration_ms: Some(15_000),
        });
        
        // Suggest using appropriate timeout tier
        if ctx.timeout_ms < 60_000 {
            alts.push(Alternative {
                suggestion: "Request longer timeout from Factory client".into(),
                reason: format!(
                    "Current timeout ({}ms) may be too short. Try 60s or 120s tier.",
                    ctx.timeout_ms
                ),
                estimated_duration_ms: None,
            });
        }
        
        alts
    }
    
    fn rate_limit_alternatives(&self, ctx: &ErrorContext) -> Vec<Alternative> {
        vec![
            Alternative {
                suggestion: "Retry after rate limit expires".into(),
                reason: "Wait for rate limit window to reset".into(),
                estimated_duration_ms: None,
            },
            Alternative {
                suggestion: "Use reasoning_checkpoint to save progress".into(),
                reason: "Save current state before retrying".into(),
                estimated_duration_ms: Some(100),
            },
        ]
    }
    
    fn unavailable_alternatives(&self, ctx: &ErrorContext) -> Vec<Alternative> {
        // Check if we have cached/historical data
        let has_history = self.metrics
            .invocations_by_mode(&ctx.failed_tool)
            .len() > 0;
        
        let mut alts = vec![
            Alternative {
                suggestion: "Retry with exponential backoff".into(),
                reason: "API may be temporarily unavailable".into(),
                estimated_duration_ms: None,
            },
        ];
        
        if has_history {
            alts.push(Alternative {
                suggestion: "Check reasoning_metrics for past successful patterns".into(),
                reason: "Review historical data for working alternatives".into(),
                estimated_duration_ms: None,
            });
        }
        
        alts
    }
}
```

#### 3.3: Integration with Tool Handlers

**Update tool handler pattern:**
```rust
async fn reasoning_divergent(&self, req: Parameters<DivergentRequest>) -> DivergentResponse {
    let req = req.0;
    let timer = Timer::start();
    
    // Build error context BEFORE operation
    let error_context = ErrorContext {
        failed_tool: "reasoning_divergent".into(),
        failed_operation: None,
        complexity: ComplexityMetrics {
            content_length: req.content.len(),
            operation_depth: req.num_perspectives.map(|n| n as u32),
            branching_factor: req.num_perspectives.map(|n| n as u32),
        },
        timeout_ms: self.state.config.request_timeout_ms,
    };
    
    let mode = DivergentMode::new(
        Arc::clone(&self.state.storage),
        Arc::clone(&self.state.client),
    );
    
    let result = mode
        .generate(&req.content, /* ... */)
        .await;
    
    match result {
        Ok(response) => {
            // Success path with metadata
            self.build_success_response(response, timer.elapsed_ms()).await
        }
        Err(e) => {
            // ENHANCED: Return error with alternatives
            let enhanced = self.state.error_enhancer.enhance(&e, error_context);
            DivergentResponse {
                error: Some(enhanced.error),
                alternatives: Some(enhanced.alternatives),
                metadata: None,
                ..Default::default()
            }
        }
    }
}
```

**Update Response Types:**
```rust
// Add to ALL response types
#[derive(Debug, Serialize, JsonSchema)]
pub struct DivergentResponse {
    // ... existing fields ...
    
    /// Error message if operation failed
    pub error: Option<String>,
    
    /// Suggested alternatives on failure
    pub alternatives: Option<Vec<Alternative>>,
}
```

### Example Output

**Before (current):**
```json
{
  "error": "Anthropic API error: Request timeout after 30000ms"
}
```

**After (enhanced):**
```json
{
  "error": "Request timeout (30s limit)",
  "alternatives": [
    {
      "suggestion": "Use reasoning_linear instead",
      "reason": "Completes in ~12s vs 45s for divergent mode",
      "estimated_duration_ms": 12000
    },
    {
      "suggestion": "Break content into 2-3 smaller reasoning_linear calls",
      "reason": "Content length 15000 is high. Splitting may help.",
      "estimated_duration_ms": 24000
    },
    {
      "suggestion": "Use reasoning_auto to select faster mode",
      "reason": "Automatically routes to optimal mode for complexity",
      "estimated_duration_ms": 15000
    }
  ]
}
```

### Implementation Roadmap

#### Phase 1: Error Context & Enhancement
1. Create error/enhanced.rs with types
2. Implement ErrorEnhancer struct
3. Add categorize_error() method
4. Implement timeout_alternatives()
5. Add unit tests for each alternative generator

#### Phase 2: Response Type Updates
1. Add `error` and `alternatives` fields to all 15 response types
2. Update response constructors to handle errors
3. Ensure backward compatibility (fields are optional)

#### Phase 3: Tool Handler Integration
1. Update each tool handler to build ErrorContext
2. Integrate ErrorEnhancer calls on failures
3. Return enhanced errors in responses
4. Add integration tests for error scenarios

#### Phase 4: Documentation
1. Update TOOL_REFERENCE.md with error response examples
2. Add error handling guide to README
3. Document ErrorCategory enum values

---

## Testing Strategy

### Tool Composition Tests

#### Unit Tests
```rust
#[test]
fn test_chain_detection() {
    let collector = MetricsCollector::new();
    
    // Record a chain: linear -> divergent -> decision
    collector.record_transition(ToolTransition {
        from_tool: "reasoning_linear".into(),
        to_tool: "reasoning_divergent".into(),
        session_id: "s1".into(),
        success: true,
        timestamp: 1000,
    });
    collector.record_transition(ToolTransition {
        from_tool: "reasoning_divergent".into(),
        to_tool: "reasoning_decision".into(),
        session_id: "s1".into(),
        success: true,
        timestamp: 2000,
    });
    
    // Repeat chain 5+ times for detection
    // ...
    
    let summary = collector.chain_summary();
    assert!(summary.common_chains.iter().any(|c| 
        c.tools == vec!["reasoning_linear", "reasoning_divergent", "reasoning_decision"]
    ));
}

#[test]
fn test_dynamic_suggestions() {
    let metrics = Arc::new(MetricsCollector::new());
    // Record many transitions from linear -> evidence
    // ...
    
    let engine = SuggestionEngine::with_metrics(
        Arc::new(PresetIndex::build()),
        metrics,
    );
    
    let suggestions = engine.suggest_next_tools(
        "reasoning_linear",
        &ResultContext::default(),
    );
    
    assert!(suggestions.iter().any(|s| s.tool == "reasoning_evidence"));
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_reasoning_metrics_chains() {
    let server = create_test_server().await;
    
    // Execute a tool chain
    server.reasoning_linear(/* ... */).await;
    server.reasoning_divergent(/* ... */).await;
    server.reasoning_decision(/* ... */).await;
    
    // Query chains
    let response = server.reasoning_metrics(Parameters(MetricsRequest {
        query: "chains".into(),
        ..Default::default()
    })).await;
    
    assert!(response.common_chains.is_some());
}
```

### Error Enhancement Tests

#### Unit Tests
```rust
#[test]
fn test_timeout_alternatives() {
    let enhancer = create_test_enhancer();
    let context = ErrorContext {
        failed_tool: "reasoning_divergent".into(),
        failed_operation: None,
        complexity: ComplexityMetrics {
            content_length: 15_000,
            ..Default::default()
        },
        timeout_ms: 30_000,
    };
    
    let error = AppError::Anthropic(AnthropicError::Timeout {
        timeout_ms: 30_000,
    });
    
    let enhanced = enhancer.enhance(&error, context);
    
    assert_eq!(enhanced.category, ErrorCategory::Timeout);
    assert!(enhanced.alternatives.len() >= 3);
    assert!(enhanced.alternatives.iter().any(|a| 
        a.suggestion.contains("reasoning_linear")
    ));
}

#[test]
fn test_complex_content_alternatives() {
    let enhancer = create_test_enhancer();
    let context = ErrorContext {
        failed_tool: "reasoning_graph".into(),
        failed_operation: Some("init".into()),
        complexity: ComplexityMetrics {
            content_length: 20_000,
            operation_depth: Some(8),
            ..Default::default()
        },
        timeout_ms: 30_000,
    };
    
    let error = AppError::Anthropic(AnthropicError::Timeout {
        timeout_ms: 30_000,
    });
    
    let enhanced = enhancer.enhance(&error, context);
    
    // Should suggest breaking down
    assert!(enhanced.alternatives.iter().any(|a| 
        a.suggestion.contains("smaller") || a.suggestion.contains("Break")
    ));
}
```

#### Integration Tests
```rust
#[tokio::test]
async fn test_tool_failure_with_alternatives() {
    let server = create_test_server_with_failing_client().await;
    
    let response = server.reasoning_divergent(Parameters(DivergentRequest {
        content: "x".repeat(20_000), // Force timeout
        num_perspectives: Some(5),
        ..Default::default()
    })).await;
    
    assert!(response.error.is_some());
    assert!(response.alternatives.is_some());
    let alts = response.alternatives.unwrap();
    assert!(alts.len() >= 2);
}
```

---

## Database Schema Changes

None required. All new data structures use in-memory storage in MetricsCollector (existing pattern).

If persistence is desired later, add:
```sql
-- migrations/004_tool_transitions.sql
CREATE TABLE IF NOT EXISTS tool_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_tool TEXT NOT NULL,
    to_tool TEXT NOT NULL,
    session_id TEXT NOT NULL,
    success INTEGER NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE INDEX idx_transitions_from ON tool_transitions(from_tool);
CREATE INDEX idx_transitions_session ON tool_transitions(session_id);
```

---

## API Changes

### Breaking Changes
None. All new features are additive.

### New Fields (Backward Compatible)
All response types gain optional fields:
- `error: Option<String>`
- `alternatives: Option<Vec<Alternative>>`

### New Query Types
reasoning_metrics gains:
- `query: "chains"` - Returns common tool chains
- `query: "transitions"` - Returns transition matrix

---

## Performance Considerations

### Chain Detection
- **Memory**: O(N) where N = number of transitions (typically < 10K)
- **CPU**: Pattern matching with sliding window (O(N * W) where W = max chain length)
- **Mitigation**: Run chain analysis lazily on metrics query, not per-request

### Error Enhancement
- **Overhead**: ~1-2ms per failed request (negligible)
- **CPU**: Simple rule-based logic, no LLM calls
- **Memory**: Small structs (~1KB per enhanced error)

### Transition Tracking
- **Overhead**: ~50 bytes per transition event
- **CPU**: HashMap insert (O(1))
- **Mitigation**: Implement circular buffer with max 10K transitions

---

## Documentation Updates

### Files to Update
1. **README.md**: Add error handling section, tool composition examples
2. **TOOL_REFERENCE.md**: Document error response format, alternatives field
3. **docs/DESIGN.md**: Add ErrorEnhancer architecture
4. **New**: docs/TOOL_COMPOSITION_GUIDE.md - Best practices for tool chains
5. **New**: docs/ERROR_HANDLING_GUIDE.md - How agents should handle errors

---

## Rollout Plan

### Phase 1: Tool Composition
1. Implement chain tracking
2. Add chain analysis
3. Update metrics queries
4. Testing + documentation

### Phase 2: Error Enhancement
1. Implement ErrorEnhancer
2. Update response types
3. Integrate with tool handlers
4. Testing + documentation

**Sequencing**: Phase 1 should complete before Phase 2. Both features share `MetricsCollector` dependencies.

---

## Success Metrics

### Tool Composition
- **Chain Coverage**: 80%+ of multi-tool sessions captured in common_chains
- **Suggestion Quality**: Dynamic suggestions used 30%+ of the time
- **Documentation**: 5+ example chains documented for common workflows

### Error Enhancement
- **Alternative Usage**: 40%+ of timeout errors result in trying suggested alternative
- **Recovery Rate**: 60%+ of errors with alternatives lead to eventual success
- **Agent Satisfaction**: Measured via feedback in retry attempts

---

## Future Enhancements

### Beyond This Plan
1. **LLM-Based Alternatives**: Use Claude to generate custom alternatives based on error context
2. **Persistent Chain Storage**: Move transition data to SQLite for cross-session learning
3. **Chain Prediction**: ML model to predict optimal next tool based on content features
4. **Error Pattern Learning**: Identify recurring error patterns and proactive warnings
5. **Visual Chain Explorer**: Web UI for exploring tool composition patterns

---

## Risks & Mitigations

### Risk: Chain Detection False Positives
**Mitigation**: Require minimum 5 occurrences and 70% success rate for chain inclusion

### Risk: Alternative Overload
**Mitigation**: Limit to 5 alternatives per error, ranked by relevance

### Risk: Performance Impact
**Mitigation**: Lazy evaluation, circular buffers, async metrics collection

### Risk: Breaking Changes
**Mitigation**: All new fields optional, feature flags for gradual rollout

---

## Appendix A: Example Workflows

### Workflow: Architecture Decision (with composition)
```
1. reasoning_linear "Analyze microservices vs monolith"
   -> Metadata suggests: reasoning_divergent, reasoning_decision
   
2. reasoning_divergent "Explore perspectives" (4 perspectives)
   -> Metadata suggests: reasoning_decision, reasoning_detect
   
3. reasoning_decision "Compare options" (TOPSIS)
   -> Metadata suggests: reasoning_checkpoint, reasoning_reflection
   
4. reasoning_checkpoint "Save decision"
   -> Chain complete

Metrics show: This chain used 73 times, 89% success rate
```

### Workflow: Timeout Recovery
```
1. reasoning_graph init (fails with timeout after 30s)
   -> Error alternatives:
      - "Use reasoning_tree with 2-3 branches" (18s)
      - "Break into smaller reasoning_linear calls" (12s each)
      - "Use reasoning_auto for mode selection" (15s)
      
2. Agent chooses reasoning_tree (3 branches)
   -> Succeeds in 19s
   -> Metadata records: graph_timeout -> tree_success transition
   
3. Next time graph times out, tree is suggested first (learned from history)
```

---

## Appendix B: Code Locations

### New Files
- `src/metrics/chains.rs` - Chain detection logic
- `src/metrics/transitions.rs` - Transition tracking
- `src/error/enhanced.rs` - Error enhancement
- `docs/TOOL_COMPOSITION_GUIDE.md`
- `docs/ERROR_HANDLING_GUIDE.md`

### Modified Files
- `src/metrics/mod.rs` - Add chain methods
- `src/metadata/suggestions.rs` - Add dynamic suggestions
- `src/server/responses.rs` - Add error/alternatives fields (all types)
- `src/server/tools.rs` - Integrate ErrorEnhancer (all handlers)
- `docs/TOOL_REFERENCE.md` - Document error responses
- `README.md` - Add sections

### Test Files
- `tests/integration/chains.rs`
- `tests/integration/error_enhancement.rs`
- `src/metrics/chains.rs` (unit tests inline)
- `src/error/enhanced.rs` (unit tests inline)

---

**End of Plan**
