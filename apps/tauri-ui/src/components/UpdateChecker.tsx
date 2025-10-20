import { useCallback, useEffect, useMemo, useState } from "react";
import { check, installUpdate } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Download, RefreshCw, CheckCircle } from "lucide-react";

type UpdateInfo = {
  version: string;
  notes?: string;
  pubDate?: string;
  size?: number;
};

const SKIP_STORAGE_KEY = "edge-policy-hub.skip-version";
const AUTO_CHECK_INTERVAL_MS = 1000 * 60 * 60 * 6; // every 6 hours

const formatBytes = (value?: number) => {
  if (!value || value <= 0) {
    return "Unknown";
  }
  if (value < 1024) {
    return `${value} B`;
  }
  const units = ["KB", "MB", "GB", "TB"];
  let size = value / 1024;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`;
};

const UpdateChecker = () => {
  const [isChecking, setIsChecking] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [bannerVisible, setBannerVisible] = useState(false);
  const [lastCheckedAt, setLastCheckedAt] = useState<Date | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const skippedVersion = useMemo(() => {
    if (typeof window === "undefined") {
      return null;
    }
    return localStorage.getItem(SKIP_STORAGE_KEY);
  }, []);

  const handleCheck = useCallback(async () => {
    if (isChecking || isDownloading) {
      return;
    }
    setIsChecking(true);
    setError(null);
    setMessage(null);
    try {
      const result = await check();
      setLastCheckedAt(new Date());
      if (result?.available) {
        if (result.version && skippedVersion && skippedVersion === result.version) {
          setMessage(`Latest update v${result.version} was skipped.`);
          setBannerVisible(false);
          return;
        }
        setUpdateInfo({
          version: result.version ?? "unknown",
          notes: result.notes ?? undefined,
          pubDate: result.pub_date ?? undefined,
          size: result.size ?? undefined,
        });
        setBannerVisible(true);
      } else {
        setBannerVisible(false);
        setMessage("You're up to date.");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsChecking(false);
    }
  }, [isChecking, isDownloading, skippedVersion]);

  const handleInstall = useCallback(async () => {
    if (!updateInfo) {
      return;
    }
    setIsDownloading(true);
    setError(null);
    setDownloadProgress(0);
    try {
      await installUpdate((event) => {
        if ("percent" in event && typeof event.percent === "number") {
          setDownloadProgress(Math.round(event.percent));
        }
      });
      setMessage("Update installed. Restarting Edge Policy Hub...");
      await relaunch();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsDownloading(false);
    }
  }, [updateInfo]);

  const handleSkip = useCallback(() => {
    if (updateInfo?.version) {
      localStorage.setItem(SKIP_STORAGE_KEY, updateInfo.version);
    }
    setBannerVisible(false);
    setMessage(`Skipped update v${updateInfo?.version ?? ""}.`);
  }, [updateInfo]);

  const handleRemindLater = useCallback(() => {
    setBannerVisible(false);
    setMessage("We'll remind you about this update later.");
  }, []);

  useEffect(() => {
    handleCheck();
    const interval = window.setInterval(handleCheck, AUTO_CHECK_INTERVAL_MS);
    return () => window.clearInterval(interval);
  }, [handleCheck]);

  if (!bannerVisible && !isChecking && !message && !error) {
    return (
      <div className="fixed bottom-4 right-4 z-50 flex flex-col items-end space-y-2">
        <button
          type="button"
          className="flex items-center gap-2 rounded-md border border-border bg-background px-4 py-2 text-sm shadow"
          onClick={handleCheck}
          disabled={isChecking}
        >
          <RefreshCw className={`h-4 w-4 ${isChecking ? "animate-spin" : ""}`} />
          {isChecking ? "Checking..." : "Check for Updates"}
        </button>
        {lastCheckedAt ? (
          <span className="text-xs text-muted-foreground">
            Last checked {lastCheckedAt.toLocaleString()}
          </span>
        ) : null}
      </div>
    );
  }

  return (
    <div className="fixed bottom-4 right-4 z-50 max-w-md rounded-lg border border-border bg-background p-4 shadow-xl">
      {bannerVisible && updateInfo ? (
        <div className="space-y-3">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="flex items-center gap-2 text-base font-semibold">
                <Download className="h-5 w-5 text-primary" />
                Update available: v{updateInfo.version}
              </h2>
              {updateInfo.pubDate ? (
                <p className="text-xs text-muted-foreground">
                  Released {new Date(updateInfo.pubDate).toLocaleString()}
                </p>
              ) : null}
            </div>
          </div>
          {updateInfo.notes ? (
            <div className="max-h-40 overflow-auto rounded bg-muted/40 p-2 text-sm">
              <pre className="whitespace-pre-wrap font-sans text-xs">{updateInfo.notes}</pre>
            </div>
          ) : null}
          <p className="text-xs text-muted-foreground">
            Download size: {formatBytes(updateInfo.size)}
          </p>
          {isDownloading ? (
            <div className="space-y-2">
              <p className="text-sm font-medium">Downloading update… {downloadProgress}%</p>
              <div className="h-2 w-full rounded-full bg-muted">
                <div
                  className="h-2 rounded-full bg-primary transition-all"
                  style={{ width: `${downloadProgress}%` }}
                />
              </div>
            </div>
          ) : null}
          <div className="flex items-center justify-end gap-2">
            <button
              type="button"
              className="rounded-md border border-border px-3 py-2 text-sm"
              onClick={handleRemindLater}
              disabled={isDownloading}
            >
              Remind me later
            </button>
            <button
              type="button"
              className="rounded-md border border-border px-3 py-2 text-sm"
              onClick={handleSkip}
              disabled={isDownloading}
            >
              Skip this version
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-2 rounded-md bg-primary px-3 py-2 text-sm font-semibold text-primary-foreground shadow"
              onClick={handleInstall}
              disabled={isDownloading}
            >
              <Download className="h-4 w-4" />
              {isDownloading ? "Installing…" : "Install now"}
            </button>
          </div>
        </div>
      ) : null}

      {!bannerVisible && message ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <CheckCircle className="h-4 w-4 text-primary" />
          {message}
        </div>
      ) : null}

      {error ? <p className="mt-2 text-sm text-destructive">Update error: {error}</p> : null}

      <div className="mt-4 flex items-center justify-between text-xs text-muted-foreground">
        <button
          type="button"
          className="flex items-center gap-1"
          onClick={handleCheck}
          disabled={isChecking || isDownloading}
        >
          <RefreshCw className={`h-4 w-4 ${isChecking ? "animate-spin" : ""}`} />
          {isChecking ? "Checking…" : "Check again"}
        </button>
        {lastCheckedAt ? (
          <span>Last checked {lastCheckedAt.toLocaleString()}</span>
        ) : null}
      </div>
    </div>
  );
};

export default UpdateChecker;
