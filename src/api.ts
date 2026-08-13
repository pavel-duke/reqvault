import { invoke } from "@tauri-apps/api/core";
import type {
  EnvironmentFile,
  EnvironmentSummary,
  HttpError,
  HttpResponse,
  HistoryEntry,
  HistorySettings,
  HistorySummary,
  ImportResult,
  OAuthResult,
  RequestFile,
  RequestSummary,
  SecurityReport,
  WorkspaceSnapshot,
  WorkspaceConfig,
} from "./types";

export function createWorkspace(path: string): Promise<WorkspaceSnapshot> {
  return invoke("create_workspace", { path });
}

export function inspectRequest(
  request: RequestFile,
  environment: EnvironmentFile | null,
): Promise<SecurityReport> {
  return invoke("inspect_request", { request, environment });
}

export function generateSafeCurl(
  request: RequestFile,
  environment: EnvironmentFile | null,
): Promise<string> {
  return invoke("generate_safe_curl", { request, environment });
}

export function authorizeOAuth(
  request: RequestFile,
  environment: EnvironmentFile | null,
  workspaceId: string,
): Promise<OAuthResult> {
  return invoke("authorize_oauth", { request, environment, workspaceId });
}

export function refreshOAuth(
  request: RequestFile,
  environment: EnvironmentFile | null,
  workspaceId: string,
): Promise<OAuthResult> {
  return invoke("refresh_oauth", { request, environment, workspaceId });
}

export function importCollection(workspacePath: string, filePath: string): Promise<ImportResult> {
  return invoke("import_collection", { workspacePath, filePath });
}

export function importCurl(workspacePath: string, command: string): Promise<ImportResult> {
  return invoke("import_curl", { workspacePath, command });
}

export function exportWorkspace(workspacePath: string, destinationPath: string): Promise<void> {
  return invoke("export_workspace", { workspacePath, destinationPath });
}

export function importWorkspace(sourcePath: string, targetPath: string): Promise<WorkspaceSnapshot> {
  return invoke("import_workspace", { sourcePath, targetPath });
}

export function saveWorkspaceConfig(
  workspacePath: string,
  config: WorkspaceConfig,
): Promise<WorkspaceConfig> {
  return invoke("save_workspace_config", { workspacePath, config });
}

export function getHistorySettings(workspaceId: string): Promise<HistorySettings> {
  return invoke("get_history_settings", { workspaceId });
}

export function updateHistorySettings(
  workspaceId: string,
  settings: HistorySettings,
): Promise<HistorySettings> {
  return invoke("set_history_settings", { workspaceId, settings });
}

export function listHistory(workspaceId: string): Promise<HistorySummary[]> {
  return invoke("list_history", { workspaceId });
}

export function getHistoryEntry(workspaceId: string, id: string): Promise<HistoryEntry> {
  return invoke("get_history_entry", { workspaceId, id });
}

export function removeHistoryEntry(workspaceId: string, id: string): Promise<void> {
  return invoke("delete_history_entry", { workspaceId, id });
}

export function clearHistory(workspaceId: string): Promise<void> {
  return invoke("clear_history", { workspaceId });
}

export async function sendHttpRequest(
  request: RequestFile,
  environment: EnvironmentFile | null,
  workspaceId: string,
  workspacePath: string,
): Promise<HttpResponse> {
  try {
    return await invoke("send_request", { request, environment, workspaceId, workspacePath });
  } catch (error) {
    throw error as HttpError;
  }
}

export function listSecrets(workspaceId: string): Promise<string[]> {
  return invoke("list_secrets", { workspaceId });
}

export function saveSecret(workspaceId: string, name: string, value: string): Promise<string[]> {
  return invoke("save_secret", { workspaceId, name, value });
}

export function removeSecret(workspaceId: string, name: string): Promise<string[]> {
  return invoke("delete_secret", { workspaceId, name });
}

export function openWorkspace(path: string): Promise<WorkspaceSnapshot> {
  return invoke("open_workspace", { path });
}

export function saveRequest(
  workspacePath: string,
  relativePath: string | null,
  collection: string,
  request: RequestFile,
): Promise<RequestSummary> {
  return invoke("save_request", {
    workspacePath,
    relativePath,
    collection,
    request,
  });
}

export function removeRequest(
  workspacePath: string,
  relativePath: string,
): Promise<void> {
  return invoke("delete_request", { workspacePath, relativePath });
}

export function saveEnvironment(
  workspacePath: string,
  relativePath: string | null,
  environment: EnvironmentFile,
): Promise<EnvironmentSummary> {
  return invoke("save_environment", {
    workspacePath,
    relativePath,
    environment,
  });
}

export function removeEnvironment(
  workspacePath: string,
  relativePath: string,
): Promise<void> {
  return invoke("delete_environment", { workspacePath, relativePath });
}

export function errorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") {
    return error.message;
  }
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Произошла неизвестная ошибка";
}
