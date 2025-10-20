import { useCallback, useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";

import { compilePolicyDsl } from "../lib/api";
import type {
  CompilationError,
  CompilePolicyResponse,
  PolicyMetadata,
} from "../types/policy";

interface CompileArgs {
  metadata?: PolicyMetadata;
}

export function usePolicyCompilation(
  tenantId: string,
  initialSource = "",
) {
  const [dslSource, setDslSourceState] = useState(initialSource);
  const [compiledRego, setCompiledRego] = useState<string | null>(null);
  const [compilationErrors, setCompilationErrors] = useState<CompilationError[]>([]);
  const [lastCompiled, setLastCompiled] = useState<Date | null>(null);

  const mutation = useMutation({
    mutationFn: async ({
      source,
      metadata,
    }: {
      source: string;
      metadata?: PolicyMetadata;
    }) => compilePolicyDsl(source, tenantId, metadata),
    onSuccess: (result: CompilePolicyResponse) => {
      if (result.success) {
        setCompiledRego(result.rego ?? null);
        setCompilationErrors([]);
        setLastCompiled(new Date());
      } else {
        setCompiledRego(null);
        setCompilationErrors(result.errors ?? []);
        setLastCompiled(null);
      }
    },
    onError: () => {
      setCompiledRego(null);
      setCompilationErrors([
        {
          message: "Failed to compile policy. See logs for details.",
        },
      ]);
      setLastCompiled(null);
    },
  });

  const setSource = useCallback((value: string) => {
    setDslSourceState(value);
    setCompiledRego(null);
    setCompilationErrors([]);
    setLastCompiled(null);
  }, []);

  const compile = useCallback(
    async ({ metadata }: CompileArgs = {}) => {
      return mutation.mutateAsync({
        source: dslSource,
        metadata,
      });
    },
    [dslSource, mutation],
  );

  const hasErrors = useMemo(
    () => compilationErrors.length > 0,
    [compilationErrors],
  );

  return {
    dslSource,
    setSource,
    compiledRego,
    compilationErrors,
    isCompiling: mutation.isPending,
    lastCompiled,
    compile,
    hasErrors,
  };
}
