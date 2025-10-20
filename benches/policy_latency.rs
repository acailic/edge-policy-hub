use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use edge_policy_hub::bench_support::PolicyBenchFixture;
use serde_json::json;
use tokio::runtime::Runtime;

const SIMPLE_POLICY: &str = r#"
package tenants.simple.policy

default allow := false

allow if {
  input.subject.tenant_id == "tenant-simple"
}
"#;

const DATA_RESIDENCY_POLICY: &str = r#"
package tenants.data_residency.policy

default allow := false

allow if {
  input.subject.tenant_id == "tenant-data"
  input.resource.region == "EU"
  input.environment.country == "DE"
}
"#;

const COST_GUARDRAIL_POLICY: &str = r#"
package tenants.cost.policy

default allow := false

allow if {
  input.environment.bandwidth_used < 90
}
"#;

const COMBINED_POLICY: &str = r#"
package tenants.combined.policy

default allow := false

allow if {
  input.subject.tenant_id == input.resource.owner_tenant
  input.subject.clearance_level >= 3
  input.resource.region == "EU"
  input.environment.bandwidth_used < 80
  input.environment.risk_score < 0.5
}
"#;

fn bench_policy_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("policy_evaluation");
    group
        .sample_size(1000)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3))
        .sampling_mode(SamplingMode::Auto);

    let simple_fixture = PolicyBenchFixture::new("tenant-simple", SIMPLE_POLICY);
    let simple_input = json!({
        "subject": { "tenant_id": "tenant-simple" },
        "resource": { "type": "sensor_data", "owner_tenant": "tenant-simple" },
        "action": "read",
        "environment": {}
    });
    group.bench_function(BenchmarkId::new("simple_policy", "allow"), |b| {
        let manager = simple_fixture.manager.clone();
        let tenant = simple_fixture.tenant_id.clone();
        let input = simple_input.clone();
        let runtime = Runtime::new().expect("tokio runtime");
        b.iter(|| {
            let decision = runtime
                .block_on(manager.evaluate(&tenant, input.clone()))
                .expect("policy evaluation");
            black_box(decision)
        });
    });

    let data_fixture = PolicyBenchFixture::new("tenant-data", DATA_RESIDENCY_POLICY);
    let data_input = json!({
        "subject": { "tenant_id": "tenant-data" },
        "resource": { "type": "sensor_data", "region": "EU", "owner_tenant": "tenant-data" },
        "action": "read",
        "environment": { "country": "DE" }
    });
    group.bench_function(BenchmarkId::new("data_residency", "allow"), |b| {
        let manager = data_fixture.manager.clone();
        let tenant = data_fixture.tenant_id.clone();
        let input = data_input.clone();
        let runtime = Runtime::new().expect("tokio runtime");
        b.iter(|| {
            let decision = runtime
                .block_on(manager.evaluate(&tenant, input.clone()))
                .expect("policy evaluation");
            black_box(decision)
        });
    });

    let cost_fixture = PolicyBenchFixture::new("tenant-cost", COST_GUARDRAIL_POLICY);
    let mut cost_input = json!({
        "subject": { "tenant_id": "tenant-cost" },
        "resource": { "type": "sensor_data", "owner_tenant": "tenant-cost" },
        "action": "write",
        "environment": { "bandwidth_used": 50 }
    });
    group.bench_function(BenchmarkId::new("cost_guardrail", "allow"), |b| {
        let manager = cost_fixture.manager.clone();
        let tenant = cost_fixture.tenant_id.clone();
        let runtime = Runtime::new().expect("tokio runtime");
        b.iter(|| {
            let mut input = cost_input.clone();
            input["environment"]["bandwidth_used"] = json!(45);
            let decision = runtime
                .block_on(manager.evaluate(&tenant, input))
                .expect("policy evaluation");
            black_box(decision)
        });
    });

    let combined_fixture = PolicyBenchFixture::new("tenant-combined", COMBINED_POLICY);
    let combined_input = json!({
        "subject": {
            "tenant_id": "tenant-combined",
            "roles": ["operator"],
            "clearance_level": 3
        },
        "resource": {
            "type": "sensor_data",
            "owner_tenant": "tenant-combined",
            "region": "EU"
        },
        "action": "write",
        "environment": {
            "bandwidth_used": 40,
            "risk_score": 0.1
        }
    });
    group.bench_function(BenchmarkId::new("combined_policy", "allow"), |b| {
        let manager = combined_fixture.manager.clone();
        let tenant = combined_fixture.tenant_id.clone();
        let input = combined_input.clone();
        let runtime = Runtime::new().expect("tokio runtime");
        b.iter(|| {
            let decision = runtime
                .block_on(manager.evaluate(&tenant, input.clone()))
                .expect("policy evaluation");
            black_box(decision)
        });
    });

    let runtime = Runtime::new().expect("tokio runtime");
    group.bench_function("concurrent_requests", |b| {
        let manager = combined_fixture.manager.clone();
        let tenant = combined_fixture.tenant_id.clone();
        let input = combined_input.clone();
        b.to_async(&runtime).iter(|| async {
            let futures = (0..100).map(|_| {
                let manager = manager.clone();
                let tenant = tenant.clone();
                let input = input.clone();
                tokio::spawn(async move {
                    manager
                        .evaluate(&tenant, input)
                        .await
                        .expect("policy evaluation concurrent")
                })
            });
            for task in futures {
                task.await.expect("join handle");
            }
        });
    });

    group.finish();
}

criterion_group!(policy_latency, bench_policy_evaluation);
criterion_main!(policy_latency);
