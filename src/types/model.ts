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
