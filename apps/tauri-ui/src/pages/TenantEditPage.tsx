import { useEffect } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { z } from "zod";
import TenantConfigForm from "../components/TenantConfigForm";
import { getTenant, updateTenant } from "../lib/api";
import type { TenantStatus, UpdateTenantRequest } from "../types/tenant";

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

const editTenantSchema = tenantConfigSchema.extend({
  name: z
    .string()
    .min(1, "Tenant name is required")
    .max(255, "Tenant name must be at most 255 characters"),
  status: z.enum(["active", "suspended", "deleted"]),
});

type EditTenantFormValues = z.infer<typeof editTenantSchema>;

function TenantEditPage() {
  const { id: tenantId } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const tenantQuery = useQuery({
    queryKey: ["tenant", tenantId],
    queryFn: () => getTenant(tenantId ?? ""),
    enabled: Boolean(tenantId),
  });

  const {
    register,
    control,
    handleSubmit,
    formState: { errors },
    reset,
  } = useForm<EditTenantFormValues>({
    resolver: zodResolver(editTenantSchema),
    defaultValues: {
      name: "",
      status: "active",
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

  useEffect(() => {
    if (tenantQuery.data) {
      const tenant = tenantQuery.data;
      reset({
        name: tenant.name,
        status: tenant.status as TenantStatus,
        quotas: {
          message_limit: tenant.config?.quotas?.message_limit ?? 50_000,
          bandwidth_limit_gb:
            tenant.config?.quotas?.bandwidth_limit_gb ?? 100,
        },
        features: {
          data_residency: tenant.config?.features?.data_residency ?? [],
          pii_redaction: tenant.config?.features?.pii_redaction ?? false,
        },
      });
    }
  }, [tenantQuery.data, reset]);

  const mutation = useMutation({
    mutationFn: (values: EditTenantFormValues) => {
      if (!tenantId) {
        throw new Error("Missing tenant identifier");
      }

      const payload: UpdateTenantRequest = {
        name: values.name.trim(),
        status: values.status,
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

      return updateTenant(tenantId, payload);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tenants"] });
      if (tenantId) {
        queryClient.invalidateQueries({ queryKey: ["tenant", tenantId] });
      }
      navigate("/");
    },
  });

  const onSubmit = handleSubmit((values) => {
    mutation.mutate(values);
  });

  if (tenantQuery.isLoading) {
    return <p>Loading tenant…</p>;
  }

  if (tenantQuery.isError) {
    return (
      <p className="error-text">
        Failed to load tenant: {(tenantQuery.error as Error).message}
      </p>
    );
  }

  if (!tenantQuery.data) {
    return <p className="error-text">Tenant not found.</p>;
  }

  return (
    <div className="card">
      <div className="page-header">
        <div>
          <h2>Edit Tenant</h2>
          <p className="helper-text">
            Update tenant details, lifecycle status, and configuration.
          </p>
        </div>
      </div>

      <form className="form-grid" onSubmit={onSubmit}>
        <div className="form-section">
          <div className="field">
            <label htmlFor="name">Tenant Name</label>
            <input id="name" type="text" {...register("name")} />
            {errors.name && (
              <span className="error-text">{errors.name.message}</span>
            )}
          </div>

          <div className="field">
            <label htmlFor="status">Status</label>
            <select id="status" {...register("status")}>
              <option value="active">Active</option>
              <option value="suspended">Suspended</option>
              <option value="deleted">Deleted</option>
            </select>
            {errors.status && (
              <span className="error-text">{errors.status.message}</span>
            )}
          </div>
        </div>

        <TenantConfigForm<EditTenantFormValues>
          register={register}
          control={control}
          errors={errors}
        />

        {mutation.isError && (
          <span className="error-text">
            Failed to update tenant: {(mutation.error as Error).message}
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
            {mutation.isPending ? "Saving…" : "Save Changes"}
          </button>
        </div>
      </form>
    </div>
  );
}

export default TenantEditPage;
