# Rego Bundle Development Guide

This guide explains how to develop, test, and use Rego policy bundles in Edge Policy Hub.

## Overview

The `libs/rego-bundles` library provides reusable Rego modules and templates for common policy patterns:

- **Data Residency**: Enforce geographic restrictions on data access
- **Cost Guardrails**: Prevent cost overruns with quota enforcement
- **Multi-Tenant Separation**: Hard isolation between tenants
- **Combined Policies**: Production-ready policies integrating all patterns

## Architecture

### Helper Modules (lib/)

Helper modules are reusable functions that can be imported by tenant-specific policies. They follow OPA best practices:

- **Pure functions**: No side effects, deterministic output
- **Parameterized**: Accept inputs as parameters, not global data
- **Composable**: Can be combined to build complex policies
- **Tested**: Comprehensive unit tests for all functions

### Template Policies (templates/)

Template policies demonstrate complete policy patterns. They:

- Show best practices for policy structure
- Import and use helper modules
- Include documentation and examples
- Serve as starting points for tenant-specific policies

### Testing (tests/)

All modules and templates have OPA unit tests:

- Test files end with `_test.rego`
- Test rules start with `test_`
- Use `with` keyword to mock input and data
- Cover positive and negative cases
- Validate edge cases and error conditions

## Writing Helper Modules

### Structure

```rego
package lib.myhelper

import rego.v1

# Boolean helper (returns true/false)
my_check(param1, param2) {
  # conditions
}

# Value-returning helper
my_calculation(x, y) := result {
  result := x + y
}

# Constant/set
my_constants := {"value1", "value2"}
```

### Best Practices

1. **Use rego.v1**: Always import for stable semantics
2. **Document parameters**: Add comments explaining inputs and outputs
3. **Handle edge cases**: Check for missing/invalid inputs
4. **Keep it simple**: One responsibility per function
5. **Test thoroughly**: Cover all code paths

### Example: Adding a New Helper

```rego
package lib.network

import rego.v1

# Check if IP address is in private range
is_private_ip(ip) {
  startswith(ip, "10.")
}

is_private_ip(ip) {
  startswith(ip, "192.168.")
}

is_private_ip(ip) {
  startswith(ip, "172.")
  # More specific check for 172.16-31.x.x
}
```

## Writing Template Policies

### Structure

```rego
package templates.my_template

import rego.v1
import data.lib.helper1
import data.lib.helper2

default allow := false

allow {
  # conditions using helpers
  helper1.check(input.field1)
  helper2.validate(input.field2)
}

# Optional: deny with reason
deny_reason := "explanation" {
  # conditions
}
```

### Best Practices

1. **Default deny**: Always start with `default allow := false`
2. **Import helpers**: Use shared modules instead of duplicating logic
3. **Document inputs**: Explain required input structure
4. **Provide examples**: Show usage in comments
5. **Collect deny reasons**: Help with debugging and audit logs

## Testing Policies

### Writing Tests

```rego
package lib.myhelper

import rego.v1

test_my_check_true {
  my_check("valid", "input")
}

test_my_check_false {
  not my_check("invalid", "input")
}

test_with_mock_input {
  allow with input as {
    "subject": {"tenant_id": "tenant-a"},
    "resource": {"owner_tenant": "tenant-a"}
  }
}
```

### Running Tests

```bash
# All tests
opa test libs/rego-bundles/policies/

# Specific test file
opa test libs/rego-bundles/policies/tests/geo_test.rego

# Verbose output
opa test -v libs/rego-bundles/policies/

# Coverage report
opa test --coverage libs/rego-bundles/policies/
```

### Test Best Practices

1. **Test both paths**: True and false cases
2. **Use descriptive names**: `test_allow_admin_access`, not `test1`
3. **Mock inputs**: Use `with` keyword to provide test data
4. **Test edge cases**: Empty inputs, missing fields, boundary values
5. **Keep tests independent**: Don't rely on evaluation order

## Using Helpers in Tenant Policies

### Import Pattern

```rego
package tenants.my_tenant

import rego.v1
import data.lib.geo
import data.lib.quota
import data.lib.tenant

default allow := false

allow {
  tenant.matches(input.subject.tenant_id, input.resource.owner_tenant)
  geo.is_eu_country(input.environment.country)
  quota.bandwidth_within_limit(input.environment.bandwidth_used, 100)
}
```

### Deployment

When deploying tenant policies:

1. **Include helpers**: Copy helper .rego files to tenant bundle directory
2. **Maintain structure**: Keep `lib/` subdirectory structure
3. **Test before deploy**: Run OPA tests on complete bundle
4. **Version bundles**: Use metadata.json to track versions

## Integration with Enforcer

The enforcer service loads bundles from `config/tenants.d/{tenant_id}/`:

```
config/tenants.d/
├── tenant-a/
│   ├── policy.rego          # Tenant-specific policy
│   ├── lib/                 # Helper modules
│   │   ├── geo.rego
│   │   ├── quota.rego
│   │   └── tenant.rego
│   ├── data.json            # Optional static data
│   └── metadata.json        # Bundle metadata
```

The enforcer:
1. Loads all .rego files recursively
2. Creates a Regorus Engine instance per tenant
3. Evaluates policies with `engine.eval_rule("data.tenants.{tenant_id}.allow")`
4. Returns decision to proxy/bridge services

## Integration with Policy DSL

The policy-dsl compiler can generate imports for helpers:

```rust
// In codegen.rs
fn generate_imports() -> String {
    let mut imports = String::new();
    imports.push_str("import rego.v1\n");
    imports.push_str("import data.lib.geo\n");
    imports.push_str("import data.lib.quota\n");
    imports.push_str("import data.lib.tenant\n");
    imports
}
```

This allows DSL policies to use helper functions automatically.

## Performance Considerations

### Optimization Tips

1. **Short-circuit evaluation**: Put cheapest checks first
2. **Avoid loops**: Use sets and built-ins instead of iteration
3. **Cache results**: Use intermediate rules for repeated calculations
4. **Minimize data access**: Pass values as parameters instead of reading from data

### Example: Optimized Policy

```rego
# Good: Cheap checks first
allow {
  input.action == "read"              # Fast string comparison
  tenant.matches(...)                  # Fast equality check
  geo.is_eu_country(...)              # Set membership (fast)
  quota.bandwidth_within_limit(...)   # Numeric comparison
}

# Bad: Expensive checks first
allow {
  complex_calculation(...)            # Slow
  input.action == "read"              # Fast but evaluated last
}
```

## Troubleshooting

### Common Issues

**Import not found**
- Ensure helper .rego files are in bundle directory
- Check package name matches import path
- Verify file is loaded by enforcer

**Test failures**
- Check input structure matches policy expectations
- Verify mock data is complete
- Use `opa eval` to debug rule evaluation

**Performance issues**
- Profile with OPA's built-in profiler
- Check for expensive operations in hot paths
- Optimize rule ordering

### Debugging

```bash
# Evaluate a rule with input
opa eval -i input.json -d policies/ 'data.lib.geo.is_eu_country("DE")'

# Check policy syntax
opa check policies/

# Format policies
opa fmt -w policies/

# Profile evaluation
opa eval --profile -i input.json -d policies/ 'data.tenants.tenant_a.allow'
```

## Contributing

When contributing new helpers or templates:

1. Follow existing code style (use `opa fmt`)
2. Add comprehensive tests
3. Document parameters and behavior
4. Update this guide and README
5. Ensure all tests pass before submitting PR

## References

- [OPA Documentation](https://www.openpolicyagent.org/docs/latest/)
- [Rego Language Reference](https://www.openpolicyagent.org/docs/latest/policy-language/)
- [OPA Testing Guide](https://www.openpolicyagent.org/docs/latest/policy-testing/)
- [Edge Policy Hub README](../README.md)
- [Policy DSL Reference](./policy-dsl.md)
