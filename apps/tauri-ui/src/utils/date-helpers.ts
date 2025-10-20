import {
  format,
  formatDistanceToNow,
  parseISO,
  subHours,
  subDays,
} from "date-fns";

export function formatTimestamp(
  isoString: string,
  formatString: string = "MMM dd, yyyy HH:mm:ss",
): string {
  try {
    return format(parseISO(isoString), formatString);
  } catch {
    return isoString;
  }
}

export function formatRelativeTime(isoString: string): string {
  try {
    return formatDistanceToNow(parseISO(isoString), { addSuffix: true });
  } catch {
    return isoString;
  }
}

export function formatDateRange(
  startTime?: string,
  endTime?: string,
): string {
  if (!startTime && !endTime) {
    return "Any time";
  }

  if (startTime && !endTime) {
    return `From ${formatTimestamp(startTime, "MMM dd, yyyy HH:mm")}`;
  }

  if (!startTime && endTime) {
    return `Until ${formatTimestamp(endTime, "MMM dd, yyyy HH:mm")}`;
  }

  return `${formatTimestamp(
    startTime as string,
    "MMM dd, yyyy HH:mm",
  )} → ${formatTimestamp(endTime as string, "MMM dd, yyyy HH:mm")}`;
}

export function getCurrentTimestamp(): string {
  return new Date().toISOString();
}

export function getDateRangePresets(): {
  label: string;
  start: string;
  end: string;
}[] {
  const now = new Date();
  return [
    {
      label: "Last Hour",
      start: subHours(now, 1).toISOString(),
      end: now.toISOString(),
    },
    {
      label: "Last 24 Hours",
      start: subHours(now, 24).toISOString(),
      end: now.toISOString(),
    },
    {
      label: "Last 7 Days",
      start: subDays(now, 7).toISOString(),
      end: now.toISOString(),
    },
    {
      label: "Last 30 Days",
      start: subDays(now, 30).toISOString(),
      end: now.toISOString(),
    },
    {
      label: "Custom",
      start: "",
      end: "",
    },
  ];
}
