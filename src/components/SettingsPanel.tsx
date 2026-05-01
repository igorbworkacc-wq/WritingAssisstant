import { FormEvent, useState } from "react";

interface SettingsPanelProps {
  configured: boolean;
  saving: boolean;
  error?: string;
  onSave: (apiKey: string) => Promise<void>;
}

export function SettingsPanel({ configured, saving, error, onSave }: SettingsPanelProps) {
  const [showForm, setShowForm] = useState(!configured);
  const [apiKey, setApiKey] = useState("");

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const value = apiKey;
    setApiKey("");
    await onSave(value);
    setShowForm(false);
  }

  if (!showForm && configured) {
    return (
      <div className="settingsInline">
        <span>API key configured</span>
        <button type="button" className="secondaryButton" onClick={() => setShowForm(true)}>
          Replace API key
        </button>
      </div>
    );
  }

  return (
    <form className="settingsPanel" onSubmit={handleSubmit}>
      <h2>OpenAI API Key</h2>
      <p>The key is saved through Windows credential storage and is never stored in frontend storage.</p>
      <input
        type="password"
        autoComplete="off"
        spellCheck={false}
        value={apiKey}
        onChange={(event) => setApiKey(event.target.value)}
        placeholder="Paste API key"
      />
      {error ? <div className="errorState">{error}</div> : null}
      <button type="submit" className="primaryButton" disabled={saving || apiKey.trim().length === 0}>
        Save API Key
      </button>
    </form>
  );
}
