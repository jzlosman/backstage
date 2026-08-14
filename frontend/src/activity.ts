import type { SourceTimestamp } from "./api";

export type RecencyGroup = "Today" | "Past 7 days" | "Older" | "Date unavailable";

export interface DatedRecord {
  id: string;
  sourceModifiedUnixNanos: SourceTimestamp;
}

export function compareDatedRecords(left: DatedRecord, right: DatedRecord) {
  const leftTime = validSourceNanoseconds(left.sourceModifiedUnixNanos);
  const rightTime = validSourceNanoseconds(right.sourceModifiedUnixNanos);
  if (leftTime === null && rightTime !== null) return 1;
  if (leftTime !== null && rightTime === null) return -1;
  if (leftTime !== rightTime) return rightTime! > leftTime! ? 1 : -1;
  return left.id.localeCompare(right.id);
}

export function groupDatedRecords<T extends DatedRecord>(records: T[], now: Date) {
  const groups = new Map<RecencyGroup, T[]>();
  for (const record of records) {
    const label = recencyGroup(record.sourceModifiedUnixNanos, now);
    const group = groups.get(label) ?? [];
    group.push(record);
    groups.set(label, group);
  }
  return (["Today", "Past 7 days", "Older", "Date unavailable"] as const).flatMap((label) => {
    const group = groups.get(label);
    return group?.length ? [{ label, records: group }] : [];
  });
}

export function recencyGroup(unixNanos: SourceTimestamp, now: Date): RecencyGroup {
  const milliseconds = validSourceMilliseconds(unixNanos);
  if (milliseconds === null) return "Date unavailable";
  const date = new Date(milliseconds);
  const calendarDay = (value: Date) =>
    Date.UTC(value.getFullYear(), value.getMonth(), value.getDate()) / 86_400_000;
  const daysAgo = calendarDay(now) - calendarDay(date);
  if (daysAgo === 0) return "Today";
  if (daysAgo >= 1 && daysAgo <= 7) return "Past 7 days";
  return "Older";
}

export function validSourceMilliseconds(unixNanos: SourceTimestamp) {
  const nanoseconds = validSourceNanoseconds(unixNanos);
  if (nanoseconds === null) return null;
  const milliseconds = Number(nanoseconds / 1_000_000n);
  return Number.isNaN(new Date(milliseconds).valueOf()) ? null : milliseconds;
}

function validSourceNanoseconds(unixNanos: SourceTimestamp): bigint | null {
  if (typeof unixNanos === "string") {
    if (!/^\d+$/.test(unixNanos)) return null;
    return BigInt(unixNanos);
  }
  if (typeof unixNanos === "number" && Number.isFinite(unixNanos) && unixNanos >= 0) {
    return BigInt(Math.trunc(unixNanos));
  }
  return null;
}
