import { useCallback, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";

import { deployPolicy } from "../lib/api";
import type { DeployPolicyResponse, PolicyMetadata } from "../types/policy";

export function usePolicyDeployment(tenantId: string) {
  const queryClient = useQueryClient();
  const [deploymentResult, setDeploymentResult] =
    useState<DeployPolicyResponse | null>(null);
  const [deploymentError, setDeploymentError] = useState<Error | null>(null);

  const mutation = useMutation({
    mutationFn: async ({
      regoCode,
      metadata,
      activate,
    }: {
      regoCode: string;
      metadata: PolicyMetadata;
      activate: boolean;
    }) => deployPolicy(tenantId, regoCode, metadata, activate),
    onSuccess: async (result) => {
      setDeploymentResult(result);
      setDeploymentError(null);
      await queryClient.invalidateQueries({
        queryKey: ["policy-bundles", tenantId],
      });
    },
    onError: (error: Error) => {
      setDeploymentResult(null);
      setDeploymentError(error);
    },
  });

  const deploy = useCallback(
    async (regoCode: string, metadata: PolicyMetadata, activate: boolean) => {
      return mutation.mutateAsync({ regoCode, metadata, activate });
    },
    [mutation],
  );

  return {
    deploy,
    isDeploying: mutation.isPending,
    deploymentResult,
    deploymentError,
  };
}
