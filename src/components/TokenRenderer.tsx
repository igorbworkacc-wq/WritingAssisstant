import type { DiffToken } from "../types/diff";

interface TokenRendererProps {
  tokens: DiffToken[];
  onTokenClick: (tokenId: string) => void;
}

export function TokenRenderer({ tokens, onTokenClick }: TokenRendererProps) {
  return (
    <div className="tokenRenderer" aria-label="Interactive diff text">
      {tokens.map((token) => {
        const text = token.selectedSide === "candidate" ? token.candidateText : token.originalText;
        const emptyCandidate = token.candidateText.length === 0 && token.selectedSide === "candidate";
        const emptyOriginal = token.originalText.length === 0 && token.selectedSide === "original";
        const displayText = emptyCandidate
          ? token.originalText
          : emptyOriginal
            ? token.candidateText || " "
            : text;

        if (!token.clickable) {
          return (
            <span key={token.id} className="token tokenEqual">
              {text}
            </span>
          );
        }

        const className = [
          "token",
          "tokenModified",
          `token-${token.kind}`,
          token.selectedSide === "original" ? "tokenOriginal" : "tokenCandidate",
          emptyCandidate ? "tokenDeletionPlaceholder" : "",
          emptyOriginal ? "tokenInsertionPlaceholder" : ""
        ]
          .filter(Boolean)
          .join(" ");

        return (
          <button
            key={token.id}
            type="button"
            className={className}
            onClick={() => onTokenClick(token.id)}
            title="Toggle this token"
            aria-pressed={token.selectedSide === "candidate"}
          >
            {displayText}
          </button>
        );
      })}
    </div>
  );
}
