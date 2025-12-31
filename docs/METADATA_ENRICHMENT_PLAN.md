# Metadata Enrichment Implementation Plan

## Problem Statement

AI agents (primary users) are underutilizing the reasoning server's capabilities because:
- No visibility into tool execution time → can't predict Factory timeouts
- No guidance on tool composition → don't know which tools work well together
- No discovery of presets/workflows → unaware of built-in patterns
- No context-aware suggestions → miss optimization opportunities

**Impact**: AI agents use <20% of available capabilities despite rich feature set.

---

## Solution: Rich Response Metadata

Add structured metadata to every tool response enabling AI agents to:
1. **Predict timeouts** before making calls
2. **Discover next steps** through intelligent suggestions
3. **Find relevant presets** for current workflow
4. **Learn patterns** through usage examples

---

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────┐
│                    Tool Handler                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │  1. Execute Tool Logic (existing)                  │  │
│  │  2. Generate Result                                │  │
│  │  3. Enrich with Metadata (NEW)                     │  │
│  │     ├─ Timing estimates                            │  │
│  │     ├─ Timeout predictions                         │  │
│  │     ├─ Tool suggestions                            │  │
│  │     └─ Preset recommendations                      │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
           │
           ▼
    ┌─────────────────┐
    │ MetadataBuilder │ (NEW)
    │  - Timing DB    │
    │  - Suggestion   │
    │    Rules Engine │
    │  - Preset Index │
    └─────────────────┘
```

### Response Structure

```json
{
  // Existing tool-specific fields (unchanged)
  "thought_id": "...",
  "content": "...",
  
  // NEW: Universal metadata object
  "metadata": {
    "timing": {
      "estimated_duration_ms": 12000,
      "confidence": "high",  // high/medium/low based on historical data
      "will_timeout_on_factory": false,
      "factory_timeout_ms": 30000
    },
    "suggestions": {
      "next_tools": [
        {
          "tool": "reasoning_decision",
          "reason": "Synthesize these 4 perspectives into decision options",
          "estimated_duration_ms": 15000
        },
        {
          "tool": "reasoning_checkpoint",
          "reason": "Save this analysis before continuing exploration",
          "estimated_duration_ms": 100
        }
      ],
      "relevant_presets": [
        {
          "preset_id": "decision_analysis",
          "description": "Complete decision-making workflow",
          "estimated_duration_ms": 45000
        }
      ]
    },
    "context": {
      "mode_used": "divergent",
      "thinking_budget": "standard",
      "session_state": "has_active_branches"
    }
  }
}
```

---

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1)

#### 1.1 Create Metadata Module

**File**: `src/metadata/mod.rs`

```rust
/// Response metadata for tool discoverability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub timing: TimingMetadata,
    pub suggestions: SuggestionMetadata,
    pub context: ContextMetadata,
}

/// Timing predictions and timeout analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingMetadata {
    pub estimated_duration_ms: u64,
    pub confidence: ConfidenceLevel,  // High/Medium/Low
    pub will_timeout_on_factory: bool,
    pub factory_timeout_ms: u64,
}

/// Tool and preset suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetadata {
    pub next_tools: Vec<ToolSuggestion>,
    pub relevant_presets: Vec<PresetSuggestion>,
}

/// Execution context information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub mode_used: String,
    pub thinking_budget: Option<String>,  // "standard", "deep", "maximum"
    pub session_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSuggestion {
    pub tool: String,
    pub reason: String,
    pub estimated_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSuggestion {
    pub preset_id: String,
    pub description: String,
    pub estimated_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,    // Based on >100 samples
    Medium,  // Based on 10-100 samples
    Low,     // Based on <10 samples or estimation
}
```

#### 1.2 Create Timing Database

**File**: `src/metadata/timing.rs`

```rust
/// Historical timing data for duration predictions.
pub struct TimingDatabase {
    storage: Arc<SqliteStorage>,
}

impl TimingDatabase {
    /// Get estimated duration for a tool/mode combination.
    pub async fn estimate_duration(
        &self,
        tool: &str,
        mode: Option<&str>,
        complexity: ComplexityMetrics,
    ) -> Result<(u64, ConfidenceLevel), MetadataError> {
        // Query historical metrics
        // Apply complexity adjustments
        // Return estimate with confidence
    }
    
    /// Record actual execution time for learning.
    pub async fn record_execution(
        &self,
        tool: &str,
        mode: Option<&str>,
        duration_ms: u64,
        complexity: ComplexityMetrics,
    ) -> Result<(), MetadataError> {
        // Store in metrics for future predictions
    }
}

/// Complexity factors affecting execution time.
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    pub num_perspectives: Option<u32>,
    pub num_branches: Option<u32>,
    pub content_length: usize,
    pub thinking_budget: Option<u32>,
}
```

**Database Schema** (add to migrations):

```sql
CREATE TABLE IF NOT EXISTS tool_timing_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_name TEXT NOT NULL,
    mode_name TEXT,
    duration_ms INTEGER NOT NULL,
    complexity_score INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    INDEX idx_timing_lookup (tool_name, mode_name)
);
```

#### 1.3 Create Suggestion Engine

**File**: `src/metadata/suggestions.rs`

```rust
/// Rule-based suggestion engine for tool composition.
pub struct SuggestionEngine {
    preset_index: PresetIndex,
}

impl SuggestionEngine {
    /// Generate next-tool suggestions based on current context.
    pub fn suggest_next_tools(
        &self,
        current_tool: &str,
        result_context: &ResultContext,
    ) -> Vec<ToolSuggestion> {
        match current_tool {
            "reasoning_divergent" => self.suggest_after_divergent(result_context),
            "reasoning_tree" => self.suggest_after_tree(result_context),
            "reasoning_linear" => self.suggest_after_linear(result_context),
            // ... other tools
            _ => vec![],
        }
    }
    
    /// Find relevant presets for current workflow.
    pub fn suggest_presets(
        &self,
        tool_history: &[String],
        current_goal: Option<&str>,
    ) -> Vec<PresetSuggestion> {
        // Match tool history patterns to preset workflows
        self.preset_index.find_matching_presets(tool_history, current_goal)
    }
}

/// Context from tool execution result.
pub struct ResultContext {
    pub num_outputs: usize,
    pub has_branches: bool,
    pub session_id: Option<String>,
    pub complexity: String,  // "simple", "moderate", "complex"
}
```

**Suggestion Rules** (examples):

```rust
fn suggest_after_divergent(&self, ctx: &ResultContext) -> Vec<ToolSuggestion> {
    let mut suggestions = vec![];
    
    // Always suggest checkpoint after complex analysis
    suggestions.push(ToolSuggestion {
        tool: "reasoning_checkpoint".into(),
        reason: "Save this multi-perspective analysis before continuing".into(),
        estimated_duration_ms: 100,
    });
    
    // Suggest decision analysis if multiple perspectives
    if ctx.num_outputs >= 3 {
        suggestions.push(ToolSuggestion {
            tool: "reasoning_decision".into(),
            reason: format!("Synthesize {} perspectives into decision options", ctx.num_outputs),
            estimated_duration_ms: 15000,
        });
    }
    
    // Suggest reflection if complex
    if ctx.complexity == "complex" {
        suggestions.push(ToolSuggestion {
            tool: "reasoning_reflection".into(),
            reason: "Evaluate and refine this analysis".into(),
            estimated_duration_ms: 25000,
        });
    }
    
    suggestions
}
```

#### 1.4 Create Preset Index

**File**: `src/metadata/preset_index.rs`

```rust
/// Index of available presets with metadata.
pub struct PresetIndex {
    presets: HashMap<String, PresetMetadata>,
}

pub struct PresetMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,  // Sequence of tools
    pub estimated_duration_ms: u64,
    pub use_cases: Vec<String>,
    pub keywords: Vec<String>,
}

impl PresetIndex {
    /// Build index from builtin presets and custom definitions.
    pub fn build() -> Self {
        let mut presets = HashMap::new();
        
        // Example: Decision analysis preset
        presets.insert("decision_analysis".into(), PresetMetadata {
            id: "decision_analysis".into(),
            name: "Complete Decision Analysis".into(),
            description: "Multi-perspective analysis → criteria weighting → final recommendation".into(),
            tools: vec![
                "reasoning_divergent".into(),
                "reasoning_decision".into(),
                "reasoning_reflection".into(),
            ],
            estimated_duration_ms: 45000,
            use_cases: vec![
                "Complex decisions with trade-offs".into(),
                "Stakeholder alignment".into(),
            ],
            keywords: vec!["decision", "choice", "options", "trade-off"],
        });
        
        // Load other presets...
        
        Self { presets }
    }
    
    /// Find presets matching tool history pattern.
    pub fn find_matching_presets(
        &self,
        tool_history: &[String],
        goal: Option<&str>,
    ) -> Vec<PresetSuggestion> {
        // Match patterns and goals
        // Rank by relevance
        // Return top 3
    }
}
```

---

### Phase 2: Integration into Tool Handlers (Week 2)

#### 2.1 Create Metadata Builder

**File**: `src/metadata/builder.rs`

```rust
/// Builder for constructing response metadata.
pub struct MetadataBuilder {
    timing_db: Arc<TimingDatabase>,
    suggestion_engine: Arc<SuggestionEngine>,
    factory_timeout_ms: u64,
}

impl MetadataBuilder {
    /// Build complete metadata for a tool response.
    pub async fn build(
        &self,
        request: &MetadataRequest,
    ) -> Result<ResponseMetadata, MetadataError> {
        // 1. Estimate timing
        let timing = self.build_timing_metadata(request).await?;
        
        // 2. Generate suggestions
        let suggestions = self.build_suggestion_metadata(request).await?;
        
        // 3. Build context
        let context = self.build_context_metadata(request);
        
        Ok(ResponseMetadata {
            timing,
            suggestions,
            context,
        })
    }
    
    async fn build_timing_metadata(
        &self,
        request: &MetadataRequest,
    ) -> Result<TimingMetadata, MetadataError> {
        let (estimated_duration_ms, confidence) = self.timing_db
            .estimate_duration(
                &request.tool_name,
                request.mode_name.as_deref(),
                request.complexity.clone(),
            )
            .await?;
        
        Ok(TimingMetadata {
            estimated_duration_ms,
            confidence,
            will_timeout_on_factory: estimated_duration_ms > self.factory_timeout_ms,
            factory_timeout_ms: self.factory_timeout_ms,
        })
    }
    
    async fn build_suggestion_metadata(
        &self,
        request: &MetadataRequest,
    ) -> Result<SuggestionMetadata, MetadataError> {
        let next_tools = self.suggestion_engine.suggest_next_tools(
            &request.tool_name,
            &request.result_context,
        );
        
        let relevant_presets = self.suggestion_engine.suggest_presets(
            &request.tool_history,
            request.goal.as_deref(),
        );
        
        Ok(SuggestionMetadata {
            next_tools,
            relevant_presets,
        })
    }
    
    fn build_context_metadata(&self, request: &MetadataRequest) -> ContextMetadata {
        ContextMetadata {
            mode_used: request.mode_name.clone().unwrap_or_else(|| "none".into()),
            thinking_budget: request.thinking_budget.clone(),
            session_state: request.session_state.clone(),
        }
    }
}

/// Request context for metadata generation.
pub struct MetadataRequest {
    pub tool_name: String,
    pub mode_name: Option<String>,
    pub complexity: ComplexityMetrics,
    pub result_context: ResultContext,
    pub tool_history: Vec<String>,
    pub goal: Option<String>,
    pub thinking_budget: Option<String>,
    pub session_state: Option<String>,
}
```

#### 2.2 Update Tool Handler Infrastructure

**File**: `src/server/tools.rs` (modifications)

```rust
// Add metadata field to all tool response types
#[derive(Debug, Serialize, Deserialize)]
pub struct LinearResponse {
    pub thought_id: String,
    pub content: String,
    pub confidence: Option<f64>,
    pub metadata: ResponseMetadata,  // NEW
}

// Similar updates for all other response types:
// - TreeResponse
// - DivergentResponse
// - ReflectionResponse
// - etc.
```

#### 2.3 Integrate into Each Tool Handler

**Pattern** (apply to all 15 tools):

```rust
// Before
async fn handle_reasoning_linear(&self, args: Value) -> Result<Value, ToolError> {
    let request: LinearRequest = serde_json::from_value(args)?;
    
    // Execute mode
    let result = self.linear_mode.process(&request).await?;
    
    // Return result
    Ok(serde_json::to_value(result)?)
}

// After
async fn handle_reasoning_linear(&self, args: Value) -> Result<Value, ToolError> {
    let request: LinearRequest = serde_json::from_value(args)?;
    let start = Instant::now();
    
    // Execute mode
    let result = self.linear_mode.process(&request).await?;
    
    // Build metadata
    let metadata = self.metadata_builder.build(&MetadataRequest {
        tool_name: "reasoning_linear".into(),
        mode_name: Some("linear".into()),
        complexity: ComplexityMetrics {
            content_length: request.content.len(),
            thinking_budget: None,
            num_perspectives: None,
            num_branches: None,
        },
        result_context: ResultContext {
            num_outputs: 1,
            has_branches: false,
            session_id: request.session_id.clone(),
            complexity: "simple".into(),
        },
        tool_history: self.get_session_tool_history(&request.session_id).await,
        goal: None,
        thinking_budget: Some("none".into()),
        session_state: None,
    }).await?;
    
    // Record actual timing
    let actual_duration = start.elapsed().as_millis() as u64;
    self.metadata_builder.timing_db.record_execution(
        "reasoning_linear",
        Some("linear"),
        actual_duration,
        complexity,
    ).await?;
    
    // Add metadata to response
    let response = LinearResponse {
        thought_id: result.thought_id,
        content: result.content,
        confidence: result.confidence,
        metadata,
    };
    
    Ok(serde_json::to_value(response)?)
}
```

#### 2.4 Add Session Tool History Tracking

**File**: `src/storage/session.rs` (additions)

```rust
impl SqliteStorage {
    /// Get recent tool calls for a session.
    pub async fn get_session_tool_history(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<String>, StorageError> {
        let query = "
            SELECT DISTINCT tool_name 
            FROM metrics_invocations 
            WHERE session_id = ? 
            ORDER BY timestamp DESC 
            LIMIT ?
        ";
        
        let tools: Vec<String> = sqlx::query_scalar(query)
            .bind(session_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?;
        
        Ok(tools)
    }
}
```

---

### Phase 3: Baseline Timing Data (Week 3)

#### 3.1 Create Timing Calibration Tool

**File**: `benches/calibrate_timing.rs`

```rust
/// Benchmark tool for calibrating initial timing estimates.
#[tokio::main]
async fn main() -> Result<()> {
    let storage = SqliteStorage::new(":memory:").await?;
    let client = create_test_client()?;
    let timing_db = TimingDatabase::new(Arc::new(storage));
    
    // Run each tool 20 times with varying complexity
    for complexity in [SIMPLE, MODERATE, COMPLEX] {
        for tool in ALL_TOOLS {
            let durations = benchmark_tool(tool, complexity, 20).await?;
            
            for duration in durations {
                timing_db.record_execution(
                    tool,
                    None,
                    duration,
                    complexity,
                ).await?;
            }
        }
    }
    
    println!("Calibration complete. {} samples recorded.", total);
    Ok(())
}
```

#### 3.2 Initial Timing Estimates (Hardcoded Fallbacks)

**File**: `src/metadata/timing_defaults.rs`

```rust
/// Default timing estimates (used when no historical data available).
pub fn get_default_timing(tool: &str, complexity: &ComplexityMetrics) -> u64 {
    let base_time = match tool {
        // Fast tools (<5s)
        "reasoning_checkpoint" => 100,
        "reasoning_metrics" => 500,
        "reasoning_si_status" => 100,
        
        // Standard tools (8-15s)
        "reasoning_linear" => 12_000,
        "reasoning_auto" => 10_000,
        
        // Medium tools (15-30s)
        "reasoning_tree" => 18_000,
        "reasoning_decision" => 20_000,
        "reasoning_evidence" => 22_000,
        
        // Heavy tools (30-60s)
        "reasoning_divergent" => 45_000,
        "reasoning_reflection" => 35_000,
        
        // Very heavy tools (60-120s)
        "reasoning_graph" => 75_000,
        "reasoning_mcts" => 90_000,
        "reasoning_counterfactual" => 65_000,
        "reasoning_timeline" => 55_000,
        
        _ => 15_000,  // Default fallback
    };
    
    // Apply complexity multipliers
    let complexity_factor = match (
        complexity.num_perspectives,
        complexity.num_branches,
        complexity.thinking_budget,
    ) {
        (Some(4..), _, _) => 1.5,  // 4+ perspectives
        (_, Some(4..), _) => 1.4,  // 4+ branches
        (_, _, Some(8192..)) => 1.3,  // Deep/maximum thinking
        _ => 1.0,
    };
    
    (base_time as f64 * complexity_factor) as u64
}
```

---

### Phase 4: Testing & Validation (Week 4)

#### 4.1 Unit Tests

**File**: `src/metadata/tests.rs`

```rust
#[tokio::test]
async fn test_timing_estimation_linear() {
    let timing_db = create_test_timing_db().await;
    
    // Record some samples
    for _ in 0..10 {
        timing_db.record_execution(
            "reasoning_linear",
            Some("linear"),
            12_000,
            simple_complexity(),
        ).await.unwrap();
    }
    
    // Estimate
    let (estimate, confidence) = timing_db.estimate_duration(
        "reasoning_linear",
        Some("linear"),
        simple_complexity(),
    ).await.unwrap();
    
    assert!(estimate >= 10_000 && estimate <= 15_000);
    assert_eq!(confidence, ConfidenceLevel::High);
}

#[tokio::test]
async fn test_suggestion_after_divergent() {
    let engine = SuggestionEngine::new();
    
    let ctx = ResultContext {
        num_outputs: 4,
        has_branches: false,
        session_id: Some("test".into()),
        complexity: "complex".into(),
    };
    
    let suggestions = engine.suggest_next_tools("reasoning_divergent", &ctx);
    
    assert!(suggestions.iter().any(|s| s.tool == "reasoning_decision"));
    assert!(suggestions.iter().any(|s| s.tool == "reasoning_checkpoint"));
}

#[tokio::test]
async fn test_timeout_prediction() {
    let builder = create_test_metadata_builder(30_000);  // 30s Factory timeout
    
    let request = MetadataRequest {
        tool_name: "reasoning_divergent".into(),
        mode_name: Some("divergent".into()),
        complexity: ComplexityMetrics {
            num_perspectives: Some(4),
            ..default()
        },
        ..default()
    };
    
    let metadata = builder.build(&request).await.unwrap();
    
    assert!(metadata.timing.will_timeout_on_factory);
    assert!(metadata.timing.estimated_duration_ms > 30_000);
}
```

#### 4.2 Integration Tests

**File**: `tests/metadata_integration_tests.rs`

```rust
#[tokio::test]
async fn test_linear_response_has_metadata() {
    let server = create_test_server().await;
    
    let request = json!({
        "content": "Test analysis",
        "session_id": "test"
    });
    
    let response: LinearResponse = server
        .call_tool("reasoning_linear", request)
        .await
        .unwrap();
    
    // Verify metadata present
    assert!(response.metadata.timing.estimated_duration_ms > 0);
    assert!(!response.metadata.suggestions.next_tools.is_empty());
    assert_eq!(response.metadata.context.mode_used, "linear");
}

#[tokio::test]
async fn test_metadata_learns_from_actual_timing() {
    let server = create_test_server().await;
    
    // Call tool 5 times
    for _ in 0..5 {
        server.call_tool("reasoning_linear", test_request()).await.unwrap();
    }
    
    // 6th call should have high confidence
    let response: LinearResponse = server
        .call_tool("reasoning_linear", test_request())
        .await
        .unwrap();
    
    assert_eq!(response.metadata.timing.confidence, ConfidenceLevel::High);
}
```

---

### Phase 5: Documentation (Week 4)

#### 5.1 Update README

Add section:

```markdown
## Response Metadata (AI Agent Features)

Every tool response includes rich metadata to help AI agents:

### Timing Predictions
- Estimated duration before making calls
- Factory timeout warnings
- Confidence levels based on historical data

### Smart Suggestions
- Next logical tools to call
- Relevant workflow presets
- Context-aware recommendations

### Example Response
\`\`\`json
{
  "result": "...",
  "metadata": {
    "timing": {
      "estimated_duration_ms": 12000,
      "will_timeout_on_factory": false,
      "confidence": "high"
    },
    "suggestions": {
      "next_tools": [
        {
          "tool": "reasoning_decision",
          "reason": "Synthesize perspectives",
          "estimated_duration_ms": 15000
        }
      ]
    }
  }
}
\`\`\`

See [METADATA.md](docs/METADATA.md) for full details.
```

#### 5.2 Create METADATA.md

**File**: `docs/METADATA.md`

Comprehensive guide covering:
- Metadata structure reference
- How timing predictions work
- Suggestion engine rules
- Using metadata effectively as an AI agent
- Examples for each tool

---

## Testing Strategy

### Unit Tests
- ✅ Timing estimation logic
- ✅ Suggestion rule engine
- ✅ Preset matching
- ✅ Confidence calculations

### Integration Tests
- ✅ Metadata present in all tool responses
- ✅ Timing predictions accurate within 20%
- ✅ Suggestions relevant to context
- ✅ Learning from actual execution times

### Manual Testing (AI Agent)
- ✅ Make timeout predictions accurate
- ✅ Verify suggestions are helpful
- ✅ Check preset recommendations match workflows
- ✅ Confirm discoverability improved

---

## Success Metrics

### Quantitative
- **100%** of tool responses include metadata
- **±20%** timing prediction accuracy after 10 samples
- **3-5** relevant next-tool suggestions per response
- **0 regressions** in existing tool functionality

### Qualitative (AI Agent Feedback)
- Can predict which calls will timeout
- Discover and use presets effectively
- Learn optimal tool sequences
- Reduce trial-and-error experimentation

---

## Migration & Backwards Compatibility

### Non-Breaking Changes
- Metadata is **additive** - existing response fields unchanged
- Old clients ignore `metadata` field
- New clients benefit from enhanced responses

### MCP Tool Schema Updates
- Update JSON schemas to include optional `metadata` field
- Document in tool descriptions
- No version bump required (backwards compatible)

---

## Future Enhancements

### Phase 6+ (Post-Launch)
1. **Machine learning timing predictions** - Replace heuristics with ML model
2. **User goal detection** - Infer intent from conversation context
3. **Workflow validation** - Detect invalid tool sequences before execution
4. **Interactive suggestions** - "Did you mean to call X instead of Y?"
5. **Cost optimization** - Suggest cheaper equivalent tool chains

---

## Implementation Checklist

### Week 1: Infrastructure
- [ ] Create `src/metadata/` module structure
- [ ] Implement `ResponseMetadata` types
- [ ] Build `TimingDatabase` with SQLite backend
- [ ] Create `SuggestionEngine` with rule patterns
- [ ] Build `PresetIndex` from builtin presets
- [ ] Write unit tests for each component

### Week 2: Integration
- [ ] Create `MetadataBuilder` orchestrator
- [ ] Update all 15 tool response types
- [ ] Integrate metadata into all tool handlers
- [ ] Add session tool history tracking
- [ ] Update MCP tool schemas
- [ ] Write integration tests

### Week 3: Calibration
- [ ] Create timing calibration tool
- [ ] Run benchmarks for baseline data
- [ ] Populate `timing_defaults.rs`
- [ ] Validate predictions against actuals
- [ ] Tune suggestion rules
- [ ] Test with real AI agent workflows

### Week 4: Polish
- [ ] Complete test coverage
- [ ] Write METADATA.md documentation
- [ ] Update README with examples
- [ ] Performance testing
- [ ] Code review and refactoring
- [ ] Commit and push

---

## Estimated Effort

- **Development**: 3-4 weeks (1 developer)
- **Testing**: Embedded in development
- **Documentation**: 2-3 days
- **Total**: ~1 month

**Complexity**: Medium
- Mostly additive changes
- Clear architecture
- Minimal risk to existing functionality
- High value for AI agents

---

## Notes

This implementation prioritizes **AI agent discoverability** over human UX. The metadata format is optimized for programmatic consumption by LLMs and other AI systems.

Key design decisions:
1. **Additive-only** - No breaking changes to existing responses
2. **Rule-based suggestions** - Simple, predictable, testable (vs ML complexity)
3. **Learning timing model** - Improves accuracy over time automatically
4. **Factory-aware** - Explicitly handles Factory's 30s timeout limitation
5. **Preset integration** - Leverages existing workflow system
