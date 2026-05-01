export interface OperationStartedPayload {
  operationId: string;
  originalText: string;
  targetCaptured: boolean;
}

export interface OperationErrorPayload {
  message: string;
}
