import { request } from "./client";
import type { AdminStatus, ApiKey, AppSettings, AvailableModel, CreatedApiKey, ModelRoute, PageRequest, Paginated, RequestLog, Upstream, UpstreamModel, UpstreamModels } from "../types/admin";

export function getAdminStatus() {
  return request<AdminStatus>("/admin/api/status");
}

export function initAdmin(username: string, password: string) {
  return request("/admin/api/init", {
    method: "POST",
    body: JSON.stringify({ username, password })
  });
}

export function loginAdmin(username: string, password: string) {
  return request("/admin/api/login", {
    method: "POST",
    body: JSON.stringify({ username, password })
  });
}

export function logoutAdmin() {
  return request("/admin/api/logout", { method: "POST" });
}

function pageQuery(input: PageRequest) {
  const params = new URLSearchParams({
    page: String(input.page),
    page_size: String(input.pageSize)
  });
  const q = input.q?.trim();
  if (q) params.set("q", q);
  return params.toString();
}

export function listUpstreams(input: PageRequest) {
  return request<Paginated<Upstream>>(`/admin/api/upstreams?${pageQuery(input)}`);
}

type UpstreamPayload = { name: string; base_url: string; api_key: string; enabled: boolean; models?: string[]; model_configs?: UpstreamModel[] };

export function createUpstream(input: UpstreamPayload) {
  return request("/admin/api/upstreams", {
    method: "POST",
    body: JSON.stringify(input)
  });
}

export function updateUpstream(id: number, input: UpstreamPayload) {
  return request<Upstream>(`/admin/api/upstreams/${id}`, {
    method: "PUT",
    body: JSON.stringify(input)
  });
}

export function deleteUpstream(id: number) {
  return request(`/admin/api/upstreams/${id}`, { method: "DELETE" });
}

export function fetchUpstreamModels(id: string) {
  return request<UpstreamModels>(`/admin/api/upstreams/${id}/models`);
}

export function listLocalUpstreamModels(id: number | string) {
  return request<UpstreamModel[]>(`/admin/api/upstreams/${id}/models/local`);
}

export function saveLocalUpstreamModels(id: number | string, models: UpstreamModel[]) {
  return request<UpstreamModel[]>(`/admin/api/upstreams/${id}/models`, {
    method: "PUT",
    body: JSON.stringify({ models })
  });
}

export function discoverUpstreamModels(input: { base_url: string; api_key: string }) {
  return request<UpstreamModels>("/admin/api/upstreams/discover-models", {
    method: "POST",
    body: JSON.stringify(input)
  });
}

export function listAvailableModels() {
  return request<AvailableModel[]>("/admin/api/available-models");
}

export function listModelRoutes(input: PageRequest) {
  return request<Paginated<ModelRoute>>(`/admin/api/models?${pageQuery(input)}`);
}

export function createModelRoute(input: {
  public_model: string;
  upstream_id: number;
  upstream_model: string;
  context_window: number;
  max_context_window: number;
  supports_parallel_tool_calls: boolean;
  supports_reasoning_summaries: boolean;
  enabled: boolean;
}) {
  return request("/admin/api/models", {
    method: "POST",
    body: JSON.stringify(input)
  });
}

export function updateModelRoute(
  id: number,
  input: {
    public_model: string;
    upstream_id: number;
    upstream_model: string;
    context_window: number;
    max_context_window: number;
    supports_parallel_tool_calls: boolean;
    supports_reasoning_summaries: boolean;
    enabled: boolean;
  }
) {
  return request<ModelRoute>(`/admin/api/models/${id}`, {
    method: "PUT",
    body: JSON.stringify(input)
  });
}

export function deleteModelRoute(id: number) {
  return request(`/admin/api/models/${id}`, { method: "DELETE" });
}

export function listApiKeys(input: PageRequest) {
  return request<Paginated<ApiKey>>(`/admin/api/keys?${pageQuery(input)}`);
}

export function createApiKey(input: { name: string; enabled: boolean; models?: string[] }) {
  return request<CreatedApiKey>("/admin/api/keys", {
    method: "POST",
    body: JSON.stringify(input)
  });
}

export function revealApiKey(id: number) {
  return request<{ key: string; masked_key: string }>(`/admin/api/keys/${id}/reveal`);
}

export function enableApiKey(id: number) {
  return request(`/admin/api/keys/${id}/enable`, { method: "POST" });
}

export function disableApiKey(id: number) {
  return request(`/admin/api/keys/${id}/disable`, { method: "POST" });
}

export function deleteApiKey(id: number) {
  return request(`/admin/api/keys/${id}`, { method: "DELETE" });
}

export function listRequestLogs(input: PageRequest) {
  return request<Paginated<RequestLog>>(`/admin/api/logs?${pageQuery(input)}`);
}

export function getSettings() {
  return request<AppSettings>("/admin/api/settings");
}

export function updateSettings(input: AppSettings) {
  return request<AppSettings>("/admin/api/settings", {
    method: "PUT",
    body: JSON.stringify(input)
  });
}
