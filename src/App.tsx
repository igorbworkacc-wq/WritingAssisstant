import { useEffect, useReducer, useRef, useState } from "react";
import type { Dispatch, MutableRefObject } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { buildDiffTokens, reconstructText, toggleToken } from "./lib/diffTokens";
import { Popup } from "./components/Popup";
import type { OperationErrorPayload, OperationStartedPayload } from "./types/tauri";
import type { OperationState, SectionKind, TransformSectionState } from "./types/diff";

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
        }
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
          errorMessage: action.message
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
  const [keyError, setKeyError] = useState<string>();
  const requestCounter = useRef(0);

  useEffect(() => {
    void invoke<boolean>("has_api_key")
      .then(setHasApiKey)
      .catch(() => setHasApiKey(false));
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
        dispatch
      );
      void runTransform(
        "rephrase",
        event.payload.operationId,
        event.payload.originalText,
        rephraseRequestId,
        dispatch
      );
    });

    const unlistenError = listen<OperationErrorPayload>("operation-error", (event) => {
      dispatch({ type: "operation-error", message: event.payload.message });
    });

    return () => {
      void unlistenStarted.then((unlisten) => unlisten());
      void unlistenError.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
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
    try {
      await invoke("set_api_key", { apiKey });
      setHasApiKey(true);
    } catch (error) {
      setKeyError(errorMessage(error));
    } finally {
      setSavingKey(false);
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

    await getCurrentWindow().hide();
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
    void runTransform("rephrase", operationId, state.originalText, requestId, dispatch);
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
      keyError={keyError}
      onSaveKey={handleSaveKey}
      onClose={handleClose}
      onTokenClick={(section, tokenId) => dispatch({ type: "toggle-token", section, tokenId })}
      onApply={handleApply}
      onRetryRephrase={handleRetryRephrase}
    />
  );
}

async function runTransform(
  section: SectionKind,
  operationId: string,
  originalText: string,
  requestId: number,
  dispatch: Dispatch<Action>
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
    dispatch({
      type: "section-error",
      operationId,
      section,
      requestId,
      message: errorMessage(error)
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
