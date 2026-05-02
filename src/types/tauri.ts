export interface OperationStartedPayload {
  operationId: string;
  originalText: string;
  targetCaptured: boolean;
  model: string;
}

export interface OperationErrorPayload {
  message: string;
}
