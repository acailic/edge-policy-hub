import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Activity, Shield } from "lucide-react";
import { deleteTenant, listTenants } from "../lib/api";
import type { TenantStatus } from "../types/tenant";

const STATUS_OPTIONS: Array<{ label: string; value?: TenantStatus }> = [
  { label: "All" },
  { label: "Active", value: "active" },
  { label: "Suspended", value: "suspended" },
  { label: "Deleted", value: "deleted" },
];

function TenantListPage() {
  const [statusFilter, setStatusFilter] = useState<TenantStatus | undefined>();
  const queryClient = useQueryClient();

  const { data, isLoading, isError, error, refetch, isFetching } = useQuery({
    queryKey: ["tenants", statusFilter],
    queryFn: () => listTenants(statusFilter),
  });

  const deleteMutation = useMutation({
    mutationFn: deleteTenant,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tenants"] });
    },
  });

  const handleDelete = (tenantId: string) => {
    if (
      !window.confirm(
        "Are you sure you want to delete this tenant? This action cannot be undone.",
      )
    ) {
      return;
    }

    deleteMutation.mutate(tenantId);
  };

  return (
    <div className="card">
      <div className="page-header">
        <div>
          <h2>Registered Tenants</h2>
          <p className="helper-text">
            Manage tenants, quotas, and feature availability.
          </p>
        </div>
        <div className="actions">
          <button
            className="button secondary"
            type="button"
            onClick={() => refetch()}
            disabled={isFetching}
          >
            Refresh
          </button>
          <Link className="button primary" to="/tenants/new">
            Add New Tenant
          </Link>
        </div>
      </div>

      <div className="field" style={{ maxWidth: "240px", marginBottom: "16px" }}>
        <label htmlFor="tenant-status-filter">Status Filter</label>
        <select
          id="tenant-status-filter"
          value={statusFilter ?? ""}
          onChange={(event) => {
            const value = event.target.value;
            setStatusFilter(value === "" ? undefined : (value as TenantStatus));
          }}
        >
          {STATUS_OPTIONS.map((option) => (
            <option key={option.label} value={option.value ?? ""}>
              {option.label}
            </option>
          ))}
        </select>
      </div>

      {isLoading ? (
        <p>Loading tenants…</p>
      ) : isError ? (
        <p className="error-text">Failed to load tenants: {error?.message}</p>
      ) : data && data.length === 0 ? (
        <p>No tenants found for the selected filter.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Tenant ID</th>
              <th>Name</th>
              <th>Status</th>
              <th>Created</th>
              <th>Updated</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {data?.map((tenant) => (
              <tr key={tenant.tenant_id}>
                <td>{tenant.tenant_id}</td>
                <td>{tenant.name}</td>
                <td>{tenant.status}</td>
                <td>{new Date(tenant.created_at).toLocaleString()}</td>
                <td>{new Date(tenant.updated_at).toLocaleString()}</td>
                <td>
                  <div className="actions">
                    <Link
                      className="button secondary"
                      to={`/tenants/${tenant.tenant_id}/edit`}
                    >
                      Edit
                    </Link>
                    <Link
                      className="button"
                      to={`/tenants/${tenant.tenant_id}/policies`}
                    >
                      <Shield size={16} /> Policies
                    </Link>
                    <Link
                      className="button secondary"
                      to={`/tenants/${tenant.tenant_id}/monitor`}
                    >
                      <Activity size={16} /> Monitor
                    </Link>
                    <button
                      className="button danger"
                      type="button"
                      onClick={() => handleDelete(tenant.tenant_id)}
                      disabled={deleteMutation.isPending}
                    >
                      Delete
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {deleteMutation.isError && (
        <p className="error-text" style={{ marginTop: "16px" }}>
          Failed to delete tenant: {(deleteMutation.error as Error).message}
        </p>
      )}
    </div>
  );
}

export default TenantListPage;
