//! Microbenchmarks for dashboard activity emission.
//!
//! Backs the PR #189 claim that `dashboard::emit` is a "no-op when off / cheap
//! when on". Run with `cargo bench --bench dashboard_emit`.
//!
//! Three things are measured:
//! - `global_unset_noop` — `dashboard::emit` with the process-global sink never
//!   installed (the default, dashboard-off path): just an atomic `OnceLock` load
//!   that returns `None`, then the event is dropped.
//! - `bus_emit_subscribers/{0,1,4,16}` — `ActivityBus::emit` (exactly what the
//!   global wrapper forwards to once installed) with N live subscribers. The
//!   global wrapper adds only the one atomic load measured above. Receivers are
//!   held but never drained, so this also exercises the lossy full-buffer path:
//!   the send must stay O(1) and never block even when subscribers lag.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mcp_reasoning::dashboard::{self, ActivityBus, ActivityEvent, EdgeId, Node, Phase};

/// A representative event: the shape `metrics.record` emits for every tool call
/// (carries a heap `String` tool name, so per-subscriber clone cost is realistic).
fn sample_event() -> ActivityEvent {
    ActivityEvent::new(Node::Mode, Phase::Completed)
        .with_edge(EdgeId::ModeToClient)
        .with_tool("reasoning_linear")
        .with_duration_ms(123)
}

fn bench_emit(c: &mut Criterion) {
    let mut group = c.benchmark_group("dashboard_emit");
    group.throughput(Throughput::Elements(1));

    // "No-op when off": global sink never installed in this binary (no call to
    // set_global anywhere), so emit short-circuits on the OnceLock load.
    group.bench_function("global_unset_noop", |b| {
        b.iter(|| dashboard::emit(black_box(sample_event())));
    });

    // "Cheap when on": the underlying bus send, scaling subscribers. Buffers fill
    // and stay full (256 cap, lossy), so this is the steady-state worst case.
    for subs in [0usize, 1, 4, 16] {
        let bus = ActivityBus::new();
        let _receivers: Vec<_> = (0..subs).map(|_| bus.subscribe()).collect();
        group.bench_with_input(
            BenchmarkId::new("bus_emit_subscribers", subs),
            &bus,
            |b, bus| b.iter(|| bus.emit(black_box(sample_event()))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_emit);
criterion_main!(benches);
