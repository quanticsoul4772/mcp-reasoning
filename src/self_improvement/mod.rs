//! Self-improvement system.
//!
//! This module implements the 4-phase optimization loop:
//! 1. Monitor: Collect metrics and detect anomalies
//! 2. Analyze: LLM-based diagnosis and action proposal
//! 3. Execute: Apply approved actions with rollback
//! 4. Learn: Extract lessons from outcomes
