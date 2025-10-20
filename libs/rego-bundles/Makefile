# Makefile for Rego policy development and testing

# Default target - runs formatting check, tests, and build
.PHONY: all
all: check-fmt test build

# Run all OPA tests with verbose output
.PHONY: test
test:
	opa test -v policies/

# Generate coverage report
.PHONY: test-coverage
test-coverage:
	opa test --coverage policies/

# Run tests with detailed verbose output
.PHONY: test-verbose
test-verbose:
	opa test -v policies/

# Run tests with JSON output (useful for CI/CD)
.PHONY: test-json
test-json:
	opa test -f json policies/

# Test only helper modules
.PHONY: test-helpers
test-helpers:
	opa test policies/tests/geo_test.rego \
		policies/tests/quota_test.rego \
		policies/tests/tenant_test.rego \
		policies/tests/time_test.rego

# Test only template policies
.PHONY: test-templates
test-templates:
	opa test policies/tests/data_residency_test.rego \
		policies/tests/cost_guardrail_test.rego \
		policies/tests/multi_tenant_separation_test.rego \
		policies/tests/combined_guardrails_test.rego

# Format all Rego files
.PHONY: fmt
fmt:
	opa fmt -w policies/

# Check formatting without modifying files (useful for CI/CD)
.PHONY: check-fmt
check-fmt:
	opa fmt -l policies/

# Build the Rust library
.PHONY: build
build:
	cargo build --package edge-policy-rego-bundles

# Clean build artifacts
.PHONY: clean
clean:
	cargo clean

# Validate Rego syntax
.PHONY: check
check:
	opa check policies/
