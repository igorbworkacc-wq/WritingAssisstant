export type SectionKind = "correction" | "rephrase";

export type TokenSide = "original" | "candidate";

export type DiffTokenKind = "equal" | "replace" | "insert" | "delete";

export interface DiffToken {
  id: string;
  section: SectionKind;
  index: number;
  kind: DiffTokenKind;
  originalText: string;
  candidateText: string;
  selectedSide: TokenSide;
  clickable: boolean;
  highlighted: boolean;
  groupId?: string;
}

export interface TransformSectionState {
  section: SectionKind;
  status: "idle" | "loading" | "ready" | "error";
  originalText: string;
  candidateText: string;
  tokens: DiffToken[];
  errorMessage?: string;
  requestId: number;
}

export interface OperationState {
  operationId: string | null;
  originalText: string;
  correction: TransformSectionState;
  rephrase: TransformSectionState;
  targetCaptured: boolean;
  operationError?: string;
}
