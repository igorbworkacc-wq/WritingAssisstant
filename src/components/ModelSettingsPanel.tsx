import { FormEvent, useEffect, useMemo, useState } from "react";
import type { ModelAvailabilityStatus, ModelPreset, ModelSettings, ModelTestStatus } from "../types/model";

const CUSTOM_MODEL = "custom";
const FALLBACK_MODEL = "gpt-4o-mini";

interface ModelSettingsPanelProps {
  settings: ModelSettings;
  presets: ModelPreset[];
  forceOpen: boolean;
  saving: boolean;
  testing: boolean;
  error?: string;
  message?: string;
  availableModels: string[] | null;
  refreshingModels: boolean;
  availability: ModelAvailabilityStatus;
  testStatus: ModelTestStatus;
  apiKeyConfigured: boolean;
  onSave: (settings: ModelSettings) => Promise<void>;
  onTest: () => Promise<void>;
  onRefreshAvailableModels: () => Promise<void>;
}

export function ModelSettingsPanel({
  settings,
  presets,
  forceOpen,
  saving,
  testing,
  error,
  message,
  availableModels,
  refreshingModels,
  availability,
  testStatus,
  apiKeyConfigured,
  onSave,
  onTest,
  onRefreshAvailableModels
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
  const displayedAvailability = availableModels ? availabilityFor(modelToSave, availableModels) : availability;

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

  function useFallbackModel() {
    setSelectedOption(FALLBACK_MODEL);
    setCustomModel("");
  }

  function useAvailableModel(model: string) {
    if (presetIds.has(model)) {
      setSelectedOption(model);
      setCustomModel("");
      return;
    }

    setSelectedOption(CUSTOM_MODEL);
    setCustomModel(model);
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
              {preset.id === CUSTOM_MODEL
                ? preset.label
                : `${preset.label} - ${preset.id} - ${availabilityLabel(
                    availabilityFor(preset.id, availableModels)
                  )}`}
            </option>
          ))}
        </select>
      </label>

      <div className="modelStatusGrid">
        <span>Selected model: {modelToSave || "none"}</span>
        <span>Availability: {availabilityLabel(displayedAvailability)}</span>
        <span>Last model test: {testStatusLabel(testStatus)}</span>
      </div>

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

      <details className="availableModels">
        <summary>Available models from this API key/project</summary>
        <div className="settingsActions">
          <button
            type="button"
            className="secondaryButton"
            onClick={onRefreshAvailableModels}
            disabled={!apiKeyConfigured || refreshingModels}
          >
            Refresh available models
          </button>
        </div>
        {availableModels ? (
          <>
            <p>{availableModels.length} model IDs available.</p>
            <select
              value=""
              onChange={(event) => {
                if (event.target.value) {
                  useAvailableModel(event.target.value);
                }
              }}
            >
              <option value="">Choose an available model</option>
              {availableModels.map((model) => (
                <option key={model} value={model}>
                  {model}
                </option>
              ))}
            </select>
          </>
        ) : (
          <p>Availability has not been checked yet.</p>
        )}
      </details>

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
        <button type="button" className="secondaryButton" onClick={useFallbackModel}>
          Use GPT-4o mini
        </button>
      </div>
    </form>
  );
}

function availabilityFor(model: string, availableModels: string[] | null): ModelAvailabilityStatus {
  if (!availableModels) {
    return "not_checked";
  }

  return availableModels.includes(model) ? "available" : "not_found";
}

function availabilityLabel(status: ModelAvailabilityStatus) {
  switch (status) {
    case "available":
      return "available";
    case "not_found":
      return "not found in available model list";
    default:
      return "not checked yet";
  }
}

function testStatusLabel(status: ModelTestStatus) {
  switch (status) {
    case "successful":
      return "successful";
    case "failed":
      return "failed";
    default:
      return "not tested";
  }
}
