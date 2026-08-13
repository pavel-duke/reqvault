import { invoke } from "@tauri-apps/api/core";
import type {
  EnvironmentFile,
  EnvironmentSummary,
  RequestFile,
  RequestSummary,
  WorkspaceSnapshot,
} from "./types";

export function createWorkspace(path: string): Promise<WorkspaceSnapshot> {
  return invoke("create_workspace", { path });
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
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Произошла неизвестная ошибка";
}
