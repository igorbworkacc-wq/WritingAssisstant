import type { OperationState } from "../types/diff";
import { DiffSection } from "./DiffSection";
import { SettingsPanel } from "./SettingsPanel";

interface PopupProps {
  state: OperationState;
  hasApiKey: boolean;
  savingKey: boolean;
  keyError?: string;
  onSaveKey: (apiKey: string) => Promise<void>;
  onClose: () => void;
  onTokenClick: (section: "correction" | "rephrase", tokenId: string) => void;
  onApply: (section: "correction" | "rephrase") => void;
  onRetryRephrase: () => void;
}

export function Popup({
  state,
  hasApiKey,
  savingKey,
  keyError,
  onSaveKey,
  onClose,
  onTokenClick,
  onApply,
  onRetryRephrase
}: PopupProps) {
  const hasOperation = Boolean(state.operationId);

  return (
    <main className="appShell">
      <header className="appHeader">
        <div>
          <h1>PrivacyTextAssistant</h1>
          <p>Ctrl+Space</p>
        </div>
        <button type="button" className="iconButton" onClick={onClose} aria-label="Close">
          x
        </button>
      </header>

      <SettingsPanel
        configured={hasApiKey}
        saving={savingKey}
        error={keyError}
        onSave={onSaveKey}
      />

      {state.operationError ? <div className="topError">{state.operationError}</div> : null}

      {hasOperation ? (
        <>
          <details className="originalPreview">
            <summary>Original text</summary>
            <pre>{state.originalText}</pre>
          </details>

          <div className="sectionsGrid">
            <DiffSection
              title="Corrected Text"
              state={state.correction}
              onTokenClick={(tokenId) => onTokenClick("correction", tokenId)}
              onApply={() => onApply("correction")}
            />
            <DiffSection
              title="Rephrased Text"
              state={state.rephrase}
              onTokenClick={(tokenId) => onTokenClick("rephrase", tokenId)}
              onApply={() => onApply("rephrase")}
              onRetry={onRetryRephrase}
              retryDisabled={!hasApiKey || !state.operationId}
            />
          </div>
        </>
      ) : (
        <div className="emptyState">
          Highlight text in another Windows app, then press Ctrl+Space.
        </div>
      )}
    </main>
  );
}
