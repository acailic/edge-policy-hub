# Policy DSL Reference

Edge Policy Hub ships with a concise Attribute-Based Access Control (ABAC) DSL that compiles into Rego. This reference covers syntax, attributes, operators, examples, and best practices.

## Syntax Overview

- **Structure:** `<effect> <action> <resource_type> if <conditions>`
- **Effects:** `allow` or `deny`
- **Actions:** `read`, `write`, `delete`, `execute`, `publish`, `subscribe`, `invoke`
- **Resource Type:** Identifier describing domain object (`sensor_data`, `payment_data`, `gps_telemetry`)
- **Conditions:** Boolean expression evaluated against ABAC input (`subject`, `resource`, `environment`, `action`)

```dsl
allow read sensor_data if
  subject.tenant_id == "tenant-a" and
  resource.region == "EU"
```

## ABAC Attributes

### Subject (Who)
- `subject.tenant_id` – Tenant namespace (string)
- `subject.user_id` – User identifier (string)
- `subject.device_id` – Device identifier (string)
- `subject.roles` – Array of roles (`["operator", "admin"]`)
- `subject.clearance_level` – Numeric classification (integer)
- `subject.device_location` – ISO country code or location tag (string)
- `subject.groups` – Hierarchical group membership (array)

### Resource (What)
- `resource.type` – Domain object type (`sensor_data`, `payment_data`)
- `resource.id` – Resource identifier (string)
- `resource.owner_tenant` – Owning tenant (string)
- `resource.owner_user` – Owning user (string)
- `resource.region` – Residency region (string)
- `resource.classification` – Sensitivity label (`public`, `internal`, `restricted`)

### Action (Operation)
- `action` – Literal string representing operation (`read`, `write`, `publish`)

### Environment (Context)
- `environment.time` – ISO 8601 timestamp or HH:MM string
- `environment.country` – Request origin country (string)
- `environment.geo` – Geolocation tuple or hash
- `environment.network` – Network metadata (e.g., `vpn`, `public`, `private`)
- `environment.risk_score` – ML/UEBA risk score (float)
- `environment.bandwidth_used` – Bandwidth counter in GB (number)
- `environment.message_count` – Message counter for quota enforcement (integer)
- `environment.device_trust_level` – Device trust score (number)

## Operators

| Operator | Description                         | Example                                        |
|----------|-------------------------------------|------------------------------------------------|
| `==`     | Equality                            | `subject.tenant_id == "tenant-a"`              |
| `!=`     | Inequality                          | `resource.region != "US"`                      |
| `<` `<=` | Less than / less than or equal      | `environment.bandwidth_used < 80`              |
| `>` `>=` | Greater than / greater than or equal| `subject.clearance_level >= 3`                 |
| `in`     | Membership                          | `subject.roles in ["admin", "operator"]`       |
| `and`    | Logical conjunction                 | `cond_a and cond_b`                            |
| `or`     | Logical disjunction                 | `cond_a or cond_b`                             |
| `not`    | Negation                            | `not subject.roles in ["suspended"]`           |
| `()`     | Parentheses to control precedence   | `(cond_a or cond_b) and cond_c`                |

Operator precedence (highest to lowest): parentheses, not, comparison/in, and, or.

## Literals

- **Strings:** Double-quoted with escape support (`"EU"`, `"tenant-a"`)
- **Numbers:** Integers or floats (`42`, `0.5`, `1000`)
- **Booleans:** `true`, `false`
- **Arrays:** `[ "EU", "US" ]`
- **Comments:** `#` inline or full line

## Complete Examples

### Data Residency
```dsl
allow read sensor_data if
  subject.tenant_id == "tenant-eu" and
  resource.region == "EU" and
  subject.device_location in ["DE", "FR", "NL"]
```

### Cost Guardrail
```dsl
deny write sensor_data if
  environment.bandwidth_used >= 100
```

### Multi-Tenant Separation
```dsl
allow read sensor_data if
  subject.tenant_id == resource.owner_tenant
```

### Time-Based Access
```dsl
allow read payment_data if
  environment.time >= "09:00" and
  environment.time <= "17:00"
```

### Role-Based Access
```dsl
allow execute admin_api if
  subject.roles in ["platform-admin", "security-admin"]
```

### Complex Policy
```dsl
allow publish gps_telemetry if
  subject.tenant_id == resource.owner_tenant and
  subject.device_id in ["gps_tracker"] and
  (environment.country in ["SG", "JP"] or environment.network == "private") and
  environment.message_count < 200000
```

## Compilation Pipeline

1. **Parsing:** DSL parsed into AST (`libs/policy-dsl/src/parser.rs`)
2. **Validation:** Semantic checks (required tenant guardrail, attribute existence)
3. **Code Generation:** AST transpiled to Rego modules (`libs/policy-dsl/src/codegen.rs`)
4. **Bundling:** Rego packaged with metadata and optional data.json
5. **Enforcer Load:** PolicyManager loads bundle into Regorus engine

**Example:** DSL → Rego

```dsl
allow read sensor_data if
  subject.tenant_id == "tenant-a"
```

```rego
package tenants.tenant_a.policy

default allow := false

allow {
  input.subject.tenant_id == "tenant-a"
  input.action == "read"
  input.resource.type == "sensor_data"
}
```

## Error Messages

- `E1001 Missing tenant guardrail` – Add `subject.tenant_id == "<tenant>"`.
- `E2002 Unknown attribute` – Attribute not registered in schema.
- `E3003 Invalid literal` – Strings must be quoted; arrays require brackets.
- `E4004 Circular condition` – Nested condition references itself.

The compiler returns structured diagnostics with line/column numbers. Use Tauri Policy Builder for inline highlighting.

## Best Practices

1. **One Concern per Policy** – Avoid mixing residency, quota, and role logic; compose multiple rules instead.
2. **Always Include Tenant Guardrail** – `subject.tenant_id == resource.owner_tenant` prevents cross-tenant leakage.
3. **Prefer Positive Logic** – Write allow rules with clear criteria and explicit deny for high-risk scenarios.
4. **Comment Business Logic** – Use `#` to document reasoning for auditors.
5. **Version Policies** – Include metadata (`version`, `author`, `description`) in bundles.
6. **Test Before Deploy** – Use simulator and staged bundles before activation.

## Limitations

- No loops or recursion (mirrors Rego constraints).
- No user-defined functions (use helper modules in Rego if needed).
- Maximum nesting depth is 8 to simplify audit review.
- DSL uses strict typing; implicit conversions (string ↔ number) are not permitted.
- `in` supports array literals but not dynamic arrays generated at runtime.

## Additional Resources

- Parser implementation: `libs/policy-dsl/src/parser.rs`
- AST definitions: `libs/policy-dsl/src/ast.rs`
- Sample policies: `libs/policy-dsl/examples/`
- Rego templates: `libs/rego-bundles/policies/templates/`

For feedback or feature requests, open an issue on the Edge Policy Hub repository.
