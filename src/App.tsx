import { useEffect, useReducer, useRef, useState } from "react";
import type { Dispatch, MutableRefObject } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { buildDiffTokens, reconstructText, toggleToken } from "./lib/diffTokens";
import { Popup } from "./components/Popup";
import type { OperationErrorPayload, OperationStartedPayload } from "./types/tauri";
import type { OperationState, SectionKind, TransformSectionState } from "./types/diff";
import type { ModelAvailabilityStatus, ModelPreset, ModelSettings, ModelTestStatus } from "./types/model";

type Action =
  | {
      type: "operation-started";
      payload: OperationStartedPayload;
      correctionRequestId: number;
      rephraseRequestId: number;
    }
  | {
      type: "operation-error";
      message: string;
    }
  | {
      type: "section-started";
      operationId: string;
      section: SectionKind;
      requestId: number;
    }
  | {
      type: "section-success";
      operationId: string;
      section: SectionKind;
      requestId: number;
      candidateText: string;
    }
  | {
      type: "section-error";
      operationId: string;
      section: SectionKind;
      requestId: number;
      message: string;
    }
  | {
      type: "toggle-token";
      section: SectionKind;
      tokenId: string;
    }
  | {
      type: "clear";
    };

const initialState: OperationState = {
  operationId: null,
  originalText: "",
  model: "gpt-5-nano",
  targetCaptured: false,
  correction: emptySection("correction"),
  rephrase: emptySection("rephrase")
};

function emptySection(section: SectionKind): TransformSectionState {
  return {
    section,
    status: "idle",
    originalText: "",
    candidateText: "",
    tokens: [],
    requestId: 0
  };
}

function reducer(state: OperationState, action: Action): OperationState {
  switch (action.type) {
    case "operation-started":
      return {
        operationId: action.payload.operationId,
        originalText: action.payload.originalText,
        model: action.payload.model,
        targetCaptured: action.payload.targetCaptured,
        correction: {
          ...emptySection("correction"),
          status: "loading",
          originalText: action.payload.originalText,
          requestId: action.correctionRequestId
        },
        rephrase: {
          ...emptySection("rephrase"),
          status: "loading",
          originalText: action.payload.originalText,
          requestId: action.rephraseRequestId
        },
        operationError: undefined
      };

    case "operation-error":
      return {
        ...state,
        operationError: action.message
      };

    case "section-started":
      if (state.operationId !== action.operationId) {
        return state;
      }
      return {
        ...state,
        [action.section]: {
          ...state[action.section],
          status: "loading",
          errorMessage: undefined,
          candidateText: "",
          tokens: [],
          requestId: action.requestId
        }
      };

    case "section-success":
      if (
        state.operationId !== action.operationId ||
        state[action.section].requestId !== action.requestId
      ) {
        return state;
      }
      return {
        ...state,
        [action.section]: {
          ...state[action.section],
          status: "ready",
          candidateText: action.candidateText,
          tokens: buildDiffTokens(state.originalText, action.candidateText, action.section),
          errorMessage: undefined
        }
      };

    case "section-error":
      if (
        state.operationId !== action.operationId ||
        state[action.section].requestId !== action.requestId
      ) {
        return state;
      }
      return {
        ...state,
        [action.section]: {
          ...state[action.section],
          status: "error",
          errorMessage: action.message,
          candidateText: "",
          tokens: []
        }
      };

    case "toggle-token":
      return {
        ...state,
        [action.section]: {
          ...state[action.section],
          tokens: toggleToken(state[action.section].tokens, action.tokenId)
        }
      };

    case "clear":
      return initialState;

    default:
      return state;
  }
}

export default function App() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const [hasApiKey, setHasApiKey] = useState(false);
  const [savingKey, setSavingKey] = useState(false);
  const [testingKey, setTestingKey] = useState(false);
  const [keyError, setKeyError] = useState<string>();
  const [keyTestMessage, setKeyTestMessage] = useState<string>();
  const [settingsMode, setSettingsMode] = useState(false);
  const [modelSettingsMode, setModelSettingsMode] = useState(false);
  const [modelSettings, setModelSettings] = useState<ModelSettings>({
    selected_model: "gpt-5-nano",
    temperature: 1.0
  });
  const [modelPresets, setModelPresets] = useState<ModelPreset[]>([]);
  const [savingModel, setSavingModel] = useState(false);
  const [testingModel, setTestingModel] = useState(false);
  const [modelError, setModelError] = useState<string>();
  const [modelMessage, setModelMessage] = useState<string>();
  const [availableModels, setAvailableModels] = useState<string[] | null>(null);
  const [refreshingModels, setRefreshingModels] = useState(false);
  const [modelAvailability, setModelAvailability] = useState<ModelAvailabilityStatus>("not_checked");
  const [modelTestStatus, setModelTestStatus] = useState<ModelTestStatus>("not_tested");
  const requestCounter = useRef(0);

  useEffect(() => {
    void refreshApiKeyStatus().then((configured) => {
      setSettingsMode(!configured);
    });
    void refreshModelSettings().catch(() => {
      setModelSettings({
        selected_model: "gpt-5-nano",
        temperature: 1.0
      });
    });
    void invoke<ModelPreset[]>("get_model_presets")
      .then(setModelPresets)
      .catch(() => setModelPresets([]));
  }, []);

  useEffect(() => {
    const unlistenStarted = listen<OperationStartedPayload>("operation-started", (event) => {
      const correctionRequestId = nextRequestId(requestCounter);
      const rephraseRequestId = nextRequestId(requestCounter);

      dispatch({
        type: "operation-started",
        payload: event.payload,
        correctionRequestId,
        rephraseRequestId
      });

      void runTransform(
        "correction",
        event.payload.operationId,
        event.payload.originalText,
        correctionRequestId,
        dispatch,
        handleTransformConfigurationError
      );
      void runTransform(
        "rephrase",
        event.payload.operationId,
        event.payload.originalText,
        rephraseRequestId,
        dispatch,
        handleTransformConfigurationError
      );
    });

    const unlistenError = listen<OperationErrorPayload>("operation-error", (event) => {
      dispatch({ type: "operation-error", message: event.payload.message });
    });

    const unlistenSettings = listen("show-settings", () => {
      setSettingsMode(true);
      setModelSettingsMode(true);
    });

    return () => {
      void unlistenStarted.then((unlisten) => unlisten());
      void unlistenError.then((unlisten) => unlisten());
      void unlistenSettings.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const appWindow = getCurrentWindow();
    const unlistenResized = appWindow.onResized(async () => {
      if (await appWindow.isMinimized()) {
        await invoke("hide_to_tray");
      }
    });

    return () => {
      void unlistenResized.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (isEditableTarget(event.target)) {
        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        void handleClose();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  async function handleSaveKey(apiKey: string) {
    setSavingKey(true);
    setKeyError(undefined);
    setKeyTestMessage(undefined);
    const trimmedKey = apiKey.trim();
    if (!trimmedKey) {
      setKeyError("OpenAI API key is missing.");
      setSavingKey(false);
      return false;
    }

    try {
      await invoke("set_api_key", { apiKey: trimmedKey });
      const configured = await refreshApiKeyStatus();
      if (!configured) {
        throw new Error("OpenAI API key is missing.");
      }
      setKeyError(undefined);
      setKeyTestMessage("API key configured. Click Test API key to verify access.");
      setSettingsMode(false);
      return true;
    } catch (error) {
      setHasApiKey(false);
      setSettingsMode(true);
      setKeyError(errorMessage(error));
      return false;
    } finally {
      setSavingKey(false);
    }
  }

  async function handleTestKey() {
    setTestingKey(true);
    setKeyError(undefined);
    setKeyTestMessage(undefined);
    try {
      await invoke("test_api_key");
      await refreshApiKeyStatus();
      setKeyTestMessage("API key configured and verified.");
    } catch (error) {
      const message = errorMessage(error);
      setKeyError(message);
      setKeyTestMessage(undefined);
      setSettingsMode(true);
      if (message === "OpenAI API key is missing.") {
        setHasApiKey(false);
      }
    } finally {
      setTestingKey(false);
    }
  }

  async function handleSaveModel(nextSettings: ModelSettings) {
    setSavingModel(true);
    setModelError(undefined);
    setModelMessage(undefined);
    try {
      await invoke("set_model_settings", { settings: nextSettings });
      const confirmed = await refreshModelSettings();
      const availability = availabilityForModel(confirmed.selected_model, availableModels);
      setModelAvailability(availability);
      setModelTestStatus("not_tested");
      setModelMessage(`Model settings saved: ${confirmed.selected_model}`);
      setModelSettingsMode(false);
    } catch (error) {
      setModelError(errorMessage(error));
      setModelSettingsMode(true);
    } finally {
      setSavingModel(false);
    }
  }

  async function handleTestStoredModel() {
    setTestingModel(true);
    setModelError(undefined);
    setModelMessage(undefined);
    try {
      await invoke("test_selected_model");
      setModelAvailability("available");
      setModelTestStatus("successful");
      setModelMessage("Model test successful.");
    } catch (error) {
      const message = errorMessage(error);
      setModelTestStatus("failed");
      if (
        message ===
        "The selected model is not available for this API key. Choose another model from the available models list."
      ) {
        setModelAvailability("not_found");
        setModelError(
          `${modelSettings.selected_model} was not found in the models available to this API key.`
        );
      } else if (message === "OpenAI request format failed. Please update the app.") {
        setModelError(
          modelAvailability === "available"
            ? "Model is available, but the generation request failed. The app may need an OpenAI request-format update."
            : message
        );
      } else {
        setModelError(message);
      }
      setModelSettingsMode(true);
    } finally {
      setTestingModel(false);
    }
  }

  async function handleRefreshAvailableModels() {
    setRefreshingModels(true);
    setModelError(undefined);
    setModelMessage(undefined);
    try {
      const models = await invoke<string[]>("list_available_models");
      const sortedModels = [...models].sort((a, b) => a.localeCompare(b));
      const availability = availabilityForModel(modelSettings.selected_model, sortedModels);
      setAvailableModels(sortedModels);
      setModelAvailability(availability);
      setModelMessage(
        availability === "available"
          ? "Available models refreshed. Selected model is available to this API key."
          : "Available models refreshed. Selected model was not found in the models available to this API key."
      );
    } catch (error) {
      setModelAvailability("not_checked");
      setModelError(errorMessage(error));
      setModelSettingsMode(true);
    } finally {
      setRefreshingModels(false);
    }
  }

  async function handleClose() {
    if (state.operationId) {
      try {
        await invoke("cancel_operation", { operationId: state.operationId });
      } catch {
        // Cancellation errors are intentionally not expanded to avoid surfacing internals.
      }
      dispatch({ type: "clear" });
      return;
    }

    await invoke("hide_to_tray");
  }

  function handleRetryRephrase() {
    if (!state.operationId || !state.originalText) {
      return;
    }
    const requestId = nextRequestId(requestCounter);
    const operationId = state.operationId;

    dispatch({
      type: "section-started",
      operationId,
      section: "rephrase",
      requestId
    });
    void runTransform(
      "rephrase",
      operationId,
      state.originalText,
      requestId,
      dispatch,
      handleTransformConfigurationError
    );
  }

  async function handleApply(section: SectionKind) {
    if (!state.operationId) {
      return;
    }

    const finalText = reconstructText(state[section].tokens);
    try {
      await invoke("apply_text", {
        operationId: state.operationId,
        finalText
      });
      dispatch({ type: "clear" });
    } catch (error) {
      dispatch({ type: "operation-error", message: errorMessage(error) });
    }
  }

  return (
    <Popup
      state={state}
      hasApiKey={hasApiKey}
      savingKey={savingKey}
      testingKey={testingKey}
      keyError={keyError}
      keyTestMessage={keyTestMessage}
      settingsMode={settingsMode}
      modelSettingsMode={modelSettingsMode}
      modelSettings={modelSettings}
      modelPresets={modelPresets}
      savingModel={savingModel}
      testingModel={testingModel}
      modelError={modelError}
      modelMessage={modelMessage}
      availableModels={availableModels}
      refreshingModels={refreshingModels}
      modelAvailability={modelAvailability}
      modelTestStatus={modelTestStatus}
      onSaveKey={handleSaveKey}
      onTestKey={handleTestKey}
      onShowSettings={() => setSettingsMode(true)}
      onShowModelSettings={() => setModelSettingsMode(true)}
      onSaveModel={handleSaveModel}
      onTestStoredModel={handleTestStoredModel}
      onRefreshAvailableModels={handleRefreshAvailableModels}
      onHideToTray={() => void invoke("hide_to_tray")}
      onQuit={() => void invoke("quit_app")}
      onClose={handleClose}
      onTokenClick={(section, tokenId) => dispatch({ type: "toggle-token", section, tokenId })}
      onApply={handleApply}
      onRetryRephrase={handleRetryRephrase}
    />
  );

  async function refreshApiKeyStatus() {
    try {
      const configured = await invoke<boolean>("has_api_key");
      setHasApiKey(configured);
      return configured;
    } catch {
      setHasApiKey(false);
      return false;
    }
  }

  async function refreshModelSettings() {
    const settings = await invoke<ModelSettings>("get_model_settings");
    setModelSettings(settings);
    setModelAvailability(availabilityForModel(settings.selected_model, availableModels));
    return settings;
  }

  function handleTransformConfigurationError(message: string) {
    if (message === "OpenAI API key is missing.") {
      void refreshApiKeyStatus();
      setSettingsMode(true);
    }
    if (
      message === "The selected OpenAI model is unavailable or invalid. Please choose another model in Settings." ||
      message === "The selected model is not available for this API key. Choose another model from the available models list."
    ) {
      setModelError(message);
      setModelSettingsMode(true);
    }
  }
}

async function runTransform(
  section: SectionKind,
  operationId: string,
  originalText: string,
  requestId: number,
  dispatch: Dispatch<Action>,
  onConfigurationError: (message: string) => void
) {
  const command = section === "correction" ? "run_correction" : "run_rephrase";

  try {
    const candidateText = await invoke<string>(command, {
      operationId,
      originalText
    });
    dispatch({
      type: "section-success",
      operationId,
      section,
      requestId,
      candidateText
    });
  } catch (error) {
    const message = errorMessage(error);
    onConfigurationError(message);
    dispatch({
      type: "section-error",
      operationId,
      section,
      requestId,
      message
    });
  }
}

function nextRequestId(counter: MutableRefObject<number>) {
  counter.current += 1;
  return counter.current;
}

function errorMessage(error: unknown) {
  return typeof error === "string" ? error : "Something went wrong. Please try again.";
}

function availabilityForModel(model: string, availableModels: string[] | null): ModelAvailabilityStatus {
  if (!availableModels) {
    return "not_checked";
  }

  return availableModels.includes(model) ? "available" : "not_found";
}

function isEditableTarget(target: EventTarget | null) {
  const element = target as HTMLElement | null;
  if (!element) {
    return false;
  }

  const tagName = element.tagName?.toLowerCase();
  return tagName === "input" || tagName === "textarea" || tagName === "select" || element.isContentEditable;
}
