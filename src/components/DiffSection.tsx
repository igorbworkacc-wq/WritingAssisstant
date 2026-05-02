import type { TransformSectionState } from "../types/diff";
import { TokenRenderer } from "./TokenRenderer";

interface DiffSectionProps {
  title: string;
  state: TransformSectionState;
  onTokenClick: (tokenId: string) => void;
  onApply: () => void;
  onRetry?: () => void;
  retryDisabled?: boolean;
}

export function DiffSection({
  title,
  state,
  onTokenClick,
  onApply,
  onRetry,
  retryDisabled
}: DiffSectionProps) {
  const canApply =
    state.status === "ready" && state.tokens.length > 0 && state.candidateText.length > 0;

  return (
    <section className="diffSection">
      <div className="sectionHeader">
        <h2>{title}</h2>
        {onRetry ? (
          <button
            type="button"
            className="secondaryButton"
            onClick={onRetry}
            disabled={retryDisabled || state.status === "loading"}
          >
            Retry
          </button>
        ) : null}
      </div>

      {state.status === "loading" ? (
        <div className="sectionState">Working...</div>
      ) : null}

      {state.status === "error" ? (
        <div className="sectionState errorState">{state.errorMessage}</div>
      ) : null}

      {state.status === "ready" ? (
        <TokenRenderer tokens={state.tokens} onTokenClick={onTokenClick} />
      ) : null}

      <div className="sectionActions">
        <button type="button" className="primaryButton" disabled={!canApply} onClick={onApply}>
          Apply
        </button>
      </div>
    </section>
  );
}
