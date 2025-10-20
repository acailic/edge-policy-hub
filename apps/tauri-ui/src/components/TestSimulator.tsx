import { useCallback } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation } from "@tanstack/react-query";
import { Loader2, RefreshCw, TestTube } from "lucide-react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { testPolicy } from "../lib/api";
import type { TestPolicyResponse } from "../types/policy";
import type { AbacInput } from "../types/abac";
import { SAMPLE_ABAC_INPUTS } from "../utils/sample-policies";

interface TestSimulatorProps {
  tenantId: string;
}

const formSchema = z.object({
  subject: z.object({
    tenant_id: z.string().min(1, "Tenant ID is required"),
    user_id: z.string().optional(),
    device_id: z.string().optional(),
    roles: z.string().optional(),
    clearance_level: z.string().optional(),
    device_location: z.string().optional(),
  }),
  action: z.string().min(1, "Action is required"),
  resource: z.object({
    type: z.string().min(1, "Resource type is required"),
    id: z.string().optional(),
    classification: z.string().optional(),
    region: z.string().optional(),
    owner_tenant: z.string().min(1, "Owner tenant is required"),
    owner_user: z.string().optional(),
  }),
  environment: z.object({
    time: z.string().min(1, "Timestamp is required"),
    country: z.string().optional(),
    network: z.string().optional(),
    risk_score: z.string().optional(),
    bandwidth_used: z.string().optional(),
    message_count: z.string().optional(),
  }),
});

type PolicyTestForm = z.infer<typeof formSchema>;

function parseNumber(value?: string) {
  if (!value) {
    return undefined;
  }

  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function parseRoles(value?: string) {
  if (!value) {
    return undefined;
  }

  const roles = value
    .split(",")
    .map((role) => role.trim())
    .filter(Boolean);

  return roles.length > 0 ? roles : undefined;
}

function toIso(value: string) {
  if (!value) {
    return new Date().toISOString();
  }

  const date = new Date(value);
  return date.toISOString();
}

function toDateTimeLocal(value: string) {
  const date = new Date(value);
  const pad = (input: number) => input.toString().padStart(2, "0");
  const year = date.getFullYear();
  const month = pad(date.getMonth() + 1);
  const day = pad(date.getDate());
  const hour = pad(date.getHours());
  const minute = pad(date.getMinutes());
  return `${year}-${month}-${day}T${hour}:${minute}`;
}

function buildInput(values: PolicyTestForm): AbacInput {
  return {
    subject: {
      tenant_id: values.subject.tenant_id,
      user_id: values.subject.user_id?.trim() || undefined,
      device_id: values.subject.device_id?.trim() || undefined,
      roles: parseRoles(values.subject.roles),
      clearance_level: parseNumber(values.subject.clearance_level),
      device_location: values.subject.device_location?.trim() || undefined,
    },
    action: values.action,
    resource: {
      type: values.resource.type,
      id: values.resource.id?.trim() || undefined,
      classification: values.resource.classification?.trim() || undefined,
      region: values.resource.region?.trim() || undefined,
      owner_tenant: values.resource.owner_tenant,
      owner_user: values.resource.owner_user?.trim() || undefined,
    },
    environment: {
      time: toIso(values.environment.time),
      country: values.environment.country?.trim() || undefined,
      network: values.environment.network?.trim() || undefined,
      risk_score: parseNumber(values.environment.risk_score),
      bandwidth_used: parseNumber(values.environment.bandwidth_used),
      message_count: parseNumber(values.environment.message_count),
    },
  };
}

export function TestSimulator({ tenantId }: TestSimulatorProps) {
  const defaultTime = toDateTimeLocal(new Date().toISOString());
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<PolicyTestForm>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      subject: {
        tenant_id: tenantId,
        roles: "",
        clearance_level: "",
      },
      action: "read",
      resource: {
        type: "sensor_data",
        owner_tenant: tenantId,
      },
      environment: {
        time: defaultTime,
      },
    },
  });

  const mutation = useMutation<TestPolicyResponse, Error, AbacInput>({
    mutationFn: (input) => testPolicy(tenantId, input),
  });

  const onSubmit = useCallback(
    (values: PolicyTestForm) => {
      const payload = buildInput(values);
      mutation.mutate(payload);
    },
    [mutation],
  );

  const applySample = useCallback(
    (key: keyof typeof SAMPLE_ABAC_INPUTS) => {
      const sample = SAMPLE_ABAC_INPUTS[key];
      reset({
        subject: {
          tenant_id: sample.subject.tenant_id,
          user_id: sample.subject.user_id ?? "",
          device_id: sample.subject.device_id ?? "",
          roles: sample.subject.roles?.join(", ") ?? "",
          clearance_level: sample.subject.clearance_level
            ? String(sample.subject.clearance_level)
            : "",
          device_location: sample.subject.device_location ?? "",
        },
        action: sample.action,
        resource: {
          type: sample.resource.type,
          id: sample.resource.id ?? "",
          classification: sample.resource.classification ?? "",
          region: sample.resource.region ?? "",
          owner_tenant: sample.resource.owner_tenant,
          owner_user: sample.resource.owner_user ?? "",
        },
        environment: {
          time: toDateTimeLocal(sample.environment.time),
          country: sample.environment.country ?? "",
          network: sample.environment.network ?? "",
          risk_score: sample.environment.risk_score
            ? String(sample.environment.risk_score)
            : "",
          bandwidth_used: sample.environment.bandwidth_used
            ? String(sample.environment.bandwidth_used)
            : "",
          message_count: sample.environment.message_count
            ? String(sample.environment.message_count)
            : "",
        },
      });
    },
    [reset],
  );

  const clearForm = useCallback(() => {
    const refreshedTime = toDateTimeLocal(new Date().toISOString());
    reset({
      subject: {
        tenant_id: tenantId,
        roles: "",
        clearance_level: "",
      },
      action: "read",
      resource: {
        type: "sensor_data",
        owner_tenant: tenantId,
      },
      environment: {
        time: refreshedTime,
      },
    });
    mutation.reset();
  }, [mutation, reset, tenantId]);

  return (
    <div className="test-simulator">
      <header className="test-simulator__header">
        <h3>
          <TestTube /> Test Simulator
        </h3>
        <div className="test-simulator__actions">
          <button type="button" onClick={() => applySample("eu_allow")}>
            Load EU example
          </button>
          <button type="button" onClick={() => applySample("quota_deny")}>
            Load quota example
          </button>
          <button type="button" onClick={clearForm}>
            <RefreshCw /> Clear
          </button>
        </div>
      </header>

      <form
        className="test-simulator__form"
        onSubmit={handleSubmit(onSubmit)}
      >
        <section>
          <h4>Subject Attributes</h4>
          <div className="field">
            <label htmlFor="subject-tenant">Tenant ID</label>
            <input
              id="subject-tenant"
              {...register("subject.tenant_id")}
              readOnly
            />
            {errors.subject?.tenant_id?.message && (
              <p className="field-hint warning">
                {errors.subject.tenant_id.message}
              </p>
            )}
          </div>

          <div className="field-grid">
            <label>
              User ID
              <input {...register("subject.user_id")} placeholder="alice" />
            </label>
            <label>
              Device ID
              <input {...register("subject.device_id")} placeholder="device-01" />
            </label>
          </div>

          <div className="field-grid">
            <label>
              Roles (comma separated)
              <input {...register("subject.roles")} placeholder="engineer, admin" />
            </label>
            <label>
              Clearance Level
              <input
                {...register("subject.clearance_level")}
                placeholder="3"
              />
            </label>
          </div>

          <div className="field">
            <label>
              Device Location
              <input
                {...register("subject.device_location")}
                placeholder="US"
              />
            </label>
          </div>
        </section>

        <section>
          <h4>Action</h4>
          <div className="field">
            <label>
              Action
              <select {...register("action")}>
                <option value="read">read</option>
                <option value="write">write</option>
                <option value="delete">delete</option>
                <option value="publish">publish</option>
                <option value="subscribe">subscribe</option>
              </select>
            </label>
          </div>
        </section>

        <section>
          <h4>Resource Attributes</h4>
          <div className="field-grid">
            <label>
              Type
              <input {...register("resource.type")} placeholder="sensor_data" />
            </label>
            <label>
              ID
              <input {...register("resource.id")} placeholder="sensor-001" />
            </label>
          </div>

          <div className="field-grid">
            <label>
              Classification
              <input
                {...register("resource.classification")}
                placeholder="confidential"
              />
            </label>
            <label>
              Region
              <input {...register("resource.region")} placeholder="EU" />
            </label>
          </div>

          <div className="field-grid">
            <label>
              Owner Tenant
              <input {...register("resource.owner_tenant")} />
            </label>
            <label>
              Owner User
              <input {...register("resource.owner_user")} placeholder="bob" />
            </label>
          </div>
        </section>

        <section>
          <h4>Environment</h4>
          <div className="field-grid">
            <label>
              Time
              <input type="datetime-local" {...register("environment.time")} />
            </label>
            <label>
              Country
              <input {...register("environment.country")} placeholder="DE" />
            </label>
          </div>

          <div className="field-grid">
            <label>
              Network
              <input {...register("environment.network")} placeholder="vpn" />
            </label>
            <label>
              Risk Score
              <input {...register("environment.risk_score")} placeholder="0.4" />
            </label>
          </div>

          <div className="field-grid">
            <label>
              Bandwidth Used (GB)
              <input
                {...register("environment.bandwidth_used")}
                placeholder="80"
              />
            </label>
            <label>
              Message Count
              <input
                {...register("environment.message_count")}
                placeholder="1500"
              />
            </label>
          </div>
        </section>

        <footer className="test-simulator__footer">
          <button
            type="submit"
            className="primary"
            disabled={mutation.isPending}
          >
            {mutation.isPending ? <Loader2 className="spin" /> : "Test Policy"}
          </button>
        </footer>
      </form>

      {mutation.isError && (
        <div className="test-simulator__result test-simulator__result--error">
          {mutation.error?.message ?? "Policy evaluation failed"}
        </div>
      )}

      {mutation.data && (
        <div
          className={`test-simulator__result ${
            mutation.data.allow
              ? "test-simulator__result--allow"
              : "test-simulator__result--deny"
          }`}
        >
          <h4>
            Decision: {mutation.data.allow ? "ALLOW" : "DENY"}
          </h4>
          {mutation.data.reason && <p>Reason: {mutation.data.reason}</p>}
          {mutation.data.redact && mutation.data.redact.length > 0 && (
            <p>Redact: {mutation.data.redact.join(", ")}</p>
          )}
          {mutation.data.eval_duration_micros !== undefined && (
            <p>
              Evaluation time: {mutation.data.eval_duration_micros} μs
            </p>
          )}
        </div>
      )}
    </div>
  );
}
