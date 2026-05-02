import { FormEvent, useEffect, useMemo, useState } from "react";
import type { ModelPreset, ModelSettings } from "../types/model";

const CUSTOM_MODEL = "custom";

interface ModelSettingsPanelProps {
  settings: ModelSettings;
  presets: ModelPreset[];
  forceOpen: boolean;
  saving: boolean;
  testing: boolean;
  error?: string;
  message?: string;
  apiKeyConfigured: boolean;
  onSave: (settings: ModelSettings) => Promise<void>;
  onTest: () => Promise<void>;
}

export function ModelSettingsPanel({
  settings,
  presets,
  forceOpen,
  saving,
  testing,
  error,
  message,
  apiKeyConfigured,
  onSave,
  onTest
}: ModelSettingsPanelProps) {
  const presetIds = useMemo(() => new Set(presets.map((preset) => preset.id)), [presets]);
  const [open, setOpen] = useState(forceOpen);
  const [selectedOption, setSelectedOption] = useState(
    presetIds.has(settings.selected_model) ? settings.selected_model : CUSTOM_MODEL
  );
  const [customModel, setCustomModel] = useState(
    presetIds.has(settings.selected_model) ? "" : settings.selected_model
  );
  const [temperature, setTemperature] = useState(settings.temperature.toString());

  useEffect(() => {
    setOpen(forceOpen);
  }, [forceOpen]);

  useEffect(() => {
    const isPreset = presetIds.has(settings.selected_model);
    setSelectedOption(isPreset ? settings.selected_model : CUSTOM_MODEL);
    setCustomModel(isPreset ? "" : settings.selected_model);
    setTemperature(settings.temperature.toString());
  }, [presetIds, settings.selected_model, settings.temperature]);

  const modelToSave = selectedOption === CUSTOM_MODEL ? customModel.trim() : selectedOption;
  const parsedTemperature = Number.parseFloat(temperature);
  const validTemperature = Number.isFinite(parsedTemperature) && parsedTemperature >= 0 && parsedTemperature <= 2;
  const canSave = modelToSave.length > 0 && validTemperature;

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!canSave) {
      return;
    }

    await onSave({
      selected_model: modelToSave,
      temperature: parsedTemperature
    });
  }

  if (!open) {
    return (
      <div className="settingsInline">
        <span>{message ?? `Model: ${settings.selected_model}`}</span>
        <div className="settingsActions">
          <button type="button" className="secondaryButton" onClick={() => setOpen(true)}>
            Model settings
          </button>
          <button
            type="button"
            className="secondaryButton"
            onClick={onTest}
            disabled={!apiKeyConfigured || testing}
          >
            Test model
          </button>
        </div>
      </div>
    );
  }

  return (
    <form className="settingsPanel" onSubmit={handleSubmit}>
      <h2>OpenAI model</h2>
      <p>The selected model is used for both correction and rephrase calls.</p>

      <label className="fieldLabel">
        Model
        <select
          value={selectedOption}
          onChange={(event) => setSelectedOption(event.target.value)}
        >
          {presets.map((preset) => (
            <option key={preset.id} value={preset.id}>
              {preset.id === CUSTOM_MODEL ? preset.label : `${preset.label} - ${preset.id}`}
            </option>
          ))}
        </select>
      </label>

      {selectedOption === CUSTOM_MODEL ? (
        <label className="fieldLabel">
          Custom model ID
          <input
            type="text"
            autoComplete="off"
            spellCheck={false}
            value={customModel}
            onChange={(event) => setCustomModel(event.target.value)}
            placeholder="Enter model ID"
          />
        </label>
      ) : null}

      {selectedOption !== CUSTOM_MODEL ? (
        <div className="modelDescription">
          {presets.find((preset) => preset.id === selectedOption)?.description}
        </div>
      ) : null}

      <details className="advancedSettings">
        <summary>Advanced</summary>
        <p>Some models may reject temperature. If that happens, the app retries once without it.</p>
        <label className="fieldLabel">
          Temperature
          <input
            type="number"
            min="0"
            max="2"
            step="0.1"
            value={temperature}
            onChange={(event) => setTemperature(event.target.value)}
          />
        </label>
      </details>

      {error ? <div className="errorState">{error}</div> : null}
      {message ? <div className="successState">{message}</div> : null}

      <div className="settingsActions">
        <button type="submit" className="primaryButton" disabled={saving || !canSave}>
          Save model
        </button>
        <button
          type="button"
          className="secondaryButton"
          onClick={onTest}
          disabled={!apiKeyConfigured || testing || modelToSave.length === 0}
        >
          Test selected model
        </button>
      </div>
    </form>
  );
}
