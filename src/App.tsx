import { useEffect, useState } from "react";
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
  openWorkspace,
  removeEnvironment,
  removeHistoryEntry,
  removeRequest,
  removeSecret,
  refreshOAuth,
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
import { CookieDialog } from "./components/CookieDialog";
import { HistoryDialog } from "./components/HistoryDialog";
import { RequestEditor } from "./components/RequestEditor";
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
import { draftStorageKey, sanitizeDraft, type StoredDraft } from "./draft-storage";
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
  const modalOpen = environmentsOpen || secretsOpen || historyOpen || cookiesOpen || compareOpen
    || curlOpen || settingsOpen || runnerOpen || streamOpen || diagnosticsOpen;

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
    if (!workspace || !draft || !draftDirty) return;
    const timer = window.setTimeout(() => {
      const stored: StoredDraft = {
        relativePath: selectedPath,
        collection,
        updatedAt: Date.now(),
        request: sanitizeDraft(draft),
      };
      window.localStorage.setItem(draftStorageKey(workspace.config.id), JSON.stringify(stored));
    }, 250);
    return () => window.clearTimeout(timer);
  }, [collection, draft, draftDirty, selectedPath, workspace]);

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
    if (!workspace || !draft || !draftDirty) return;
    const saveBeforeClose = () => {
      const stored: StoredDraft = { relativePath: selectedPath, collection, updatedAt: Date.now(), request: sanitizeDraft(draft) };
      window.localStorage.setItem(draftStorageKey(workspace.config.id), JSON.stringify(stored));
    };
    window.addEventListener("beforeunload", saveBeforeClose);
    return () => window.removeEventListener("beforeunload", saveBeforeClose);
  }, [collection, draft, draftDirty, selectedPath, workspace]);

  function applyWorkspace(snapshot: WorkspaceSnapshot) {
    const workspaceChanged = workspace?.config.id !== snapshot.config.id;
    setWorkspace(snapshot);
    window.localStorage.setItem(LAST_WORKSPACE_KEY, snapshot.root_path);
    setExternalChange(false);
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
      setDraft(null);
      setSelectedPath(null);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  function selectRequest(summary: RequestSummary) {
    setSelectedPath(summary.relative_path);
    setDraft(structuredClone(summary.request));
    setCollection(collectionFromPath(summary.relative_path));
    setError(null);
    setResponse(null);
    setHttpError(null);
    setDraftDirty(false);
  }

  function newRequest() {
    setSelectedPath(null);
    setDraft(emptyRequest());
    setCollection("Общее");
    setError(null);
    setResponse(null);
    setHttpError(null);
    setDraftDirty(true);
  }

  function updateDraft(request: RequestFile) {
    setDraft(request);
    setDraftDirty(true);
  }

  async function sendCurrentRequest() {
    if (!workspace || !draft) return;
    const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
    setSending(true);
    setHttpError(null);
    setResponseActionStatus(null);
    try {
      setResponse(await sendHttpRequest(draft, environment, workspace.config.id, workspace.root_path));
    } catch (caught) {
      setResponse(null);
      if (caught && typeof caught === "object" && "message" in caught) {
        setHttpError(caught as HttpError);
      } else {
        setHttpError({ message: errorMessage(caught), details: null, error_type: "unknown" });
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
    setBusy(true);
    setError(null);
    setResponse(null);
    setHttpError(null);
    try {
      const saved = await saveRequest(workspace.root_path, selectedPath, collection, draft);
      const snapshot = await openWorkspace(workspace.root_path);
      applyWorkspace(snapshot);
      selectRequest(saved);
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
    setBusy(true);
    setError(null);
    try {
      await removeRequest(workspace.root_path, selectedPath);
      applyWorkspace(await openWorkspace(workspace.root_path));
      setSelectedPath(null);
      setDraft(null);
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
      const selected = selectedPath ? snapshot.requests.find((item) => item.relative_path === selectedPath) : null;
      applyWorkspace(snapshot);
      if (selected) selectRequest(selected);
      else {
        setSelectedPath(null);
        setDraft(null);
        setDraftDirty(false);
      }
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  function restoreDraft() {
    if (!recoverableDraft) return;
    setSelectedPath(recoverableDraft.relativePath);
    setCollection(recoverableDraft.collection);
    setDraft(recoverableDraft.request);
    setDraftDirty(true);
    setRecoverableDraft(null);
    setResponse(null);
    setHttpError(null);
  }

  function discardRecoveredDraft() {
    if (workspace) window.localStorage.removeItem(draftStorageKey(workspace.config.id));
    setRecoverableDraft(null);
  }

  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        document.querySelector<HTMLInputElement>("#workspace-search")?.focus();
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
  }

  return (
    <main className="app-shell">
      <a className="skip-link" href="#main-workspace">Перейти к редактору</a>
      <header className="topbar">
        <div className="topbar-leading">
          <div className="brand" aria-label="ReqVault"><ReqVaultMark className="brand-mark" /><span>ReqVault</span><em>1.0</em></div>
          {workspace && <div className="workspace-breadcrumb"><span>/</span><strong>{workspace.config.name}</strong></div>}
        </div>
        <div className="topbar-actions">
          {workspace && <button className="command-trigger" type="button" onClick={() => document.querySelector<HTMLInputElement>("#workspace-search")?.focus()}><Icon name="search" /><span>Быстрый поиск</span><kbd>Ctrl K</kbd></button>}
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
            onSelectRequest={selectRequest}
            onNewRequest={newRequest}
            onEnvironmentChange={setActiveEnvironment}
            onEditEnvironments={() => { setEnvironmentError(null); setEnvironmentsOpen(true); }}
            onEditSecrets={() => void openSecrets()}
            onClose={closeWorkspace}
          />
          <div className="main-panel" id="main-workspace" tabIndex={-1}>
            {externalChange && <div className="external-change-banner" role="status"><div><strong>Файлы workspace изменились вне ReqVault</strong><p>{draftDirty ? "Перезагрузка заменит текущий несохранённый черновик." : "Перезагрузите данные, чтобы увидеть актуальную версию."}</p></div><button className="secondary-button" type="button" onClick={() => void reloadExternalChanges()}>Перезагрузить</button></div>}
            {recoverableDraft && <div className="draft-recovery-banner" role="status"><div><strong>Найден несохранённый черновик</strong><p>Сохранён {new Intl.DateTimeFormat("ru-RU", { dateStyle: "short", timeStyle: "short" }).format(recoverableDraft.updatedAt)}. Credential были удалены из локальной копии.</p></div><div className="inline-actions"><button className="primary-button" type="button" onClick={restoreDraft}>Восстановить</button><button className="secondary-button" type="button" onClick={discardRecoveredDraft}>Удалить</button></div></div>}
            {draft ? (
              <div className="request-workbench">
                {importStatus && <div className="success-banner">{importStatus}</div>}
                <RequestEditor request={draft} relativePath={selectedPath} collection={collection} saving={busy} sending={sending} error={error} securityReport={securityReport} copyStatus={copyStatus} dirty={draftDirty} onChange={updateDraft} onCollectionChange={(value) => { setCollection(value); setDraftDirty(true); }} onSave={() => void persistRequest()} onDelete={() => void deleteCurrentRequest()} onSend={() => void sendCurrentRequest()} onCopyCurl={() => void copyCurl()} onAuthorizeOAuth={() => void authorizeCurrentOAuth()} onRefreshOAuth={() => void refreshCurrentOAuth()} oauthBusy={oauthBusy} oauthStatus={oauthStatus} />
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
