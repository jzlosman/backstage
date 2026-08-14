import { describe, expect, it } from "vitest";

import { compareDatedRecords, groupDatedRecords, recencyGroup } from "./activity";
import type { SourceTimestamp } from "./api";

const nanos = (date: Date) => date.getTime() * 1_000_000;

interface Record {
  id: string;
  sourceModifiedUnixNanos: SourceTimestamp;
}

describe("activity chronology", () => {
  it("sorts newest first, puts unavailable dates last, and breaks ties by identity", () => {
    const records: Record[] = [
      { id: "unknown", sourceModifiedUnixNanos: null },
      { id: "b", sourceModifiedUnixNanos: nanos(new Date(2026, 7, 14, 9)) },
      { id: "a", sourceModifiedUnixNanos: nanos(new Date(2026, 7, 14, 9)) },
      { id: "newest", sourceModifiedUnixNanos: nanos(new Date(2026, 7, 14, 10)) },
    ];

    expect(records.sort(compareDatedRecords).map((record) => record.id)).toEqual([
      "newest",
      "a",
      "b",
      "unknown",
    ]);
  });

  it("sorts decimal-string nanoseconds losslessly and leaves null last", () => {
    const records: Record[] = [
      { id: "unknown", sourceModifiedUnixNanos: null },
      { id: "a-older", sourceModifiedUnixNanos: "1786712400000000000" },
      { id: "z-newer", sourceModifiedUnixNanos: "1786712400000000001" },
    ];

    expect(records.sort(compareDatedRecords).map((record) => record.id)).toEqual([
      "z-newer",
      "a-older",
      "unknown",
    ]);
  });

  it("treats both numeric and decimal-string epoch timestamps as available", () => {
    const now = new Date(2026, 7, 14, 12);

    expect(recencyGroup(0, now)).toBe("Older");
    expect(recencyGroup("0", now)).toBe("Older");
  });

  it("uses local calendar boundaries across midnight and daylight-saving weeks", () => {
    const beforeMidnight = new Date(2026, 2, 8, 23, 59);
    const afterMidnight = new Date(2026, 2, 9, 0, 1);

    expect(recencyGroup(nanos(beforeMidnight), beforeMidnight)).toBe("Today");
    expect(recencyGroup(nanos(beforeMidnight), afterMidnight)).toBe("Past 7 days");
    expect(recencyGroup(nanos(new Date(2026, 2, 2, 12)), afterMidnight)).toBe("Past 7 days");
    expect(recencyGroup(nanos(new Date(2026, 2, 1, 23, 59)), afterMidnight)).toBe("Older");
    expect(recencyGroup(null, afterMidnight)).toBe("Date unavailable");
  });

  it("omits empty groups while preserving chronological group order", () => {
    const now = new Date(2026, 7, 14, 12);
    const groups = groupDatedRecords(
      [
        { id: "unknown", sourceModifiedUnixNanos: null },
        { id: "today", sourceModifiedUnixNanos: nanos(new Date(2026, 7, 14, 9)) },
        { id: "older", sourceModifiedUnixNanos: nanos(new Date(2026, 6, 1, 9)) },
      ],
      now,
    );

    expect(groups.map((group) => group.label)).toEqual(["Today", "Older", "Date unavailable"]);
  });
});
