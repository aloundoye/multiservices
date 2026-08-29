import { describe, expect, it } from "vitest";
import { formatMoney, label, signed } from "./format";

describe("formatting helpers", () => {
  it("formats whole FCFA amounts", () => {
    expect(formatMoney(5_000_000)).toContain("5");
    expect(formatMoney(5_000_000)).toContain("FCFA");
  });

  it("keeps a visible sign for positive variances", () => {
    expect(signed(10_000)).toMatch(/^\+/);
    expect(signed(-10_000)).toMatch(/^-/);
  });

  it("translates accounting labels", () => {
    expect(label("orange_money")).toBe("Orange Money");
    expect(label("expense")).toBe("Dépense");
  });
});
