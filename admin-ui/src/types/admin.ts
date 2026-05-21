export type AdminStatus = { initialized: boolean; user: null | { id: number; username: string } };

export type Paginated<T> = {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
};

export type PageRequest = {
  page: number;
  pageSize: number;
  q?: string;
};

export type PageState = {
  page: number;
  pageSize: number;
  q: string;
  total: number;
  totalPages: number;
};

export type AppSettings = {
  request_logging_enabled: boolean;
  upstream_timeout_seconds: number;
  log_error_max_chars: number;
};

export type Upstream = {
  id: number;
  name: string;
  base_url: string;
  enabled: boolean;
  model_count: number;
  enabled_model_count: number;
  created_at: string;
  updated_at: string;
};

export type UpstreamModel = {
  id?: number;
  upstream_id?: number;
  upstream_name?: string;
  model: string;
  enabled: boolean;
  context_window: number;
  max_context_window: number;
  supports_parallel_tool_calls: boolean;
  supports_reasoning_summaries: boolean;
  supports_image_input: boolean;
  last_seen_at?: string;
  created_at?: string;
  updated_at?: string;
};

export type ModelRoute = {
  id: number;
  public_model: string;
  upstream_id: number;
  upstream_name: string;
  upstream_model: string;
  context_window: number;
  max_context_window: number;
  supports_parallel_tool_calls: boolean;
  supports_reasoning_summaries: boolean;
  supports_image_input: boolean;
  enabled: boolean;
};

export type ApiKey = {
  id: number;
  name: string;
  enabled: boolean;
  created_at: string;
  last_used_at: string | null;
  masked_key: string | null;
  key_recoverable: boolean;
  models: string[];
};

export type CreatedApiKey = ApiKey & { key: string };

export type UpstreamModels = { data: unknown[]; models: string[]; model_configs: UpstreamModel[] };

export type AvailableModel = {
  id: string;
  source: string;
  owner: string;
  candidate_count: number;
  context_window: number;
  max_context_window: number;
  supports_parallel_tool_calls: boolean;
  supports_reasoning_summaries: boolean;
  supports_image_input: boolean;
};

export type CodexCatalogStatus = {
  source_model_count: number;
  generated_model_count: number;
};

export type RequestLog = {
  id: number;
  api_key_name: string | null;
  public_model: string | null;
  upstream_name: string | null;
  upstream_model: string | null;
  status_code: number;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  error: string | null;
  duration_ms: number;
  created_at: string;
};

export type ToastState = { type: "ok" | "error"; message: string } | null;
