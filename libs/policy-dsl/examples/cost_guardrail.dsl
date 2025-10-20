# Cost Guardrail Policy
# Blocks uploads when monthly bandwidth quota is exceeded

deny write sensor_data if environment.bandwidth_used >= 100
