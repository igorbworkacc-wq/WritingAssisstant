export interface ModelSettings {
  selected_model: string;
  temperature: number;
}

export interface ModelPreset {
  id: string;
  label: string;
  description: string;
  recommended_for: string;
}

export type ModelAvailabilityStatus = "not_checked" | "available" | "not_found";

export type ModelTestStatus = "not_tested" | "successful" | "failed";
