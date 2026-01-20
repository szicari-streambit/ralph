// ABOUTME: Criterion benchmarks for Ralph core library
// ABOUTME: Measures performance of PRD parsing, ledger operations, and validation

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ralph_lib::ledger::{EventStatus, Ledger, LedgerEvent};
use ralph_lib::prd::{Prd, Requirement, RequirementStatus};

fn sample_prd() -> Prd {
    Prd {
        schema_version: "1.0".to_string(),
        slug: "benchmark-feature".to_string(),
        title: "Benchmark Feature".to_string(),
        active_run_id: "bench-20260119-1".to_string(),
        validation_profiles: vec!["rust-cargo".to_string()],
        requirements: (1..=10)
            .map(|i| Requirement {
                id: format!("REQ-{i:02}"),
                title: format!("Requirement {i}"),
                status: RequirementStatus::Todo,
                acceptance_criteria: vec![
                    format!("Given X{i}, when Y{i}, then Z{i}"),
                    format!("Given A{i}, when B{i}, then C{i}"),
                ],
            })
            .collect(),
    }
}

fn bench_prd_json_roundtrip(c: &mut Criterion) {
    let prd = sample_prd();
    let json = prd.to_json().unwrap();

    c.bench_function("prd_to_json", |b| {
        b.iter(|| black_box(prd.to_json().unwrap()));
    });

    c.bench_function("prd_from_json", |b| {
        b.iter(|| black_box(Prd::from_json(&json).unwrap()));
    });
}

fn bench_prd_markdown(c: &mut Criterion) {
    let prd = sample_prd();

    c.bench_function("prd_to_markdown", |b| {
        b.iter(|| black_box(prd.to_markdown()));
    });

    c.bench_function("prd_to_markdown_with_markers", |b| {
        b.iter(|| black_box(prd.to_markdown_with_markers(Some("Planning notes"))));
    });
}

fn bench_ledger_append(c: &mut Criterion) {
    c.bench_function("ledger_append_100", |b| {
        b.iter(|| {
            let mut ledger = Ledger::new();
            for i in 1..=100 {
                ledger
                    .append(LedgerEvent::new(i, "REQ-01", EventStatus::Started))
                    .unwrap();
            }
            black_box(ledger)
        });
    });
}

fn bench_ledger_query(c: &mut Criterion) {
    let mut ledger = Ledger::new();
    for i in 1..=1000 {
        let req = format!("REQ-{:02}", (i % 10) + 1);
        ledger
            .append(LedgerEvent::new(i, &req, EventStatus::Started))
            .unwrap();
    }

    c.bench_function("ledger_latest_iteration", |b| {
        b.iter(|| black_box(ledger.latest_iteration()));
    });

    c.bench_function("ledger_events_for_requirement", |b| {
        b.iter(|| black_box(ledger.events_for_requirement("REQ-05")));
    });

    c.bench_function("ledger_is_requirement_failed", |b| {
        b.iter(|| black_box(ledger.is_requirement_failed("REQ-05")));
    });
}

criterion_group!(
    benches,
    bench_prd_json_roundtrip,
    bench_prd_markdown,
    bench_ledger_append,
    bench_ledger_query
);
criterion_main!(benches);
