import { diffWordsWithSpace } from "diff";
import type { Change } from "diff";
import type { DiffToken, DiffTokenKind, SectionKind } from "../types/diff";

type TokenDraft = {
  kind: DiffTokenKind;
  originalText: string;
  candidateText: string;
  groupId?: string;
};

export function buildDiffTokens(
  originalText: string,
  candidateText: string,
  section: SectionKind
): DiffToken[] {
  const changes = diffWordsWithSpace(originalText, candidateText);
  const drafts: TokenDraft[] = [];

  for (let i = 0; i < changes.length; i += 1) {
    const change = changes[i];
    const next = changes[i + 1];

    if (change.removed && next?.added) {
      drafts.push(...buildReplacementDrafts(change.value, next.value, drafts.length));
      i += 1;
      continue;
    }

    if (change.added) {
      drafts.push(...tokenizeEditableChunk(change.value).map((value) => ({
        kind: "insert" as const,
        originalText: "",
        candidateText: value
      })));
      continue;
    }

    if (change.removed) {
      drafts.push(...tokenizeEditableChunk(change.value).map((value) => ({
        kind: "delete" as const,
        originalText: value,
        candidateText: ""
      })));
      continue;
    }

    drafts.push({
      kind: "equal",
      originalText: change.value,
      candidateText: change.value
    });
  }

  return drafts.map((draft, index) => toToken(draft, section, index));
}

export function toggleToken(tokens: DiffToken[], tokenId: string): DiffToken[] {
  return tokens.map((token) => {
    if (token.id !== tokenId || !token.clickable) {
      return token;
    }

    return {
      ...token,
      selectedSide: token.selectedSide === "candidate" ? "original" : "candidate"
    };
  });
}

export function reconstructText(tokens: DiffToken[]): string {
  return tokens
    .map((token) => token.selectedSide === "candidate" ? token.candidateText : token.originalText)
    .join("");
}

function buildReplacementDrafts(
  removedValue: string,
  addedValue: string,
  groupSeed: number
): TokenDraft[] {
  const removedTokens = tokenizeEditableChunk(removedValue);
  const addedTokens = tokenizeEditableChunk(addedValue);
  const max = Math.max(removedTokens.length, addedTokens.length);
  const groupId = `replace-${groupSeed}`;
  const drafts: TokenDraft[] = [];

  for (let i = 0; i < max; i += 1) {
    const originalText = removedTokens[i] ?? "";
    const candidateText = addedTokens[i] ?? "";

    if (originalText === candidateText) {
      drafts.push({
        kind: "equal",
        originalText,
        candidateText
      });
    } else {
      drafts.push({
        kind: "replace",
        originalText,
        candidateText,
        groupId
      });
    }
  }

  return drafts;
}

function toToken(draft: TokenDraft, section: SectionKind, index: number): DiffToken {
  const modified = draft.kind !== "equal";

  return {
    id: `${section}-${index}`,
    section,
    index,
    kind: draft.kind,
    originalText: draft.originalText,
    candidateText: draft.candidateText,
    selectedSide: "candidate",
    clickable: modified,
    highlighted: modified,
    groupId: draft.groupId
  };
}

function tokenizeEditableChunk(value: string): string[] {
  if (!value) {
    return [];
  }

  const raw = value.match(/\s+|[^\s]+/g) ?? [value];
  const tokens: string[] = [];
  let leadingWhitespace = "";

  for (const part of raw) {
    if (/^\s+$/.test(part)) {
      if (tokens.length === 0) {
        leadingWhitespace += part;
      } else {
        tokens[tokens.length - 1] += part;
      }
    } else {
      tokens.push(`${leadingWhitespace}${part}`);
      leadingWhitespace = "";
    }
  }

  if (leadingWhitespace) {
    tokens.push(leadingWhitespace);
  }

  return tokens;
}

export function modifiedTokens(tokens: DiffToken[]): DiffToken[] {
  return tokens.filter((token) => token.clickable);
}

export type { Change };
