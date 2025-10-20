# Edge Policy Rego Bundles

Reusable Rego policy modules and templates for Edge Policy Hub.

## Overview

This library provides:
- **Helper Modules** (`lib/`): Reusable functions for geo validation, quota checks, tenant isolation, and time-based access control
- **Template Policies** (`templates/`): Complete policy examples demonstrating data residency, cost guardrails, and multi-tenant separation
- **OPA Tests** (`tests/`): Comprehensive unit tests for all modules and templates

All policies are embedded at compile-time using `include_dir` and exposed through library API functions.

## Structure

```
policies/
├── lib/                    # Reusable helper modules
│   ├── geo.rego           # Geographic validation
│   ├── quota.rego         # Quota and cost checks
│   ├── tenant.rego        # Multi-tenant isolation
│   └── time.rego          # Time-based access control
├── templates/              # Complete policy templates
│   ├── data_residency.rego
│   ├── cost_guardrail.rego
│   ├── multi_tenant_separation.rego
│   └── combined_guardrails.rego
└── tests/                  # OPA unit tests
    ├── geo_test.rego
    ├── quota_test.rego
    ├── tenant_test.rego
    ├── time_test.rego
    ├── data_residency_test.rego
    ├── cost_guardrail_test.rego
    ├── multi_tenant_separation_test.rego
    └── combined_guardrails_test.rego
```

## Helper Modules

### lib/geo.rego

Geographic validation for data residency enforcement:

- `is_eu_country(country_code)` - Check if country is in EU
- `is_eu_location(geo)` - Check if geo object represents EU location
- `validate_data_residency(resource_region, subject_location)` - Validate region match
- `eu_countries` - Set of all EU country codes

**Usage:**
```rego
import data.lib.geo

allow {
  geo.is_eu_country(input.environment.country)
  input.resource.region == "EU"
}
```

### lib/quota.rego

Quota and cost guardrail checks:

- `bandwidth_exceeded(used_gb, limit_gb)` - Check if bandwidth quota exceeded
- `bandwidth_within_limit(used_gb, limit_gb)` - Check if within quota
- `message_count_exceeded(count, limit)` - Check MQTT message rate limit
- `quota_percentage(used, limit)` - Calculate usage percentage
- `quota_remaining(used, limit)` - Calculate remaining quota
- `is_approaching_limit(used, limit, threshold_percent)` - Check if approaching limit

**Usage:**
```rego
import data.lib.quota

allow {
  input.action == "write"
  quota.bandwidth_within_limit(input.environment.bandwidth_used, 100)
}
```

### lib/tenant.rego

Multi-tenant isolation and validation:

- `matches(subject_tenant_id, resource_owner_tenant)` - Check tenant ID match
- `is_owner(subject, resource)` - Check tenant and user ownership
- `has_clearance(subject, required_level)` - Check clearance level
- `has_role(subject, role_name)` - Check if subject has role
- `has_any_role(subject, role_list)` - Check if subject has any role from list
- `validate_tenant_boundary(subject_tenant, resource_tenant)` - Strict boundary validation

**Usage:**
```rego
import data.lib.tenant

allow {
  tenant.validate_tenant_boundary(input.subject.tenant_id, input.resource.owner_tenant)
  tenant.has_clearance(input.subject, 2)
}
```

### lib/time.rego

Time-based access control:

- `is_business_hours(timestamp)` - Check if within 09:00-17:00 UTC
- `is_weekday(timestamp)` - Check if Monday-Friday
- `is_within_window(timestamp, start_time, end_time)` - Check custom time window
- `is_expired(timestamp, max_age_seconds)` - Check if timestamp is too old
- `parse_iso8601(timestamp_string)` - Parse ISO 8601 timestamps

**Usage:**
```rego
import data.lib.time

allow {
  time.is_business_hours(input.environment.time)
  time.is_weekday(input.environment.time)
}
```

## Template Policies

### templates/data_residency.rego

Enforces EU data residency by blocking EU resource access from non-EU locations.

**Key Features:**
- Tenant isolation check
- EU country validation
- Resource region matching
- Read/write action support

### templates/cost_guardrail.rego

Enforces bandwidth quotas to prevent cost overruns.

**Key Features:**
- Reads always allowed (don't count against quota)
- Writes blocked when quota exceeded
- Configurable limit (default 100 GB)
- Deny reason with usage details

### templates/multi_tenant_separation.rego

Enforces hard tenant boundaries to prevent cross-tenant access.

**Key Features:**
- Strict tenant ID validation
- Clearance level checks
- Admin override support
- Cross-tenant denial with reason

### templates/combined_guardrails.rego

Production-ready policy combining all guardrails.

**Key Features:**
- All three patterns integrated
- Multiple deny reasons collected
- Helper rules for modularity
- Time-based access control
- Admin overrides where appropriate

## Library API

### Rust Usage

```rust
use edge_policy_rego_bundles::{
    list_helpers,
    load_helper,
    list_template_policies,
    load_template_policy,
    load_all_helpers,
};

// List available helpers
let helpers = list_helpers();
// ["geo", "quota", "tenant", "time"]

// Load a specific helper
let geo_module = load_helper("geo").expect("geo helper not found");

// Load all helpers at once
let all_helpers = load_all_helpers();

// List template policies
let templates = list_template_policies();
// ["data_residency", "cost_guardrail", "multi_tenant_separation", "combined_guardrails"]

// Load a template
let template = load_template_policy("data_residency").expect("template not found");
```

## Testing

All policies include comprehensive OPA unit tests.

### Run All Tests

```bash
# From repository root
opa test libs/rego-bundles/policies/
```

### Run Specific Test Suite

```bash
# Test geo helpers
opa test libs/rego-bundles/policies/tests/geo_test.rego

# Test data residency template
opa test libs/rego-bundles/policies/tests/data_residency_test.rego
```

### Verbose Output

```bash
opa test -v libs/rego-bundles/policies/
```

### Coverage Report

```bash
opa test --coverage libs/rego-bundles/policies/
```

## Integration with Enforcer

The enforcer service can use these helpers in tenant-specific policies:

1. **Load helpers** using `load_all_helpers()` at startup
2. **Include helpers** in tenant bundle directories alongside tenant policies
3. **Import in tenant policies** using `import data.lib.geo`, etc.
4. **Evaluate** tenant policies with helper functions available

## Integration with Policy DSL

The policy-dsl compiler can reference these templates:

1. **Load templates** using `load_template_policy()` for code generation patterns
2. **Generate imports** for helper modules in compiled Rego
3. **Reuse patterns** from templates when compiling DSL to Rego

## Best Practices

1. **Always import helpers** at the top of tenant policies
2. **Use helper functions** instead of duplicating logic
3. **Test policies** with OPA test framework before deployment
4. **Document custom policies** with comments explaining business logic
5. **Version policies** using metadata.json in bundles
6. **Monitor performance** - helpers are optimized for < 2ms evaluation

## Performance

- All helpers are pure functions with no external data dependencies
- Rego short-circuits on first failing condition
- In-memory evaluation with no I/O
- Target: p99 < 2ms for policy evaluation including helper calls

## Contributing

When adding new helpers or templates:

1. Create the .rego file in appropriate directory (lib/ or templates/)
2. Add comprehensive OPA tests in tests/ directory
3. Update this README with documentation
4. Run `opa test` to verify all tests pass
5. Update library API in src/lib.rs if needed

Reference the main project `README.md` for overall architecture and contribution guidelines.
