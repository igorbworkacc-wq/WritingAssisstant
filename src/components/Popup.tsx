import type { OperationState } from "../types/diff";
import type { ModelPreset, ModelSettings } from "../types/model";
import { DiffSection } from "./DiffSection";
import { ModelSettingsPanel } from "./ModelSettingsPanel";
import { SettingsPanel } from "./SettingsPanel";

interface PopupProps {
  state: OperationState;
  hasApiKey: boolean;
  savingKey: boolean;
  testingKey: boolean;
  keyError?: string;
  keyTestMessage?: string;
  settingsMode: boolean;
  modelSettingsMode: boolean;
  modelSettings: ModelSettings;
  modelPresets: ModelPreset[];
  savingModel: boolean;
  testingModel: boolean;
  modelError?: string;
  modelMessage?: string;
  onSaveKey: (apiKey: string) => Promise<boolean>;
  onTestKey: () => Promise<void>;
  onShowSettings: () => void;
  onShowModelSettings: () => void;
  onSaveModel: (settings: ModelSettings) => Promise<void>;
  onTestStoredModel: () => Promise<void>;
  onHideToTray: () => void;
  onQuit: () => void;
  onClose: () => void;
  onTokenClick: (section: "correction" | "rephrase", tokenId: string) => void;
  onApply: (section: "correction" | "rephrase") => void;
  onRetryRephrase: () => void;
}

export function Popup({
  state,
  hasApiKey,
  savingKey,
  testingKey,
  keyError,
  keyTestMessage,
  settingsMode,
  modelSettingsMode,
  modelSettings,
  modelPresets,
  savingModel,
  testingModel,
  modelError,
  modelMessage,
  onSaveKey,
  onTestKey,
  onShowSettings,
  onShowModelSettings,
  onSaveModel,
  onTestStoredModel,
  onHideToTray,
  onQuit,
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
        testing={testingKey}
        error={keyError}
        testMessage={keyTestMessage}
        forceOpen={settingsMode || !hasApiKey}
        onSave={onSaveKey}
        onTest={onTestKey}
        onReplace={onShowSettings}
      />

      <ModelSettingsPanel
        settings={modelSettings}
        presets={modelPresets}
        forceOpen={modelSettingsMode || !hasApiKey}
        saving={savingModel}
        testing={testingModel}
        error={modelError}
        message={modelMessage}
        apiKeyConfigured={hasApiKey}
        onSave={onSaveModel}
        onTest={onTestStoredModel}
      />

      {state.operationError ? <div className="topError">{state.operationError}</div> : null}

      {hasOperation ? (
        <>
          <details className="originalPreview">
            <summary>Original text</summary>
            <pre>{state.originalText}</pre>
          </details>
          <div className="operationMeta">Using model: {state.model}</div>

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
        <div className="readyPanel">
          <div>
            <h2>Ready</h2>
            <p>
              {hasApiKey
                ? "Highlight text in any app and press Ctrl+Space."
                : "An OpenAI API key is required before correction and rephrase can work."}
            </p>
            <div className="statusMeta">
              <span>API key: {hasApiKey ? "configured" : "missing"}</span>
              <span>Model: {modelSettings.selected_model}</span>
            </div>
          </div>
          <div className="readyActions">
            <button type="button" className="secondaryButton" onClick={onShowSettings}>
              Replace API key
            </button>
            <button type="button" className="secondaryButton" onClick={onShowModelSettings}>
              Model settings
            </button>
            <button
              type="button"
              className="secondaryButton"
              onClick={onTestKey}
              disabled={!hasApiKey || testingKey}
            >
              Test API key
            </button>
            <button
              type="button"
              className="secondaryButton"
              onClick={onTestStoredModel}
              disabled={!hasApiKey || testingModel}
            >
              Test model
            </button>
            <button type="button" className="secondaryButton" onClick={onHideToTray}>
              Hide to tray
            </button>
            <button type="button" className="secondaryButton" onClick={onQuit}>
              Quit
            </button>
          </div>
        </div>
      )}
    </main>
  );
}
