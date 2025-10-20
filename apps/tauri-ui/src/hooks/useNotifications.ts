import { useCallback, useEffect, useState } from "react";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

interface NotificationOptions {
  icon?: string;
  body?: string;
}

export function useNotifications() {
  const [permissionGranted, setPermissionGranted] = useState(false);
  const [permissionRequested, setPermissionRequested] = useState(false);

  useEffect(() => {
    let isMounted = true;

    async function ensurePermission() {
      try {
        const granted = await isPermissionGranted();
        if (!isMounted) {
          return;
        }

        if (granted) {
          setPermissionGranted(true);
          return;
        }

        if (!permissionRequested) {
          setPermissionRequested(true);
          const result = await requestPermission();
          if (!isMounted) {
            return;
          }
          setPermissionGranted(result === "granted");
        }
      } catch (error) {
        console.warn("Failed to request notification permission", error);
      }
    }

    void ensurePermission();

    return () => {
      isMounted = false;
    };
  }, [permissionRequested]);

  const notify = useCallback(
    async (title: string, body: string, options?: NotificationOptions) => {
      if (!permissionGranted) {
        console.warn(
          "Notification permission not granted. Skipping notification",
          { title, body },
        );
        return;
      }

      try {
        await sendNotification({
          title,
          body,
          ...options,
        });
      } catch (error) {
        console.error("Failed to deliver notification", error);
      }
    },
    [permissionGranted],
  );

  const notifyQuotaWarning = useCallback(
    (tenantId: string, quotaType: string, percentage: number) => {
      const formattedPercentage = Math.round(percentage);
      void notify(
        `Quota Warning: ${tenantId}`,
        `${quotaType} usage at ${formattedPercentage}%`,
        {
          icon: "warning",
        },
      );
    },
    [notify],
  );

  const notifyPolicyViolation = useCallback(
    (tenantId: string, action: string, reason?: string) => {
      const body = reason
        ? `Action '${action}' denied: ${reason}`
        : `Action '${action}' denied`;
      void notify(`Policy Violation: ${tenantId}`, body, {
        icon: "error",
      });
    },
    [notify],
  );

  return {
    permissionGranted,
    notify,
    notifyQuotaWarning,
    notifyPolicyViolation,
  };
}
