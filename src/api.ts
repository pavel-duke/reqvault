import { invoke } from "@tauri-apps/api/core";
import type {
  EnvironmentFile,
  EnvironmentSummary,
  HttpError,
  HttpResponse,
  RequestFile,
  RequestSummary,
  WorkspaceSnapshot,
} from "./types";

export function createWorkspace(path: string): Promise<WorkspaceSnapshot> {
  return invoke("create_workspace", { path });
}

export async function sendHttpRequest(
  request: RequestFile,
  environment: EnvironmentFile | null,
  workspaceId: string,
): Promise<HttpResponse> {
  try {
    return await invoke("send_request", { request, environment, workspaceId });
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
