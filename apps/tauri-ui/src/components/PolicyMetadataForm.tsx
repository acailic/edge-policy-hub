import { useCallback, useMemo, useState } from "react";
import { username } from "@tauri-apps/api/os";

import type { PolicyMetadata } from "../types/policy";

interface PolicyMetadataFormProps {
  metadata: PolicyMetadata;
  onChange: (metadata: PolicyMetadata) => void;
}

const SEMVER_PATTERN = /^\d+\.\d+\.\d+$/;
const MAX_DESCRIPTION_LENGTH = 500;

export function PolicyMetadataForm({
  metadata,
  onChange,
}: PolicyMetadataFormProps) {
  const [autoFillError, setAutoFillError] = useState<string | null>(null);

  const versionIsValid = useMemo(
    () => SEMVER_PATTERN.test(metadata.version),
    [metadata.version],
  );

  const descriptionLength = metadata.description?.length ?? 0;

  const updateMetadata = useCallback(
    (partial: Partial<PolicyMetadata>) => {
      onChange({
        ...metadata,
        ...partial,
      });
    },
    [metadata, onChange],
  );

  const handleAutoFill = useCallback(async () => {
    setAutoFillError(null);
    try {
      const systemUsername = await username();
      if (!systemUsername) {
        throw new Error("Unable to determine system user");
      }

      updateMetadata({
        author: systemUsername,
      });
    } catch (error) {
      setAutoFillError(
        error instanceof Error ? error.message : "Failed to read system user",
      );
    }
  }, [updateMetadata]);

  return (
    <div className="policy-metadata-form">
      <div className="field">
        <label htmlFor="policy-version">Version</label>
        <input
          id="policy-version"
          name="version"
          value={metadata.version}
          onChange={(event) =>
            updateMetadata({ version: event.target.value.trim() })
          }
          placeholder="1.0.0"
          aria-invalid={!versionIsValid}
        />
        {!versionIsValid && (
          <p className="field-hint">
            Use semantic versioning format (e.g., 1.2.3).
          </p>
        )}
      </div>

      <div className="field">
        <label htmlFor="policy-author">Author</label>
        <div className="field-inline">
          <input
            id="policy-author"
            name="author"
            value={metadata.author ?? ""}
            onChange={(event) => updateMetadata({ author: event.target.value })}
            placeholder="operator@example.com"
            type="email"
          />
          <button type="button" onClick={handleAutoFill}>
            Auto-fill
          </button>
        </div>
        {autoFillError && (
          <p className="field-hint warning">{autoFillError}</p>
        )}
      </div>

      <div className="field">
        <label htmlFor="policy-description">
          Description
          <span className="char-count">
            {descriptionLength}/{MAX_DESCRIPTION_LENGTH}
          </span>
        </label>
        <textarea
          id="policy-description"
          name="description"
          value={metadata.description ?? ""}
          onChange={(event) =>
            updateMetadata({ description: event.target.value })
          }
          maxLength={MAX_DESCRIPTION_LENGTH}
          rows={4}
          placeholder="Summarise the intent and scope of this policy."
        />
      </div>

      <div className="field">
        <label htmlFor="policy-created-at">Created</label>
        <input
          id="policy-created-at"
          value={metadata.created_at}
          readOnly
        />
      </div>
    </div>
  );
}
