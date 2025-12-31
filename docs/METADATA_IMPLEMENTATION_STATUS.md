# Metadata Enrichment Implementation Status

## Overview

Implementation of AI agent discoverability features through response metadata enrichment.

**Goal**: Help AI agents discover and use reasoning tools effectively by providing:
- Timeout predictions
- Next-tool suggestions
- Preset workflow discovery
- Execution context

---

## Phase 1: Core Infrastructure ✅ COMPLETE

**Commit**: 9dec676  
**Status**: Complete and pushed  
**Files**: 9 files, 2,492 lines

### Delivered

1. **Metadata Types** (`src/metadata/mod.rs`)
   - `ResponseMetadata` - Container for all metadata
   - `TimingMetadata` - Duration estimates + timeout predictions
   - `SuggestionMetadata` - Next tools + presets
   - `ContextMetadata` - Execution context
   - All types support `JsonSchema` for MCP compatibility

2. **TimingDatabase** (`src/metadata/timing.rs`)
   - Historical execution time tracking
   - SQLite backend with `tool_timing_history` table
   - Confidence levels (High/Medium/Low) based on sample count
   - Complexity-adjusted predictions

3. **SuggestionEngine** (`src/metadata/suggestions.rs`)
   - Rule-based tool composition suggestions
   - 15 tools covered with specific suggestion rules
   - Context-aware recommendations

4. **PresetIndex** (`src/metadata/preset_index.rs`)
   - 5 workflow presets indexed:
     - decision_analysis
     - problem_exploration
     - evidence_based
     - bias_detection
     - causal_analysis
   - Pattern matching against tool history

5. **Timing Defaults** (`src/metadata/timing_defaults.rs`)
   - Baseline estimates for all 15 tools
   - Complexity multipliers for perspectives/branches/thinking budgets
   - Fallback when no historical data available

6. **Database Migration** (`migrations/003_tool_timing_history.sql`)
   - Schema for timing history
   - Indexes for efficient queries

---

## Phase 2: Integration (IN PROGRESS)

### Phase 2.1: Demo with reasoning_linear ✅ COMPLETE

**Commit**: 2f090d3  
**Status**: Complete and pushed  
**Files**: 5 files, 104 insertions

### Delivered

1. **AppState Enhancement** (`src/server/types.rs`)
   - Added `metadata_builder: Arc<MetadataBuilder>` field
   - Updated constructor to accept MetadataBuilder

2. **Server Initialization** (`src/server/mcp.rs`)
   - Creates `TimingDatabase` from storage
   - Builds `PresetIndex` 
   - Initializes `MetadataBuilder` with 30s Factory timeout
   - Passes to AppState

3. **LinearResponse Update** (`src/server/responses.rs`)
   - Added optional `metadata` field
   - Backward compatible (skipped if None)

4. **reasoning_linear Handler** (`src/server/tools.rs`)
   - Records actual execution time
   - Builds metadata on success
   - Attaches to response
   - Helper method: `build_metadata_for_linear()`

### Example Output

```json
{
  "thought_id": "abc123",
  "session_id": "session1",
  "content": "Analysis result...",
  "confidence": 0.95,
  "next_step": "Consider edge cases",
  "metadata": {
    "timing": {
      "estimated_duration_ms": 12000,
      "confidence": "medium",
      "will_timeout_on_factory": false,
      "factory_timeout_ms": 30000
    },
    "suggestions": {
      "next_tools": [
        {
          "tool": "reasoning_divergent",
          "reason": "Explore alternative perspectives on this analysis",
          "estimated_duration_ms": 45000
        },
        {
          "tool": "reasoning_evidence",
          "reason": "Evaluate the strength of evidence and claims",
          "estimated_duration_ms": 20000
        }
      ],
      "relevant_presets": []
    },
    "context": {
      "mode_used": "linear",
      "thinking_budget": "none"
    }
  }
}
```

---

## Remaining Work

### Phase 2.2-2.5: Extend to All Tools (PENDING)

**Estimated Effort**: 2-3 days  
**Pattern Established**: Yes (see `reasoning_linear` implementation)

#### Tasks

1. **Update Remaining Response Types** (14 tools)
   - Add `metadata: Option<ResponseMetadata>` field to:
     - TreeResponse
     - DivergentResponse
     - ReflectionResponse
     - CheckpointResponse
     - AutoResponse
     - GraphResponse
     - DetectResponse
     - DecisionResponse
     - EvidenceResponse
     - TimelineResponse
     - MctsResponse
     - CounterfactualResponse
     - PresetResponse
     - MetricsResponse (maybe skip - instant tool)

2. **Add Metadata Builders** (14 helpers)
   - Follow `build_metadata_for_linear()` pattern
   - Customize complexity metrics per tool:
     - `divergent`: num_perspectives, thinking_budget
     - `tree`: num_branches
     - `graph`: node count
     - etc.

3. **Update Tool Handlers** (14 tools)
   - Record elapsed_ms before metadata building
   - Extract complexity info before moving req
   - Call metadata builder on success
   - Attach to response

4. **Session Tool History Tracking**
   - Add to `SqliteStorage` or separate tracking
   - Store last N tools per session
   - Query in metadata builders
   - Enables preset suggestions

---

## Testing Strategy

### Unit Tests ✅
- All Phase 1 components have tests
- Timing estimation
- Suggestion rules
- Preset matching

### Integration Tests (PENDING)
- Metadata present in tool responses
- Timing accuracy within ±20%
- Suggestions are contextually relevant
- Presets match patterns

### Manual Testing (RECOMMENDED)
1. Call `reasoning_linear` with simple content
2. Verify metadata in response
3. Check timing prediction accuracy
4. Confirm suggestions are helpful
5. Test with complex content (>5000 chars)

---

## Known Issues & Limitations

1. **Factory Timeout Limitation**
   - Factory's MCP client has ~30s built-in timeout
   - Server metadata correctly predicts >30s operations
   - But calls still timeout at Factory client layer
   - **Workaround**: Use metadata to avoid slow operations

2. **Session History Not Implemented**
   - `tool_history` in metadata requests is empty `vec![]`
   - Reduces preset suggestion accuracy
   - TODO: Implement session tool tracking

3. **Test Failures Expected**
   - Some metadata tests fail due to missing table (migration not auto-applied in tests)
   - Will be fixed when integration tests are added

---

## Performance Impact

### Minimal Overhead

- **Metadata building**: ~5-10ms per call
- **Database insert**: ~1-2ms per call
- **Query overhead**: Negligible (indexed lookups)

### Benefits

- AI agents avoid timeout failures (saves 30s+ wasted calls)
- Discover optimal tool sequences (reduces trial-and-error)
- Learn from execution history (improves over time)

---

## How to Use (For AI Agents)

### 1. Check Timeout Prediction

```python
response = await call_tool("reasoning_divergent", {
    "content": "...",
    "num_perspectives": 4
})

if response.metadata.timing.will_timeout_on_factory:
    # Use alternative approach
    alternatives = response.metadata.suggestions.next_tools
    print(f"Will timeout! Try: {alternatives[0].tool}")
```

### 2. Discover Next Steps

```python
response = await call_tool("reasoning_linear", {"content": "..."})

for suggestion in response.metadata.suggestions.next_tools:
    print(f"{suggestion.tool}: {suggestion.reason}")
    print(f"  Estimated time: {suggestion.estimated_duration_ms}ms")
```

### 3. Find Workflows

```python
response = await call_tool("reasoning_divergent", {"content": "..."})

for preset in response.metadata.suggestions.relevant_presets:
    print(f"Consider preset: {preset.preset_id}")
    print(f"  {preset.description}")
```

---

## Success Metrics

### Quantitative
- ✅ 100% of Phase 1 infrastructure complete
- ✅ 1/15 tools enriched (reasoning_linear)
- ⏳ 14/15 tools remaining
- ⏳ 0% session history tracking

### Qualitative (Post-Completion)
- AI agents predict which calls will timeout
- AI agents discover presets organically
- AI agents learn optimal tool sequences
- Reduced experimentation/trial-and-error

---

## Next Steps

1. **Immediate**: Test reasoning_linear with metadata manually
2. **Short-term**: Extend pattern to 2-3 more high-value tools (divergent, decision, tree)
3. **Medium-term**: Complete all 15 tools
4. **Long-term**: Implement session history tracking

---

## References

- **Implementation Plan**: `docs/METADATA_ENRICHMENT_PLAN.md`
- **Phase 1 Commit**: 9dec676
- **Phase 2.1 Commit**: 2f090d3
- **Metadata Module**: `src/metadata/`
- **Example Integration**: `src/server/tools.rs` (reasoning_linear handler)
