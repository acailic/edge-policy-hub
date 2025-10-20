import { useNavigate } from "react-router-dom";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { z } from "zod";
import TenantConfigForm from "../components/TenantConfigForm";
import { createTenant } from "../lib/api";
import type { CreateTenantRequest } from "../types/tenant";

const tenantConfigSchema = z.object({
  quotas: z.object({
    message_limit: z.coerce
      .number({ invalid_type_error: "Message limit must be a number" })
      .min(1, "Message limit must be at least 1"),
    bandwidth_limit_gb: z.coerce
      .number({ invalid_type_error: "Bandwidth limit must be a number" })
      .min(0.1, "Bandwidth limit must be at least 0.1 GB"),
  }),
  features: z.object({
    data_residency: z.array(z.enum(["EU", "US", "APAC"])).default([]),
    pii_redaction: z.boolean().default(false),
  }),
});

const createTenantSchema = tenantConfigSchema.extend({
  tenant_id: z
    .string()
    .min(1, "Tenant ID is required")
    .max(64, "Tenant ID must be at most 64 characters")
    .regex(
      /^[a-zA-Z0-9_-]+$/,
      "Tenant ID may contain letters, numbers, underscores, and hyphens only",
    ),
  name: z
    .string()
    .min(1, "Tenant name is required")
    .max(255, "Tenant name must be at most 255 characters"),
});

type CreateTenantFormValues = z.infer<typeof createTenantSchema>;

function TenantCreatePage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const {
    register,
    control,
    handleSubmit,
    formState: { errors },
  } = useForm<CreateTenantFormValues>({
    resolver: zodResolver(createTenantSchema),
    defaultValues: {
      tenant_id: "",
      name: "",
      quotas: {
        message_limit: 50_000,
        bandwidth_limit_gb: 100,
      },
      features: {
        data_residency: [],
        pii_redaction: false,
      },
    },
  });

  const mutation = useMutation({
    mutationFn: (values: CreateTenantFormValues) => {
      const payload: CreateTenantRequest = {
        tenant_id: values.tenant_id.trim(),
        name: values.name.trim(),
        config: {
          quotas: {
            message_limit: values.quotas.message_limit,
            bandwidth_limit_gb: values.quotas.bandwidth_limit_gb,
          },
          features: {
            data_residency: values.features.data_residency,
            pii_redaction: values.features.pii_redaction,
          },
        },
      };

      return createTenant(payload);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tenants"] });
      navigate("/");
    },
  });

  const onSubmit = handleSubmit((values) => {
    mutation.mutate(values);
  });

  return (
    <div className="card">
      <div className="page-header">
        <div>
          <h2>Create Tenant</h2>
          <p className="helper-text">
            Register a new tenant and configure quotas, features, and residency
            requirements.
          </p>
        </div>
      </div>

      <form className="form-grid" onSubmit={onSubmit}>
        <div className="form-section">
          <div className="field">
            <label htmlFor="tenant_id">Tenant ID</label>
            <input id="tenant_id" type="text" {...register("tenant_id")} />
            {errors.tenant_id && (
              <span className="error-text">{errors.tenant_id.message}</span>
            )}
          </div>

          <div className="field">
            <label htmlFor="name">Tenant Name</label>
            <input id="name" type="text" {...register("name")} />
            {errors.name && (
              <span className="error-text">{errors.name.message}</span>
            )}
          </div>
        </div>

        <TenantConfigForm<CreateTenantFormValues>
          register={register}
          control={control}
          errors={errors}
        />

        {mutation.isError && (
          <span className="error-text">
            Failed to create tenant: {(mutation.error as Error).message}
          </span>
        )}

        <div className="actions">
          <button
            className="button secondary"
            type="button"
            onClick={() => navigate(-1)}
            disabled={mutation.isPending}
          >
            Cancel
          </button>
          <button
            className="button primary"
            type="submit"
            disabled={mutation.isPending}
          >
            {mutation.isPending ? "Creating…" : "Create Tenant"}
          </button>
        </div>
      </form>
    </div>
  );
}

export default TenantCreatePage;
