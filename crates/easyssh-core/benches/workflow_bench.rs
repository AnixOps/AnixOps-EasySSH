//! Workflow Automation Performance Benchmarks
//!
//! Tests workflow engine, condition evaluation, and executor performance.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use easyssh_core::workflow_engine::{StepConfig, StepType, Workflow, WorkflowStep};
use easyssh_core::workflow_executor::ConditionEvaluator;
use easyssh_core::workflow_variables::VariableResolver;

fn bench_workflow_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_creation");

    group.bench_function("new", |b| {
        b.iter(|| {
            let _ = black_box(Workflow::new("Test Workflow"));
        });
    });

    group.bench_function("with_description", |b| {
        b.iter(|| {
            let _ = black_box(Workflow::new("Test Workflow").with_description("A test workflow"));
        });
    });

    for step_count in [1, 5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::new("add_steps", step_count),
            &step_count,
            |b, &step_count| {
                b.iter(|| {
                    let mut workflow = Workflow::new("Benchmark");
                    for i in 0..step_count {
                        let step = WorkflowStep::new(StepType::SshCommand, &format!("Step {}", i));
                        workflow.add_step(step);
                    }
                    black_box(workflow);
                });
            },
        );
    }

    group.finish();
}

fn bench_step_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_step_creation");

    let step_types = [
        StepType::SshCommand,
        StepType::SftpUpload,
        StepType::SftpDownload,
        StepType::LocalCommand,
        StepType::Condition,
        StepType::Loop,
        StepType::Wait,
        StepType::SetVariable,
        StepType::Notification,
        StepType::Parallel,
    ];

    for step_type in &step_types {
        let name = format!("{:?}", step_type);
        group.bench_with_input(BenchmarkId::new("new", name), step_type, |b, step_type| {
            b.iter(|| {
                let _ = black_box(WorkflowStep::new(step_type.clone(), "Test Step"));
            });
        });
    }

    group.bench_function("with_builder", |b| {
        b.iter(|| {
            let _ = black_box(
                WorkflowStep::new(StepType::SshCommand, "Command")
                    .with_description("Execute a command")
                    .with_timeout(30)
                    .with_retry(3, 5),
            );
        });
    });

    group.finish();
}

fn bench_step_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_step_config");

    group.bench_function("default_ssh_command", |b| {
        b.iter(|| {
            let _ = black_box(StepConfig::default_for(&StepType::SshCommand));
        });
    });

    group.bench_function("default_sftp_upload", |b| {
        b.iter(|| {
            let _ = black_box(StepConfig::default_for(&StepType::SftpUpload));
        });
    });

    group.bench_function("default_condition", |b| {
        b.iter(|| {
            let _ = black_box(StepConfig::default_for(&StepType::Condition));
        });
    });

    group.finish();
}

fn bench_condition_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_condition_eval");

    let resolver = VariableResolver::new();

    group.bench_with_input("true", &"true", |b, condition| {
        b.iter(|| {
            let _ = black_box(ConditionEvaluator::evaluate(condition, &resolver));
        });
    });

    group.bench_with_input("false", &"false", |b, condition| {
        b.iter(|| {
            let _ = black_box(ConditionEvaluator::evaluate(condition, &resolver));
        });
    });

    group.bench_with_input("eq", &"5 == 5", |b, condition| {
        b.iter(|| {
            let _ = black_box(ConditionEvaluator::evaluate(condition, &resolver));
        });
    });

    group.bench_with_input(
        "contains",
        &"'hello world' contains 'world'",
        |b, condition| {
            b.iter(|| {
                let _ = black_box(ConditionEvaluator::evaluate(condition, &resolver));
            });
        },
    );

    group.bench_with_input(
        "regex",
        &"'test123' matches 'test[0-9]+'",
        |b, condition| {
            b.iter(|| {
                let _ = black_box(ConditionEvaluator::evaluate(condition, &resolver));
            });
        },
    );

    group.finish();
}

fn bench_workflow_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("workflow_serialization");

    for step_count in [1, 5, 10, 20, 50] {
        let mut workflow = Workflow::new("Benchmark");
        let mut prev_id: Option<String> = None;

        for i in 0..step_count {
            let step = WorkflowStep::new(StepType::SshCommand, &format!("Step {}", i))
                .with_description(&format!("Description for step {}", i))
                .with_timeout(30);
            let id = workflow.add_step(step);

            if let Some(ref prev) = prev_id {
                workflow.connect(prev, &id).ok();
            }
            prev_id = Some(id);
        }

        let json = serde_json::to_string(&workflow).unwrap();

        group.bench_with_input(
            BenchmarkId::new("serialize", step_count),
            &step_count,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(serde_json::to_string(&workflow).unwrap());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", step_count),
            &step_count,
            |b, _| {
                b.iter(|| {
                    let _: Workflow = black_box(serde_json::from_str(&json).unwrap());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    workflow_benches,
    bench_workflow_creation,
    bench_step_creation,
    bench_step_config,
    bench_condition_evaluation,
    bench_workflow_serialization
);
criterion_main!(workflow_benches);
