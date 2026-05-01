import { describe, expect, it } from "vitest";
import { buildDiffTokens, modifiedTokens, reconstructText, toggleToken } from "../lib/diffTokens";

describe("buildDiffTokens", () => {
  it("handles no change", () => {
    const tokens = buildDiffTokens("This is correct.", "This is correct.", "correction");

    expect(tokens.every((token) => !token.clickable)).toBe(true);
    expect(reconstructText(tokens)).toBe("This is correct.");
  });

  it("handles simple replacement", () => {
    const tokens = buildDiffTokens("I has a pen.", "I have a pen.", "correction");
    const changed = modifiedTokens(tokens).find((token) => token.originalText.includes("has"));

    expect(changed).toBeDefined();
    expect(changed?.candidateText).toContain("have");
    expect(reconstructText(tokens)).toBe("I have a pen.");

    const reverted = toggleToken(tokens, changed!.id);
    expect(reconstructText(reverted)).toBe("I has a pen.");
    expect(reconstructText(toggleToken(reverted, changed!.id))).toBe("I have a pen.");
  });

  it("handles insertion", () => {
    const tokens = buildDiffTokens(
      "Please review document.",
      "Please review the document.",
      "rephrase"
    );
    const inserted = modifiedTokens(tokens).find((token) => token.candidateText.includes("the"));

    expect(inserted?.kind).toBe("insert");
    expect(reconstructText(tokens)).toBe("Please review the document.");
    expect(reconstructText(toggleToken(tokens, inserted!.id))).toBe("Please review document.");
  });

  it("handles deletion", () => {
    const tokens = buildDiffTokens(
      "Please kindly review the document.",
      "Please review the document.",
      "correction"
    );
    const deleted = modifiedTokens(tokens).find((token) => token.originalText.includes("kindly"));

    expect(deleted?.kind).toBe("delete");
    expect(reconstructText(tokens)).toBe("Please review the document.");
    expect(reconstructText(toggleToken(tokens, deleted!.id))).toBe(
      "Please kindly review the document."
    );
  });

  it("handles repeated words without toggling all occurrences", () => {
    const tokens = buildDiffTokens("The test test passed.", "The test passed.", "correction");
    const deleted = modifiedTokens(tokens).find((token) => token.originalText.includes("test"));

    expect(deleted).toBeDefined();
    const reverted = toggleToken(tokens, deleted!.id);
    expect(reconstructText(reverted)).toBe("The test test passed.");
    expect(modifiedTokens(reverted).filter((token) => token.selectedSide === "original")).toHaveLength(1);
  });

  it("handles punctuation", () => {
    const tokens = buildDiffTokens("Hello Igor", "Hello, Igor.", "correction");

    expect(reconstructText(tokens)).toBe("Hello, Igor.");
    expect(reconstructText(toggleAllModified(tokens))).toBe("Hello Igor");
  });

  it("handles newlines", () => {
    const original = "Hi Igor,\nplease review.";
    const candidate = "Hi Igor,\n\nPlease review.";
    const tokens = buildDiffTokens(original, candidate, "rephrase");

    expect(reconstructText(tokens)).toBe(candidate);
    expect(reconstructText(toggleAllModified(tokens))).toBe(original);
  });

  it("reconstructs the original when every clickable token is toggled", () => {
    const original = "Please kindly review document.";
    const candidate = "Please review the document.";
    const tokens = buildDiffTokens(original, candidate, "rephrase");

    expect(reconstructText(toggleAllModified(tokens))).toBe(original);
  });

  it("reconstructs the full candidate initially", () => {
    const candidate = "Please review the document.";
    const tokens = buildDiffTokens("Please kindly review document.", candidate, "rephrase");

    expect(reconstructText(tokens)).toBe(candidate);
  });
});

function toggleAllModified(tokens: ReturnType<typeof buildDiffTokens>) {
  return modifiedTokens(tokens).reduce((nextTokens, token) => toggleToken(nextTokens, token.id), tokens);
}
