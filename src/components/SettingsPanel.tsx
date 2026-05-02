import { FormEvent, useEffect, useRef, useState } from "react";

interface SettingsPanelProps {
  configured: boolean;
  saving: boolean;
  testing: boolean;
  error?: string;
  testMessage?: string;
  forceOpen: boolean;
  onSave: (apiKey: string) => Promise<boolean>;
  onTest: () => Promise<void>;
  onReplace: () => void;
}

export function SettingsPanel({
  configured,
  saving,
  testing,
  error,
  testMessage,
  forceOpen,
  onSave,
  onTest,
  onReplace
}: SettingsPanelProps) {
  const [showForm, setShowForm] = useState(forceOpen || !configured);
  const [apiKey, setApiKey] = useState("");
  const [localError, setLocalError] = useState<string>();
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (forceOpen || !configured) {
      setShowForm(true);
    } else {
      setShowForm(false);
    }
  }, [configured, forceOpen]);

  useEffect(() => {
    if (showForm) {
      inputRef.current?.focus();
    }
  }, [showForm]);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const value = apiKey.trim();
    if (!value) {
      setLocalError("OpenAI API key is missing.");
      return;
    }

    setLocalError(undefined);
    const saved = await onSave(value);
    if (saved) {
      setApiKey("");
    }
  }

  if (!showForm && configured) {
    return (
      <div className="settingsInline">
        <span>{testMessage ?? "API key configured. Click Test API key to verify access."}</span>
        <div className="settingsActions">
          <button type="button" className="secondaryButton" onClick={onReplace}>
            Replace API key
          </button>
          <button type="button" className="secondaryButton" onClick={onTest} disabled={testing}>
            Test API key
          </button>
        </div>
      </div>
    );
  }

  return (
    <form className="settingsPanel" onSubmit={handleSubmit}>
      <h2>OpenAI API Key</h2>
      <p>
        An API key is required before correction and rephrase can work. The key is saved through
        Windows credential storage and is never stored in frontend storage.
      </p>
      <input
        ref={inputRef}
        type="password"
        autoComplete="off"
        disabled={saving}
        onPaste={() => undefined}
        spellCheck={false}
        value={apiKey}
        onChange={(event) => setApiKey(event.target.value)}
        placeholder="Paste API key"
      />
      {localError || error ? <div className="errorState">{localError ?? error}</div> : null}
      {testMessage ? <div className="successState">{testMessage}</div> : null}
      <button type="submit" className="primaryButton" disabled={saving || apiKey.trim().length === 0}>
        Save API Key
      </button>
      <button type="button" className="secondaryButton" onClick={onTest} disabled={!configured || testing}>
        Test API key
      </button>
    </form>
  );
}
