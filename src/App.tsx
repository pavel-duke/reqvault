import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  createWorkspace,
  authorizeOAuth,
  clearCookies,
  clearHistory,
  closeWorkspaceSession,
  deleteCookie,
  diagnoseWorkspace,
  duplicateRequests,
  getHistoryEntry,
  getHistorySettings,
  getWorkspaceFingerprint,
  errorMessage,
  exportWorkspace,
  exportResponse,
  generateSafeCurl,
  importCollection,
  importCurl,
  importWorkspace,
  inspectRequest,
  listHistory,
  listCookies,
  listSecrets,
  migrateWorkspace,
  moveRequests,
  openWorkspace,
  removeEnvironment,
  removeHistoryEntry,
  removeRequest,
  removeSecret,
  refreshOAuth,
  renameRequest,
  rollbackWorkspaceMigration,
  runCollection,
  saveEnvironment,
  saveResponseFixture,
  sendHttpRequest,
  saveRequest,
  saveSecret,
  saveWorkspaceConfig,
  updateHistorySettings,
} from "./api";
import { EnvironmentDialog } from "./components/EnvironmentDialog";
import { DiagnosticsDialog } from "./components/DiagnosticsDialog";
import { Icon, ReqVaultMark } from "./components/Icon";
import { CurlImportDialog } from "./components/CurlImportDialog";
import { CollectionRunnerDialog } from "./components/CollectionRunnerDialog";
import { CommandPalette, type PaletteAction } from "./components/CommandPalette";
import { CookieDialog } from "./components/CookieDialog";
import { HistoryDialog } from "./components/HistoryDialog";
import { RequestEditor } from "./components/RequestEditor";
import { RequestManagerDialog } from "./components/RequestManagerDialog";
import { RequestTabs } from "./components/RequestTabs";
import { ResponseCompareDialog } from "./components/ResponseCompareDialog";
import { ResponseViewer } from "./components/ResponseViewer";
import { SecretDialog } from "./components/SecretDialog";
import { Sidebar } from "./components/Sidebar";
import { StartScreen } from "./components/StartScreen";
import { WorkspaceSettingsDialog } from "./components/WorkspaceSettingsDialog";
import { WorkspaceRail } from "./components/WorkspaceRail";
import { WorkspaceOverview } from "./components/WorkspaceOverview";
import { StreamDialog } from "./components/StreamDialog";
import { collectionFromPath, emptyRequest } from "./request-utils";
import { draftStorageKey, type StoredDraft } from "./draft-storage";
import { addRecent, loadNavigation, saveNavigation } from "./navigation-storage";
import { loadTabs, newTabId, savedTabId, saveTabs, type RequestTabState } from "./tabs-storage";
import type { CollectionRunOptions, CollectionRunReport, CookieSummary, EnvironmentFile, HistorySettings, HistorySummary, HttpError, HttpResponse, RequestFile, RequestSummary, SecurityReport, Theme, WorkspaceConfig, WorkspaceDiagnostics, WorkspaceSnapshot } from "./types";
import "./App.css";

const LAST_WORKSPACE_KEY = "reqvault.last-workspace";

function initialTheme(): Theme {
  const savedTheme = window.localStorage.getItem("reqvault.theme");
  if (savedTheme === "light" || savedTheme === "dark") return savedTheme;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function App() {
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [workspace, setWorkspace] = useState<WorkspaceSnapshot | null>(null);
  const [activeEnvironment, setActiveEnvironment] = useState("");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [draft, setDraft] = useState<RequestFile | null>(null);
  const [collection, setCollection] = useState("Общее");
  const [busy, setBusy] = useState(() => Boolean(window.localStorage.getItem(LAST_WORKSPACE_KEY)));
  const [error, setError] = useState<string | null>(null);
  const [environmentsOpen, setEnvironmentsOpen] = useState(false);
  const [environmentError, setEnvironmentError] = useState<string | null>(null);
  const [response, setResponse] = useState<HttpResponse | null>(null);
  const [httpError, setHttpError] = useState<HttpError | null>(null);
  const [sending, setSending] = useState(false);
  const [secretsOpen, setSecretsOpen] = useState(false);
  const [secretNames, setSecretNames] = useState<string[]>([]);
  const [secretError, setSecretError] = useState<string | null>(null);
  const [secretBusy, setSecretBusy] = useState(false);
  const [securityReport, setSecurityReport] = useState<SecurityReport | null>(null);
  const [copyStatus, setCopyStatus] = useState<string | null>(null);
  const [oauthBusy, setOauthBusy] = useState(false);
  const [oauthStatus, setOauthStatus] = useState<string | null>(null);
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const [curlOpen, setCurlOpen] = useState(false);
  const [curlError, setCurlError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [runnerOpen, setRunnerOpen] = useState(false);
  const [runnerBusy, setRunnerBusy] = useState(false);
  const [runnerError, setRunnerError] = useState<string | null>(null);
  const [runnerReport, setRunnerReport] = useState<CollectionRunReport | null>(null);
  const [streamOpen, setStreamOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [historySettings, setHistorySettings] = useState<HistorySettings>({ enabled: false, max_entries: 50 });
  const [historyEntries, setHistoryEntries] = useState<HistorySummary[]>([]);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [cookiesOpen, setCookiesOpen] = useState(false);
  const [cookies, setCookies] = useState<CookieSummary[]>([]);
  const [cookiesBusy, setCookiesBusy] = useState(false);
  const [cookiesError, setCookiesError] = useState<string | null>(null);
  const [responseActionStatus, setResponseActionStatus] = useState<string | null>(null);
  const [compareOpen, setCompareOpen] = useState(false);
  const [compareBusy, setCompareBusy] = useState(false);
  const [compareError, setCompareError] = useState<string | null>(null);
  const [diagnosticsOpen, setDiagnosticsOpen] = useState(false);
  const [diagnosticsBusy, setDiagnosticsBusy] = useState(false);
  const [diagnosticsError, setDiagnosticsError] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<WorkspaceDiagnostics | null>(null);
  const [migrationBackupId, setMigrationBackupId] = useState<string | null>(null);
  const [knownFingerprint, setKnownFingerprint] = useState("");
  const [externalChange, setExternalChange] = useState(false);
  const [draftDirty, setDraftDirty] = useState(false);
  const [recoverableDraft, setRecoverableDraft] = useState<StoredDraft | null>(null);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [requestManagerOpen, setRequestManagerOpen] = useState(false);
  const [requestManagerError, setRequestManagerError] = useState<string | null>(null);
  const [favoritePaths, setFavoritePaths] = useState<string[]>([]);
  const [recentPaths, setRecentPaths] = useState<string[]>([]);
  const [openTabs, setOpenTabs] = useState<RequestTabState[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const tabStates = useRef(new Map<string, RequestTabState>());
  const modalOpen = environmentsOpen || secretsOpen || historyOpen || cookiesOpen || compareOpen
    || curlOpen || settingsOpen || runnerOpen || streamOpen || diagnosticsOpen || commandPaletteOpen
    || requestManagerOpen;

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem("reqvault.theme", theme);
  }, [theme]);

  useEffect(() => {
    const path = window.localStorage.getItem(LAST_WORKSPACE_KEY);
    if (!path) return;
    openWorkspace(path)
      .then((snapshot) => applyWorkspace(snapshot))
      .catch(() => window.localStorage.removeItem(LAST_WORKSPACE_KEY))
      .finally(() => setBusy(false));
    // Восстановление последнего workspace выполняется только при запуске окна.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!workspace || !draft || !draft.url.trim()) {
      const clearTimer = window.setTimeout(() => setSecurityReport(null), 0);
      return () => window.clearTimeout(clearTimer);
    }
    const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
    const timer = window.setTimeout(() => {
      inspectRequest(draft, environment).then(setSecurityReport).catch(() => setSecurityReport(null));
    }, 180);
    return () => window.clearTimeout(timer);
  }, [activeEnvironment, draft, workspace]);

  useEffect(() => {
    if (!workspace) return;
    const timer = window.setTimeout(() => {
      saveTabs(workspace.config.id, activeTabId, openTabs);
      if (!recoverableDraft) window.localStorage.removeItem(draftStorageKey(workspace.config.id));
    }, 180);
    return () => window.clearTimeout(timer);
  }, [activeTabId, openTabs, recoverableDraft, workspace]);

  useEffect(() => {
    if (!workspace || !knownFingerprint) return;
    let disposed = false;
    const check = async () => {
      try {
        const fingerprint = await getWorkspaceFingerprint(workspace.root_path);
        if (!disposed && fingerprint !== knownFingerprint) setExternalChange(true);
      } catch {
        // Ошибка будет показана при ручном открытии или диагностике workspace.
      }
    };
    const timer = window.setInterval(() => void check(), 3000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [knownFingerprint, workspace]);

  useEffect(() => {
    if (!modalOpen) return;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const frame = window.requestAnimationFrame(() => {
      document.querySelector<HTMLElement>("[role='dialog'] button:not(:disabled), [role='dialog'] input:not(:disabled), [role='dialog'] select:not(:disabled)")?.focus();
    });
    const trapFocus = (event: globalThis.KeyboardEvent) => {
      const dialog = document.querySelector<HTMLElement>("[role='dialog']");
      if (!dialog) return;
      if (event.key === "Escape") {
        event.preventDefault();
        dialog.querySelector<HTMLButtonElement>("button[aria-label='Закрыть']")?.click();
        return;
      }
      if (event.key !== "Tab") return;
      const focusable = [...dialog.querySelectorAll<HTMLElement>("button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [href], [tabindex]:not([tabindex='-1'])")];
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", trapFocus);
    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("keydown", trapFocus);
      previous?.focus();
    };
  }, [modalOpen]);

  useEffect(() => {
    if (!workspace) return;
    const saveBeforeClose = () => saveTabs(workspace.config.id, activeTabId, openTabs);
    window.addEventListener("beforeunload", saveBeforeClose);
    return () => window.removeEventListener("beforeunload", saveBeforeClose);
  }, [activeTabId, openTabs, workspace]);

  function applyWorkspace(snapshot: WorkspaceSnapshot) {
    const workspaceChanged = workspace?.config.id !== snapshot.config.id;
    setWorkspace(snapshot);
    window.localStorage.setItem(LAST_WORKSPACE_KEY, snapshot.root_path);
    setExternalChange(false);
    const navigation = loadNavigation(snapshot.config.id, snapshot.requests.map((item) => item.relative_path));
    setFavoritePaths(navigation.favorites);
    setRecentPaths(navigation.recent);
    if (workspaceChanged) {
      const restored = loadTabs(snapshot.config.id, snapshot.requests);
      replaceTabs(restored.tabs);
      const active = restored.tabs.find((tab) => tab.id === restored.activeId) ?? restored.tabs[0] ?? null;
      if (active) activateTab(active);
      else clearActiveTab();
    }
    void getWorkspaceFingerprint(snapshot.root_path).then(setKnownFingerprint).catch(() => setKnownFingerprint(""));
    if (workspaceChanged) {
      const stored = window.localStorage.getItem(draftStorageKey(snapshot.config.id));
      if (stored) {
        try {
          setRecoverableDraft(JSON.parse(stored) as StoredDraft);
        } catch {
          window.localStorage.removeItem(draftStorageKey(snapshot.config.id));
          setRecoverableDraft(null);
        }
      } else {
        setRecoverableDraft(null);
      }
    }
    setActiveEnvironment((current) =>
      snapshot.environments.some((item) => item.relative_path === current)
        ? current
        : (snapshot.environments[0]?.relative_path ?? ""),
    );
  }

  async function pickWorkspace(mode: "create" | "open") {
    setError(null);
    const selected = await open({ directory: true, multiple: false, title: mode === "create" ? "Выбери папку для workspace" : "Открой папку workspace" });
    if (!selected) return;
    setBusy(true);
    try {
      const snapshot = mode === "create" ? await createWorkspace(selected) : await openWorkspace(selected);
      applyWorkspace(snapshot);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  function replaceTabs(tabs: RequestTabState[]) {
    tabStates.current = new Map(tabs.map((tab) => [tab.id, tab]));
    setOpenTabs(tabs);
  }

  function updateTabState(tab: RequestTabState) {
    tabStates.current.set(tab.id, tab);
    setOpenTabs((current) => current.map((item) => item.id === tab.id ? tab : item));
  }

  function stashActiveTab() {
    if (!activeTabId || !draft) return;
    const current = tabStates.current.get(activeTabId);
    if (!current) return;
    updateTabState({ ...current, relativePath: selectedPath, collection, request: structuredClone(draft), dirty: draftDirty, response, httpError });
  }

  function activateTab(tab: RequestTabState) {
    setActiveTabId(tab.id);
    setSelectedPath(tab.relativePath);
    setDraft(structuredClone(tab.request));
    setCollection(tab.collection);
    setDraftDirty(tab.dirty);
    setResponse(tab.response);
    setHttpError(tab.httpError);
    setError(null);
  }

  function clearActiveTab() {
    setActiveTabId(null);
    setSelectedPath(null);
    setDraft(null);
    setCollection("Общее");
    setDraftDirty(false);
    setResponse(null);
    setHttpError(null);
  }

  function selectRequest(summary: RequestSummary) {
    stashActiveTab();
    const id = savedTabId(summary.relative_path);
    const existing = tabStates.current.get(id);
    const tab = existing ?? {
      id,
      relativePath: summary.relative_path,
      collection: collectionFromPath(summary.relative_path),
      dirty: false,
      request: structuredClone(summary.request),
      response: null,
      httpError: null,
    };
    if (!existing) {
      if (openTabs.length >= 20) {
        setError("Открыто 20 вкладок. Закрой ненужную вкладку и повтори.");
        return;
      }
      tabStates.current.set(id, tab);
      setOpenTabs((current) => [...current, tab]);
    }
    activateTab(tab);
    if (workspace) {
      const nextRecent = addRecent(recentPaths, summary.relative_path);
      setRecentPaths(nextRecent);
      saveNavigation(workspace.config.id, { favorites: favoritePaths, recent: nextRecent });
    }
  }

  function toggleFavorite(path: string) {
    if (!workspace) return;
    const nextFavorites = favoritePaths.includes(path)
      ? favoritePaths.filter((item) => item !== path)
      : [path, ...favoritePaths];
    setFavoritePaths(nextFavorites);
    saveNavigation(workspace.config.id, { favorites: nextFavorites, recent: recentPaths });
  }

  async function moveSelectedRequests(paths: string[], targetCollection: string) {
    if (!workspace) return;
    stashActiveTab();
    setBusy(true);
    setRequestManagerError(null);
    try {
      const result = await moveRequests(workspace.root_path, paths, targetCollection);
      const pathMap = new Map(result.changes.map((change) => [change.from, change.to]));
      const sourceTabs = openTabs.map((tab) => tabStates.current.get(tab.id) ?? tab);
      const activeIndex = sourceTabs.findIndex((tab) => tab.id === activeTabId);
      const nextTabs = sourceTabs.map((tab) => {
        if (!tab.relativePath) return tab;
        const nextPath = pathMap.get(tab.relativePath);
        return nextPath ? { ...tab, id: savedTabId(nextPath), relativePath: nextPath, collection: collectionFromPath(nextPath) } : tab;
      });
      const nextFavorites = [...new Set(favoritePaths.map((path) => pathMap.get(path) ?? path))];
      const nextRecent = [...new Set(recentPaths.map((path) => pathMap.get(path) ?? path))];
      applyWorkspace(result.workspace);
      replaceTabs(nextTabs);
      const nextActive = nextTabs[activeIndex] ?? nextTabs[0] ?? null;
      if (nextActive) activateTab(nextActive);
      else clearActiveTab();
      setFavoritePaths(nextFavorites);
      setRecentPaths(nextRecent);
      saveNavigation(result.workspace.config.id, { favorites: nextFavorites, recent: nextRecent });
      setImportStatus(`Перемещено запросов: ${result.changes.filter((change) => change.from !== change.to).length}`);
      setRequestManagerOpen(false);
    } catch (caught) {
      setRequestManagerError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function duplicateSelectedRequests(paths: string[], targetCollection: string) {
    if (!workspace) return;
    const hasDirtySource = [...tabStates.current.values()].some((tab) => tab.dirty && tab.relativePath && paths.includes(tab.relativePath));
    if (hasDirtySource) {
      setRequestManagerError("Сначала сохрани изменённые запросы: дубликат создаётся из YAML-файла на диске.");
      return;
    }
    setBusy(true);
    setRequestManagerError(null);
    try {
      const result = await duplicateRequests(workspace.root_path, paths, targetCollection);
      applyWorkspace(result.workspace);
      setImportStatus(`Создано копий: ${result.changes.length}`);
      setRequestManagerOpen(false);
    } catch (caught) {
      setRequestManagerError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function renameSelectedRequest(path: string, name: string) {
    if (!workspace) return;
    stashActiveTab();
    setBusy(true);
    setRequestManagerError(null);
    try {
      const snapshot = await renameRequest(workspace.root_path, path, name);
      const saved = snapshot.requests.find((summary) => summary.relative_path === path);
      const sourceTabs = openTabs.map((tab) => tabStates.current.get(tab.id) ?? tab);
      const nextTabs = sourceTabs.map((tab) => tab.relativePath === path && saved
        ? { ...tab, request: { ...tab.request, name: saved.request.name } }
        : tab);
      applyWorkspace(snapshot);
      replaceTabs(nextTabs);
      const nextActive = nextTabs.find((tab) => tab.id === activeTabId) ?? nextTabs[0] ?? null;
      if (nextActive) activateTab(nextActive);
      setImportStatus("Название запроса обновлено");
      setRequestManagerOpen(false);
    } catch (caught) {
      setRequestManagerError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  function newRequest() {
    if (openTabs.length >= 20) {
      setError("Открыто 20 вкладок. Закрой ненужную вкладку перед созданием новой.");
      return;
    }
    stashActiveTab();
    const tab: RequestTabState = {
      id: newTabId(),
      relativePath: null,
      collection: "Общее",
      dirty: true,
      request: emptyRequest(),
      response: null,
      httpError: null,
    };
    tabStates.current.set(tab.id, tab);
    setOpenTabs((current) => [...current, tab]);
    activateTab(tab);
  }

  function updateDraft(request: RequestFile) {
    setDraft(request);
    setDraftDirty(true);
    if (activeTabId) {
      const current = tabStates.current.get(activeTabId);
      if (current) updateTabState({ ...current, request: structuredClone(request), dirty: true });
    }
  }

  function updateCollection(value: string) {
    setCollection(value);
    setDraftDirty(true);
    if (activeTabId) {
      const current = tabStates.current.get(activeTabId);
      if (current) updateTabState({ ...current, collection: value, dirty: true });
    }
  }

  function selectOpenTab(id: string) {
    if (id === activeTabId) return;
    stashActiveTab();
    const tab = tabStates.current.get(id);
    if (tab) activateTab(tab);
  }

  function closeRequestTab(id: string) {
    const tab = tabStates.current.get(id);
    if (!tab) return;
    if (tab.dirty && !window.confirm(`Закрыть «${tab.request.name || "Без названия"}» и удалить несохранённый черновик?`)) return;
    const index = openTabs.findIndex((item) => item.id === id);
    const remaining = openTabs.filter((item) => item.id !== id);
    tabStates.current.delete(id);
    setOpenTabs(remaining);
    if (id === activeTabId) {
      const next = remaining[Math.min(index, remaining.length - 1)] ?? null;
      if (next) activateTab(next);
      else clearActiveTab();
    }
  }

  async function sendCurrentRequest() {
    if (!workspace || !draft) return;
    const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
    setSending(true);
    setHttpError(null);
    setResponseActionStatus(null);
    try {
      const result = await sendHttpRequest(draft, environment, workspace.config.id, workspace.root_path);
      setResponse(result);
      if (activeTabId) {
        const current = tabStates.current.get(activeTabId);
        if (current) updateTabState({ ...current, response: result, httpError: null });
      }
    } catch (caught) {
      setResponse(null);
      const nextError = caught && typeof caught === "object" && "message" in caught
        ? caught as HttpError
        : { message: errorMessage(caught), details: null, error_type: "unknown" };
      setHttpError(nextError);
      if (activeTabId) {
        const current = tabStates.current.get(activeTabId);
        if (current) updateTabState({ ...current, response: null, httpError: nextError });
      }
    } finally {
      setSending(false);
    }
  }

  async function openSecrets() {
    if (!workspace) return;
    setSecretsOpen(true);
    setSecretBusy(true);
    setSecretError(null);
    try {
      setSecretNames(await listSecrets(workspace.config.id));
    } catch (caught) {
      setSecretError(errorMessage(caught));
    } finally {
      setSecretBusy(false);
    }
  }

  async function persistSecret(name: string, value: string): Promise<boolean> {
    if (!workspace) return false;
    setSecretBusy(true);
    setSecretError(null);
    try {
      setSecretNames(await saveSecret(workspace.config.id, name, value));
      return true;
    } catch (caught) {
      setSecretError(errorMessage(caught));
      return false;
    } finally {
      setSecretBusy(false);
    }
  }

  async function deleteSavedSecret(name: string) {
    if (!workspace || !window.confirm(`Удалить секрет ${name}?`)) return;
    setSecretBusy(true);
    setSecretError(null);
    try {
      setSecretNames(await removeSecret(workspace.config.id, name));
    } catch (caught) {
      setSecretError(errorMessage(caught));
    } finally {
      setSecretBusy(false);
    }
  }

  async function copyCurl() {
    if (!workspace || !draft) return;
    const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
    setCopyStatus(null);
    try {
      const curl = await generateSafeCurl(draft, environment);
      await writeText(curl);
      setCopyStatus("Безопасный cURL скопирован. Значения секретов скрыты.");
    } catch (caught) {
      setCopyStatus(errorMessage(caught));
    }
  }

  async function authorizeCurrentOAuth() {
    if (!workspace || !draft || draft.auth.type !== "oauth2") return;
    const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
    setOauthBusy(true);
    setOauthStatus("Завершите авторизацию в открывшемся браузере.");
    try {
      const result = await authorizeOAuth(draft, environment, workspace.config.id);
      setOauthStatus(`Токен сохранён в системное хранилище как ${result.access_token_secret}.`);
      setSecretNames(await listSecrets(workspace.config.id));
    } catch (caught) {
      setOauthStatus(errorMessage(caught));
    } finally {
      setOauthBusy(false);
    }
  }

  async function refreshCurrentOAuth() {
    if (!workspace || !draft || draft.auth.type !== "oauth2") return;
    const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
    setOauthBusy(true);
    setOauthStatus(null);
    try {
      const result = await refreshOAuth(draft, environment, workspace.config.id);
      setOauthStatus(`Access token обновлён и сохранён как ${result.access_token_secret}.`);
    } catch (caught) {
      setOauthStatus(errorMessage(caught));
    } finally {
      setOauthBusy(false);
    }
  }

  async function importFile() {
    if (!workspace) return;
    setError(null);
    setImportStatus(null);
    const selected = await open({ multiple: false, directory: false, title: "Импорт Postman или OpenAPI", filters: [{ name: "Postman / OpenAPI", extensions: ["json", "yaml", "yml"] }] });
    if (typeof selected !== "string") return;
    setBusy(true);
    try {
      const result = await importCollection(workspace.root_path, selected);
      applyWorkspace(result.workspace);
      const warnings = result.warnings.length ? `\n${result.warnings.join("\n")}` : "";
      setImportStatus(`${result.source}: импортировано запросов ${result.imported_requests}, окружений ${result.imported_environments}.${warnings}`);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function importCurlCommand(command: string) {
    if (!workspace) return;
    setBusy(true);
    setCurlError(null);
    try {
      const result = await importCurl(workspace.root_path, command);
      applyWorkspace(result.workspace);
      const warnings = result.warnings.length ? `\n${result.warnings.join("\n")}` : "";
      setImportStatus(`cURL: запрос импортирован.${warnings}`);
      setCurlOpen(false);
    } catch (caught) {
      setCurlError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function exportCurrentWorkspace() {
    if (!workspace) return;
    setError(null);
    const destination = await save({
      title: "Экспортировать workspace",
      defaultPath: `${workspace.config.name}.reqvault.json`,
      filters: [{ name: "ReqVault bundle", extensions: ["json"] }],
    });
    if (!destination) return;
    setBusy(true);
    try {
      await exportWorkspace(workspace.root_path, destination);
      setImportStatus(`Workspace экспортирован: ${destination}`);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function importWorkspaceBundle() {
    setError(null);
    const source = await open({ multiple: false, directory: false, title: "Открыть ReqVault bundle", filters: [{ name: "ReqVault bundle", extensions: ["json"] }] });
    if (typeof source !== "string") return;
    const target = await open({ multiple: false, directory: true, title: "Выбери пустую папку для workspace" });
    if (typeof target !== "string") return;
    setBusy(true);
    try {
      const snapshot = await importWorkspace(source, target);
      applyWorkspace(snapshot);
      setDraft(null);
      setSelectedPath(null);
      setImportStatus("Workspace импортирован. Секреты нужно добавить отдельно.");
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function persistWorkspaceConfig(config: WorkspaceConfig) {
    if (!workspace) return;
    setBusy(true);
    setSettingsError(null);
    try {
      const saved = await saveWorkspaceConfig(workspace.root_path, config);
      setWorkspace({ ...workspace, config: saved });
      setSettingsOpen(false);
    } catch (caught) {
      setSettingsError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function runWorkspaceCollection(options: CollectionRunOptions) {
    if (!workspace) return;
    setRunnerBusy(true);
    setRunnerError(null);
    setRunnerReport(null);
    try {
      setRunnerReport(await runCollection(workspace.root_path, options));
    } catch (caught) {
      setRunnerError(errorMessage(caught));
    } finally {
      setRunnerBusy(false);
    }
  }

  async function openHistory() {
    if (!workspace) return;
    setHistoryOpen(true);
    setHistoryBusy(true);
    setHistoryError(null);
    try {
      const [settings, entries] = await Promise.all([getHistorySettings(workspace.config.id), listHistory(workspace.config.id)]);
      setHistorySettings(settings);
      setHistoryEntries(entries);
    } catch (caught) {
      setHistoryError(errorMessage(caught));
    } finally {
      setHistoryBusy(false);
    }
  }

  async function persistHistorySettings(settings: HistorySettings) {
    if (!workspace) return;
    setHistoryBusy(true);
    try {
      setHistorySettings(await updateHistorySettings(workspace.config.id, settings));
      setHistoryEntries(await listHistory(workspace.config.id));
    } catch (caught) {
      setHistoryError(errorMessage(caught));
    } finally {
      setHistoryBusy(false);
    }
  }

  async function deleteHistory(id: string) {
    if (!workspace) return;
    await removeHistoryEntry(workspace.config.id, id);
    setHistoryEntries(await listHistory(workspace.config.id));
  }

  async function clearSavedHistory() {
    if (!workspace || !window.confirm("Удалить все сохранённые ответы из локальной истории?")) return;
    await clearHistory(workspace.config.id);
    setHistoryEntries([]);
  }

  async function persistRequest() {
    if (!workspace || !draft) return;
    const previousTabId = activeTabId;
    setBusy(true);
    setError(null);
    setResponse(null);
    setHttpError(null);
    try {
      const saved = await saveRequest(workspace.root_path, selectedPath, collection, draft);
      const snapshot = await openWorkspace(workspace.root_path);
      applyWorkspace(snapshot);
      const id = savedTabId(saved.relative_path);
      const savedTab: RequestTabState = {
        id,
        relativePath: saved.relative_path,
        collection: collectionFromPath(saved.relative_path),
        dirty: false,
        request: structuredClone(saved.request),
        response: null,
        httpError: null,
      };
      const withoutDuplicate = openTabs.filter((tab) => tab.id !== id || tab.id === previousTabId);
      const nextTabs = previousTabId && withoutDuplicate.some((tab) => tab.id === previousTabId)
        ? withoutDuplicate.map((tab) => tab.id === previousTabId ? savedTab : tab)
        : [...withoutDuplicate, savedTab];
      replaceTabs(nextTabs);
      activateTab(savedTab);
      const nextRecent = addRecent(recentPaths, saved.relative_path);
      setRecentPaths(nextRecent);
      saveNavigation(workspace.config.id, { favorites: favoritePaths, recent: nextRecent });
      window.localStorage.removeItem(draftStorageKey(workspace.config.id));
      setRecoverableDraft(null);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function deleteCurrentRequest() {
    if (!workspace || !selectedPath || !window.confirm("Удалить этот запрос из workspace?")) return;
    const deletingTabId = activeTabId;
    setBusy(true);
    setError(null);
    try {
      await removeRequest(workspace.root_path, selectedPath);
      applyWorkspace(await openWorkspace(workspace.root_path));
      if (deletingTabId) {
        const index = openTabs.findIndex((tab) => tab.id === deletingTabId);
        const remaining = openTabs.filter((tab) => tab.id !== deletingTabId);
        replaceTabs(remaining);
        const next = remaining[Math.min(index, remaining.length - 1)] ?? null;
        if (next) activateTab(next);
        else clearActiveTab();
      } else {
        clearActiveTab();
      }
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function persistEnvironment(relativePath: string | null, environment: EnvironmentFile) {
    if (!workspace) return;
    setBusy(true);
    setEnvironmentError(null);
    try {
      const saved = await saveEnvironment(workspace.root_path, relativePath, environment);
      applyWorkspace(await openWorkspace(workspace.root_path));
      setActiveEnvironment(saved.relative_path);
      setEnvironmentsOpen(false);
    } catch (caught) {
      setEnvironmentError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function deleteSelectedEnvironment(relativePath: string) {
    if (!workspace || !window.confirm("Удалить это окружение?")) return;
    setBusy(true);
    setEnvironmentError(null);
    try {
      await removeEnvironment(workspace.root_path, relativePath);
      applyWorkspace(await openWorkspace(workspace.root_path));
    } catch (caught) {
      setEnvironmentError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function openCookies() {
    if (!workspace) return;
    setCookiesOpen(true);
    setCookiesBusy(true);
    setCookiesError(null);
    try {
      setCookies(await listCookies(workspace.config.id));
    } catch (caught) {
      setCookiesError(errorMessage(caught));
    } finally {
      setCookiesBusy(false);
    }
  }

  async function deleteSessionCookie(id: string) {
    if (!workspace) return;
    setCookiesBusy(true);
    setCookiesError(null);
    try {
      await deleteCookie(workspace.config.id, id);
      setCookies(await listCookies(workspace.config.id));
    } catch (caught) {
      setCookiesError(errorMessage(caught));
    } finally {
      setCookiesBusy(false);
    }
  }

  async function clearSessionCookies() {
    if (!workspace || !window.confirm("Очистить все cookie текущего workspace? Активные сессии на API могут завершиться.")) return;
    setCookiesBusy(true);
    setCookiesError(null);
    try {
      await clearCookies(workspace.config.id);
      setCookies([]);
    } catch (caught) {
      setCookiesError(errorMessage(caught));
    } finally {
      setCookiesBusy(false);
    }
  }

  async function exportCurrentResponse(format: "body" | "http" | "har") {
    if (!draft || !response) return;
    const extension = format === "har" ? "har" : format === "http" ? "http" : response.is_json ? "json" : "bin";
    const destination = await save({
      title: format === "har" ? "Экспортировать безопасный HAR" : "Сохранить ответ",
      defaultPath: `${draft.name || "response"}.${extension}`,
      filters: [{ name: format === "har" ? "HTTP Archive" : "Ответ API", extensions: [extension] }],
    });
    if (!destination) return;
    setResponseActionStatus(null);
    try {
      await exportResponse(destination, format, draft, response);
      setResponseActionStatus(`Ответ сохранён: ${destination}`);
    } catch (caught) {
      setResponseActionStatus(errorMessage(caught));
    }
  }

  async function saveCurrentFixture() {
    if (!workspace || !draft || !response) return;
    const extension = response.is_json ? "json" : response.body_kind === "image"
      ? (response.content_type.split("/")[1]?.split(";")[0] || "bin")
      : "txt";
    const suggested = `${draft.name.toLocaleLowerCase("ru").replace(/[^a-zа-яё0-9]+/gi, "-").replace(/^-|-$/g, "") || "response"}.${extension}`;
    const name = window.prompt("Имя fixture в папке workspace/fixtures", suggested);
    if (!name) return;
    setResponseActionStatus(null);
    try {
      const path = await saveResponseFixture(workspace.root_path, name, response);
      setResponseActionStatus(`Fixture сохранён: ${path}`);
    } catch (caught) {
      setResponseActionStatus(errorMessage(caught));
    }
  }

  async function openResponseCompare() {
    if (!workspace || !response) return;
    setCompareOpen(true);
    setCompareBusy(true);
    setCompareError(null);
    try {
      setHistoryEntries(await listHistory(workspace.config.id));
    } catch (caught) {
      setCompareError(errorMessage(caught));
    } finally {
      setCompareBusy(false);
    }
  }

  async function openDiagnostics() {
    if (!workspace) return;
    setDiagnosticsOpen(true);
    setDiagnosticsBusy(true);
    setDiagnosticsError(null);
    try {
      setDiagnostics(await diagnoseWorkspace(workspace.root_path));
    } catch (caught) {
      setDiagnosticsError(errorMessage(caught));
    } finally {
      setDiagnosticsBusy(false);
    }
  }

  async function applyMigration() {
    if (!workspace) return;
    setDiagnosticsBusy(true);
    setDiagnosticsError(null);
    try {
      const result = await migrateWorkspace(workspace.root_path);
      applyWorkspace(result.workspace);
      setMigrationBackupId(result.backup_id);
      setDiagnostics(await diagnoseWorkspace(workspace.root_path));
    } catch (caught) {
      setDiagnosticsError(errorMessage(caught));
    } finally {
      setDiagnosticsBusy(false);
    }
  }

  async function rollbackMigration(backupId: string) {
    if (!workspace) return;
    setDiagnosticsBusy(true);
    setDiagnosticsError(null);
    try {
      applyWorkspace(await rollbackWorkspaceMigration(workspace.root_path, backupId));
      setMigrationBackupId(null);
      setDiagnostics(await diagnoseWorkspace(workspace.root_path));
    } catch (caught) {
      setDiagnosticsError(errorMessage(caught));
    } finally {
      setDiagnosticsBusy(false);
    }
  }

  async function reloadExternalChanges() {
    if (!workspace) return;
    setBusy(true);
    setError(null);
    try {
      const snapshot = await openWorkspace(workspace.root_path);
      applyWorkspace(snapshot);
      const refreshed = openTabs.flatMap((tab) => {
        if (!tab.relativePath) return [tab];
        const saved = snapshot.requests.find((item) => item.relative_path === tab.relativePath);
        return saved ? [{ ...tab, request: structuredClone(saved.request), dirty: false, response: null, httpError: null }] : [];
      });
      replaceTabs(refreshed);
      const active = refreshed.find((tab) => tab.id === activeTabId) ?? refreshed[0] ?? null;
      if (active) activateTab(active);
      else clearActiveTab();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  function restoreDraft() {
    if (!recoverableDraft) return;
    stashActiveTab();
    const id = recoverableDraft.relativePath ? savedTabId(recoverableDraft.relativePath) : newTabId();
    const tab: RequestTabState = {
      id,
      relativePath: recoverableDraft.relativePath,
      collection: recoverableDraft.collection,
      dirty: true,
      request: structuredClone(recoverableDraft.request),
      response: null,
      httpError: null,
    };
    const nextTabs = openTabs.some((item) => item.id === id)
      ? openTabs.map((item) => item.id === id ? tab : item)
      : [...openTabs, tab];
    replaceTabs(nextTabs);
    activateTab(tab);
    setRecoverableDraft(null);
  }

  function discardRecoveredDraft() {
    if (workspace) window.localStorage.removeItem(draftStorageKey(workspace.config.id));
    setRecoverableDraft(null);
  }

  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setCommandPaletteOpen(true);
      } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        document.querySelector<HTMLButtonElement>("[data-shortcut='save-request']")?.click();
      } else if (event.altKey && event.key.toLowerCase() === "n") {
        event.preventDefault();
        document.querySelector<HTMLButtonElement>("[data-shortcut='new-request']")?.click();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  function closeWorkspace() {
    if (workspace) {
      stashActiveTab();
      saveTabs(workspace.config.id, activeTabId, openTabs.map((tab) => tabStates.current.get(tab.id) ?? tab));
      void closeWorkspaceSession(workspace.config.id);
      window.localStorage.removeItem(draftStorageKey(workspace.config.id));
    }
    window.localStorage.removeItem(LAST_WORKSPACE_KEY);
    setWorkspace(null);
    setDraft(null);
    setSelectedPath(null);
    setError(null);
    setRecoverableDraft(null);
    setDraftDirty(false);
    setCommandPaletteOpen(false);
    setRequestManagerOpen(false);
    setRequestManagerError(null);
    setFavoritePaths([]);
    setRecentPaths([]);
    tabStates.current.clear();
    setOpenTabs([]);
    setActiveTabId(null);
  }

  const paletteActions: PaletteAction[] = workspace ? [
    { id: "new-request", label: "Новый запрос", description: "Создать черновик запроса", keywords: "create", shortcut: "Alt N", icon: "file", onSelect: newRequest },
    { id: "send-request", label: "Выполнить запрос", description: draft ? `${draft.method} ${draft.name}` : "Сначала открой запрос", keywords: "send run execute", shortcut: "Ctrl Enter", icon: "activity", onSelect: () => void sendCurrentRequest() },
    { id: "search-tree", label: "Поиск в workspace", description: "Отфильтровать дерево запросов", keywords: "find filter", icon: "search", onSelect: () => window.requestAnimationFrame(() => document.querySelector<HTMLInputElement>("#workspace-search")?.focus()) },
    { id: "manage-requests", label: "Управление запросами", description: "Перенос, копирование и переименование", keywords: "batch move duplicate rename", icon: "archive", onSelect: () => { setRequestManagerError(null); setRequestManagerOpen(true); } },
    { id: "environment", label: "Окружения", description: activeEnvironment ? `Активное: ${workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment.name ?? activeEnvironment}` : "Настроить переменные окружения", keywords: "environment env variables", icon: "settings", onSelect: () => { setEnvironmentError(null); setEnvironmentsOpen(true); } },
    { id: "secrets", label: "Secret Vault", description: "Управление защищёнными значениями", keywords: "token password api key", icon: "key", onSelect: () => void openSecrets() },
    { id: "diagnostics", label: "Диагностика workspace", description: "Проверить YAML, переменные, TLS и файлы", keywords: "errors health", icon: "shield", onSelect: () => void openDiagnostics() },
    { id: "settings", label: "Production Guard", description: "Сетевые правила и разрешённые хосты", keywords: "settings security ssrf", icon: "settings", onSelect: () => { setSettingsError(null); setSettingsOpen(true); } },
    { id: "import", label: "Импорт Postman / OpenAPI", description: "Добавить запросы из файла", keywords: "swagger collection", icon: "download", onSelect: () => void importFile() },
    { id: "import-curl", label: "Импорт cURL", description: "Создать запрос из команды", keywords: "terminal paste", icon: "terminal", onSelect: () => { setCurlError(null); setCurlOpen(true); } },
    { id: "history", label: "История ответов", description: "Открыть локальную opt-in историю", keywords: "recent response", icon: "clock", onSelect: () => void openHistory() },
  ] : [];

  return (
    <main className="app-shell">
      <a className="skip-link" href="#main-workspace">Перейти к редактору</a>
      <header className="topbar">
        <div className="topbar-leading">
          <div className="brand" aria-label="ReqVault"><ReqVaultMark className="brand-mark" /><span>ReqVault</span><em>1.0</em></div>
          {workspace && <div className="workspace-breadcrumb"><span>/</span><strong>{workspace.config.name}</strong></div>}
        </div>
        <div className="topbar-actions">
          {workspace && <button className="command-trigger" type="button" onClick={() => setCommandPaletteOpen(true)}><Icon name="search" /><span>Команды и поиск</span><kbd>Ctrl K</kbd></button>}
          <span className="local-status"><Icon name="shield" />Локально</span>
          <button className="icon-button" type="button" onClick={() => setTheme(theme === "dark" ? "light" : "dark")} aria-label={theme === "dark" ? "Включить светлую тему" : "Включить тёмную тему"} title={theme === "dark" ? "Светлая тема" : "Тёмная тема"}><Icon name={theme === "dark" ? "sun" : "moon"} /></button>
        </div>
      </header>

      {!workspace ? (
        <StartScreen busy={busy} error={error} onCreate={() => void pickWorkspace("create")} onOpen={() => void pickWorkspace("open")} onImport={() => void importWorkspaceBundle()} />
      ) : (
        <div className="workspace-shell">
          <WorkspaceRail
            guardEnabled={workspace.config.production_guard.enabled}
            onImport={() => void importFile()}
            onImportCurl={() => { setCurlError(null); setCurlOpen(true); }}
            onExport={() => void exportCurrentWorkspace()}
            onSettings={() => { setSettingsError(null); setSettingsOpen(true); }}
            onHistory={() => void openHistory()}
            onCookies={() => void openCookies()}
            onDiagnostics={() => void openDiagnostics()}
            onRun={() => { setRunnerError(null); setRunnerReport(null); setRunnerOpen(true); }}
            onStream={() => setStreamOpen(true)}
          />
          <Sidebar
            workspace={workspace}
            selectedPath={selectedPath}
            activeEnvironment={activeEnvironment}
            favoritePaths={favoritePaths}
            recentPaths={recentPaths}
            onSelectRequest={selectRequest}
            onToggleFavorite={toggleFavorite}
            onNewRequest={newRequest}
            onManageRequests={() => { setRequestManagerError(null); setRequestManagerOpen(true); }}
            onEnvironmentChange={setActiveEnvironment}
            onEditEnvironments={() => { setEnvironmentError(null); setEnvironmentsOpen(true); }}
            onEditSecrets={() => void openSecrets()}
            onClose={closeWorkspace}
          />
          <div className="main-panel" id="main-workspace" tabIndex={-1}>
            {externalChange && <div className="external-change-banner" role="status"><div><strong>Файлы workspace изменились вне ReqVault</strong><p>{draftDirty ? "Перезагрузка заменит текущий несохранённый черновик." : "Перезагрузите данные, чтобы увидеть актуальную версию."}</p></div><button className="secondary-button" type="button" onClick={() => void reloadExternalChanges()}>Перезагрузить</button></div>}
            {recoverableDraft && <div className="draft-recovery-banner" role="status"><div><strong>Найден несохранённый черновик</strong><p>Сохранён {new Intl.DateTimeFormat("ru-RU", { dateStyle: "short", timeStyle: "short" }).format(recoverableDraft.updatedAt)}. Credential были удалены из локальной копии.</p></div><div className="inline-actions"><button className="primary-button" type="button" onClick={restoreDraft}>Восстановить</button><button className="secondary-button" type="button" onClick={discardRecoveredDraft}>Удалить</button></div></div>}
            <RequestTabs tabs={openTabs} activeId={activeTabId} onSelect={selectOpenTab} onClose={closeRequestTab} onNew={newRequest} />
            {draft ? (
              <div className="request-workbench">
                {importStatus && <div className="success-banner">{importStatus}</div>}
                <RequestEditor request={draft} relativePath={selectedPath} collection={collection} saving={busy} sending={sending} error={error} securityReport={securityReport} copyStatus={copyStatus} dirty={draftDirty} onChange={updateDraft} onCollectionChange={updateCollection} onSave={() => void persistRequest()} onDelete={() => void deleteCurrentRequest()} onSend={() => void sendCurrentRequest()} onCopyCurl={() => void copyCurl()} onAuthorizeOAuth={() => void authorizeCurrentOAuth()} onRefreshOAuth={() => void refreshCurrentOAuth()} oauthBusy={oauthBusy} oauthStatus={oauthStatus} />
                <ResponseViewer response={response} error={httpError} loading={sending} onExport={(format) => void exportCurrentResponse(format)} onSaveFixture={() => void saveCurrentFixture()} onCompare={() => void openResponseCompare()} actionStatus={responseActionStatus} />
              </div>
            ) : (
              <WorkspaceOverview workspace={workspace} onNewRequest={newRequest} onImport={() => void importFile()} onRun={() => { setRunnerError(null); setRunnerReport(null); setRunnerOpen(true); }} />
            )}
          </div>
        </div>
      )}

      {workspace && environmentsOpen && (
        <EnvironmentDialog key={`${activeEnvironment}-${workspace.environments.length}`} environments={workspace.environments} activePath={activeEnvironment} busy={busy} error={environmentError} onSave={(path, environment) => void persistEnvironment(path, environment)} onDelete={(path) => void deleteSelectedEnvironment(path)} onClose={() => setEnvironmentsOpen(false)} />
      )}

      {workspace && commandPaletteOpen && (
        <CommandPalette
          open={commandPaletteOpen}
          actions={paletteActions}
          requests={workspace.requests}
          recentPaths={recentPaths}
          onOpenRequest={selectRequest}
          onClose={() => setCommandPaletteOpen(false)}
        />
      )}

      {workspace && requestManagerOpen && (
        <RequestManagerDialog
          requests={workspace.requests}
          busy={busy}
          error={requestManagerError}
          onMove={(paths, targetCollection) => void moveSelectedRequests(paths, targetCollection)}
          onDuplicate={(paths, targetCollection) => void duplicateSelectedRequests(paths, targetCollection)}
          onRename={(path, name) => void renameSelectedRequest(path, name)}
          onClose={() => setRequestManagerOpen(false)}
        />
      )}

      {workspace && secretsOpen && (
        <SecretDialog names={secretNames} loading={secretBusy} error={secretError} onSave={persistSecret} onDelete={(name) => void deleteSavedSecret(name)} onClose={() => setSecretsOpen(false)} />
      )}

      {workspace && historyOpen && (
        <HistoryDialog settings={historySettings} entries={historyEntries} busy={historyBusy} error={historyError} onSettingsChange={persistHistorySettings} onLoad={(id) => getHistoryEntry(workspace.config.id, id)} onDelete={deleteHistory} onClear={clearSavedHistory} onClose={() => setHistoryOpen(false)} />
      )}

      {workspace && cookiesOpen && (
        <CookieDialog cookies={cookies} busy={cookiesBusy} error={cookiesError} onDelete={deleteSessionCookie} onClear={clearSessionCookies} onClose={() => setCookiesOpen(false)} />
      )}

      {workspace && diagnosticsOpen && (
        <DiagnosticsDialog report={diagnostics} busy={diagnosticsBusy} error={diagnosticsError} backupId={migrationBackupId} onRefresh={() => void openDiagnostics()} onMigrate={applyMigration} onRollback={rollbackMigration} onClose={() => setDiagnosticsOpen(false)} />
      )}

      {workspace && response && compareOpen && (
        <ResponseCompareDialog current={response} entries={historyEntries} busy={compareBusy} error={compareError} onLoad={(id) => getHistoryEntry(workspace.config.id, id)} onClose={() => setCompareOpen(false)} />
      )}

      {workspace && curlOpen && (
        <CurlImportDialog busy={busy} error={curlError} onImport={(command) => void importCurlCommand(command)} onClose={() => setCurlOpen(false)} />
      )}

      {workspace && settingsOpen && (
        <WorkspaceSettingsDialog config={workspace.config} busy={busy} error={settingsError} onSave={(config) => void persistWorkspaceConfig(config)} onClose={() => setSettingsOpen(false)} />
      )}

      {workspace && runnerOpen && (
        <CollectionRunnerDialog workspace={workspace} activeEnvironment={activeEnvironment} busy={runnerBusy} error={runnerError} report={runnerReport} onRun={(options) => void runWorkspaceCollection(options)} onClose={() => setRunnerOpen(false)} />
      )}

      {workspace && streamOpen && (
        <StreamDialog workspace={workspace} activeEnvironment={activeEnvironment} onClose={() => setStreamOpen(false)} />
      )}
    </main>
  );
}

export default App;
