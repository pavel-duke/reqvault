import { sanitizeDraft } from "./draft-storage";
import type { HttpError, HttpResponse, RequestFile, RequestSummary } from "./types";

export type RequestTabState = {
  id: string;
  relativePath: string | null;
  collection: string;
  dirty: boolean;
  request: RequestFile;
  response: HttpResponse | null;
  httpError: HttpError | null;
};

export type StoredTabs = {
  activeId: string | null;
  tabs: RequestTabState[];
};

const MAX_TABS = 20;

function key(workspaceId: string) {
  return `reqvault.tabs.${workspaceId}`;
}

export function savedTabId(relativePath: string) {
  return `request:${relativePath}`;
}

export function newTabId() {
  return `draft:${crypto.randomUUID()}`;
}

export function loadTabs(workspaceId: string, requests: RequestSummary[]): StoredTabs {
  const sourceByPath = new Map(requests.map((summary) => [summary.relative_path, summary.request]));
  try {
    const parsed = JSON.parse(window.localStorage.getItem(key(workspaceId)) ?? "{}") as Partial<StoredTabs>;
    if (!Array.isArray(parsed.tabs)) return { activeId: null, tabs: [] };
    const ids = new Set<string>();
    const tabs = parsed.tabs.flatMap((candidate) => {
      if (!candidate || typeof candidate.id !== "string" || ids.has(candidate.id)) return [];
      const relativePath = typeof candidate.relativePath === "string" ? candidate.relativePath : null;
      const savedRequest = relativePath ? sourceByPath.get(relativePath) : null;
      if (!savedRequest && !candidate.dirty) return [];
      if (!candidate.request || typeof candidate.request !== "object") return [];
      ids.add(candidate.id);
      return [{
        id: candidate.id,
        relativePath,
        collection: typeof candidate.collection === "string" ? candidate.collection : "Общее",
        dirty: Boolean(candidate.dirty),
        request: structuredClone(candidate.dirty || !savedRequest ? candidate.request : savedRequest),
        response: null,
        httpError: null,
      } satisfies RequestTabState];
    }).slice(0, MAX_TABS);
    const activeId = tabs.some((tab) => tab.id === parsed.activeId) ? parsed.activeId ?? null : tabs[0]?.id ?? null;
    return { activeId, tabs };
  } catch {
    window.localStorage.removeItem(key(workspaceId));
    return { activeId: null, tabs: [] };
  }
}

export function saveTabs(workspaceId: string, activeId: string | null, tabs: RequestTabState[]) {
  const safe = tabs.slice(0, MAX_TABS).map((tab) => ({
    ...tab,
    request: sanitizeDraft(tab.request),
    response: null,
    httpError: null,
  }));
  window.localStorage.setItem(key(workspaceId), JSON.stringify({ activeId, tabs: safe }));
}

export function clearTabs(workspaceId: string) {
  window.localStorage.removeItem(key(workspaceId));
}
