const legacyDateParts = 9;

export function formatDateTime(
  value: unknown,
  options: Intl.DateTimeFormatOptions = defaultDateTimeOptions,
): string {
  const date = parseWireDate(value);
  if (!date) {
    return "Time unavailable";
  }

  try {
    return new Intl.DateTimeFormat(undefined, options).format(date);
  } catch {
    return "Time unavailable";
  }
}

export function parseWireDate(value: unknown): Date | null {
  if (typeof value === "string" || typeof value === "number") {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  if (!Array.isArray(value) || value.length < legacyDateParts) {
    return null;
  }

  const parts = value.map(Number);
  if (parts.some((part) => !Number.isFinite(part))) {
    return null;
  }

  const [
    year,
    ordinal,
    hour,
    minute,
    second,
    nanosecond,
    offsetHour,
    offsetMinute,
    offsetSecond,
  ] = parts;
  const offsetMilliseconds =
    (offsetHour * 60 * 60 + offsetMinute * 60 + offsetSecond) * 1000;
  const timestamp =
    Date.UTC(year, 0, ordinal, hour, minute, second, nanosecond / 1_000_000) -
    offsetMilliseconds;
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? null : date;
}

const defaultDateTimeOptions: Intl.DateTimeFormatOptions = {
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
};
