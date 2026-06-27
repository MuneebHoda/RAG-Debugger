import { describe, expect, it } from "vitest";

import { formatDateTime, parseWireDate } from "./dateTime";

describe("wire date handling", () => {
  it("parses RFC3339 timestamps", () => {
    expect(parseWireDate("2026-06-27T10:46:19.985489Z")?.toISOString()).toBe(
      "2026-06-27T10:46:19.985Z",
    );
  });

  it("parses legacy time arrays", () => {
    expect(
      parseWireDate([
        2026, 178, 10, 46, 19, 985_489_000, 0, 0, 0,
      ])?.toISOString(),
    ).toBe("2026-06-27T10:46:19.985Z");
  });

  it("returns a stable fallback for malformed values", () => {
    expect(formatDateTime("not-a-date")).toBe("Time unavailable");
  });
});
