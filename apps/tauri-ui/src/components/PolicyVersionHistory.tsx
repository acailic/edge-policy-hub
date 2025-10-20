import { useCallback } from "react";
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  Eye,
  Loader2,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
} from "lucide-react";

import {
  activatePolicyBundle,
  listPolicyBundles,
  rollbackPolicy,
} from "../lib/api";
import type { PolicyBundle } from "../types/policy";

interface PolicyVersionHistoryProps {
  tenantId: string;
  onSelectVersion?: (bundle: PolicyBundle) => void;
}

function formatTimestamp(value?: string | null) {
  if (!value) {
    return "—";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function statusBadge(status: PolicyBundle["status"]) {
  switch (status) {
    case "active":
      return <span className="badge badge--active">Active</span>;
    case "draft":
      return <span className="badge badge--draft">Draft</span>;
    case "inactive":
      return <span className="badge badge--inactive">Inactive</span>;
    case "archived":
    default:
      return <span className="badge badge--archived">Archived</span>;
  }
}

export function PolicyVersionHistory({
  tenantId,
  onSelectVersion,
}: PolicyVersionHistoryProps) {
  const queryClient = useQueryClient();
  const {
    data: bundles,
    isLoading,
    isError,
    error,
    refetch,
  } = useQuery({
    queryKey: ["policy-bundles", tenantId],
    queryFn: () => listPolicyBundles(tenantId),
  });

  const activateMutation = useMutation({
    mutationFn: (bundleId: string) => activatePolicyBundle(bundleId),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["policy-bundles", tenantId] }),
  });

  const rollbackMutation = useMutation({
    mutationFn: (bundleId: string) => rollbackPolicy(tenantId, bundleId),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["policy-bundles", tenantId] }),
  });

  const handleActivate = useCallback(
    (bundle: PolicyBundle) => {
      if (bundle.status === "active") {
        return;
      }

      const confirmed = window.confirm(
        `Activate version v${bundle.version} for ${tenantId}?`,
      );
      if (!confirmed) {
        return;
      }

      activateMutation.mutate(bundle.bundle_id);
    },
    [activateMutation, tenantId],
  );

  const handleRollback = useCallback(
    (bundle: PolicyBundle) => {
      const confirmed = window.confirm(
        `Rollback tenant ${tenantId} to bundle ${bundle.bundle_id}?`,
      );
      if (!confirmed) {
        return;
      }

      rollbackMutation.mutate(bundle.bundle_id);
    },
    [rollbackMutation, tenantId],
  );

  if (isLoading) {
    return (
      <div className="policy-version-history policy-version-history--loading">
        <Loader2 className="spin" /> Loading policy history…
      </div>
    );
  }

  if (isError) {
    return (
      <div className="policy-version-history policy-version-history--error">
        {error instanceof Error ? error.message : "Failed to load versions"}
        <button type="button" onClick={() => refetch()}>
          Retry
        </button>
      </div>
    );
  }

  if (!bundles || bundles.length === 0) {
    return (
      <div className="policy-version-history policy-version-history--empty">
        <p>No policy bundles found for tenant {tenantId}.</p>
      </div>
    );
  }

  const activeBundle = bundles.find((bundle) => bundle.status === "active");

  return (
    <div className="policy-version-history">
      <header className="policy-version-history__header">
        <h3>
          <ShieldCheck /> Version History
        </h3>
        <button
          type="button"
          onClick={() =>
            queryClient.invalidateQueries({
              queryKey: ["policy-bundles", tenantId],
            })
          }
        >
          <RefreshCw /> Refresh
        </button>
      </header>

      {activeBundle && (
        <div className="policy-version-history__active">
          <strong>Active version:</strong> v{activeBundle.version} ·{" "}
          {activeBundle.metadata?.description ?? "No description provided"}
        </div>
      )}

      <table>
        <thead>
          <tr>
            <th>Version</th>
            <th>Status</th>
            <th>Author</th>
            <th>Created</th>
            <th>Activated</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {bundles.map((bundle) => (
            <tr
              key={bundle.bundle_id}
              className={
                bundle.status === "active" ? "row--active" : undefined
              }
            >
              <td>v{bundle.version}</td>
              <td>{statusBadge(bundle.status)}</td>
              <td>{bundle.metadata?.author ?? "—"}</td>
              <td>{formatTimestamp(bundle.created_at)}</td>
              <td>{formatTimestamp(bundle.activated_at)}</td>
              <td className="actions">
                <button
                  type="button"
                  onClick={() => onSelectVersion?.(bundle)}
                  title="View bundle"
                >
                  <Eye />
                </button>
                <button
                  type="button"
                  onClick={() => handleActivate(bundle)}
                  disabled={
                    bundle.status === "active" || activateMutation.isPending
                  }
                  title="Activate bundle"
                >
                  <ShieldCheck />
                </button>
                <button
                  type="button"
                  onClick={() => handleRollback(bundle)}
                  disabled={rollbackMutation.isPending}
                  title="Rollback to this version"
                >
                  <RotateCcw />
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
