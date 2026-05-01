import { describe, expect, it } from "vitest";
import { buildDiffTokens } from "../lib/diffTokens";
import { reconstructText, toggleToken } from "../lib/textReconstruction";

describe("text reconstruction", () => {
  it("uses token state rather than global word replacement", () => {
    const tokens = buildDiffTokens("test test test", "test test", "correction");
    const deleted = tokens.find((token) => token.clickable && token.originalText.includes("test"));

    expect(deleted).toBeDefined();
    const reverted = toggleToken(tokens, deleted!.id);

    expect(reconstructText(reverted)).toBe("test test test");
  });
});
