import { useMemo, useState } from "react";
import {
  AlertTriangle,
  CheckCircle,
  FileCode2,
  Loader2,
} from "lucide-react";

import type { CompilationError } from "../types/policy";

interface CompilationStatusBarProps {
  isCompiling: boolean;
  errors: CompilationError[];
  lastCompiled?: Date | null;
  compiledRego?: string | null;
  onFocusError?: (error: CompilationError) => void;
}

export function CompilationStatusBar({
  isCompiling,
  errors,
  lastCompiled,
  compiledRego,
  onFocusError,
}: CompilationStatusBarProps) {
  const [showErrors, setShowErrors] = useState(false);

  const status = useMemo(() => {
    if (isCompiling) {
      return {
        icon: <Loader2 className="spin" />,
        message: "Compiling policy…",
        tone: "info",
      };
    }

    if (errors.length > 0) {
      return {
        icon: <AlertTriangle />,
        message: `${errors.length} error${errors.length === 1 ? "" : "s"} found`,
        tone: "error",
      };
    }

    return {
      icon: <CheckCircle />,
      message: "Compiled successfully",
      tone: "success",
    };
  }, [errors.length, isCompiling]);

  const regoSize = useMemo(() => {
    if (!compiledRego) {
      return null;
    }

    const encoder = new TextEncoder();
    const bytes = encoder.encode(compiledRego).length;
    if (bytes < 1024) {
      return `${bytes} B`;
    }

    return `${(bytes / 1024).toFixed(1)} KB`;
  }, [compiledRego]);

  const lastCompiledLabel = useMemo(() => {
    if (!lastCompiled) {
      return null;
    }

    const now = Date.now();
    const delta = now - lastCompiled.getTime();

    if (delta < 60 * 1000) {
      return "just now";
    }

    const minutes = Math.round(delta / (60 * 1000));
    if (minutes < 60) {
      return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;
    }

    const hours = Math.round(minutes / 60);
    if (hours < 24) {
      return `${hours} hour${hours === 1 ? "" : "s"} ago`;
    }

    const days = Math.round(hours / 24);
    return `${days} day${days === 1 ? "" : "s"} ago`;
  }, [lastCompiled]);

  return (
    <div className={`compilation-status compilation-status--${status.tone}`}>
      <div className="compilation-status__summary">
        <span className="compilation-status__icon">{status.icon}</span>
        <span>{status.message}</span>
        {lastCompiledLabel && (
          <span className="compilation-status__meta">
            Last compiled {lastCompiledLabel}
          </span>
        )}
        {regoSize && (
          <span className="compilation-status__meta">
            <FileCode2 size={14} /> {regoSize}
          </span>
        )}
      </div>

      {errors.length > 0 && (
        <div className="compilation-status__errors">
          <button
            type="button"
            onClick={() => setShowErrors((prev) => !prev)}
            aria-expanded={showErrors}
          >
            {showErrors ? "Hide errors" : "Show errors"}
          </button>
          {showErrors && (
            <ul>
              {errors.map((error, index) => (
                <li key={`${error.message}-${index}`}>
                  <button
                    type="button"
                    onClick={() => onFocusError?.(error)}
                    className="compilation-status__error"
                  >
                    {error.line ? `Line ${error.line}: ` : ""}
                    {error.message}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
