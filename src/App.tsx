import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  createWorkspace,
  errorMessage,
  generateSafeCurl,
  inspectRequest,
  listSecrets,
  openWorkspace,
  removeEnvironment,
  removeRequest,
  removeSecret,
  saveEnvironment,
  sendHttpRequest,
  saveRequest,
  saveSecret,
} from "./api";
import { EnvironmentDialog } from "./components/EnvironmentDialog";
import { RequestEditor } from "./components/RequestEditor";
import { ResponseViewer } from "./components/ResponseViewer";
import { SecretDialog } from "./components/SecretDialog";
import { Sidebar } from "./components/Sidebar";
import { StartScreen } from "./components/StartScreen";
import { collectionFromPath, emptyRequest } from "./request-utils";
import type { EnvironmentFile, HttpError, HttpResponse, RequestFile, RequestSummary, SecurityReport, Theme, WorkspaceSnapshot } from "./types";
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

  function applyWorkspace(snapshot: WorkspaceSnapshot) {
    setWorkspace(snapshot);
    window.localStorage.setItem(LAST_WORKSPACE_KEY, snapshot.root_path);
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
  }

  function newRequest() {
    setSelectedPath(null);
    setDraft(emptyRequest());
    setCollection("Общее");
    setError(null);
    setResponse(null);
    setHttpError(null);
  }

  async function sendCurrentRequest() {
    if (!workspace || !draft) return;
    const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
    setSending(true);
    setHttpError(null);
    try {
      setResponse(await sendHttpRequest(draft, environment, workspace.config.id));
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

  function closeWorkspace() {
    window.localStorage.removeItem(LAST_WORKSPACE_KEY);
    setWorkspace(null);
    setDraft(null);
    setSelectedPath(null);
    setError(null);
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand" aria-label="ReqVault"><span className="brand-mark" aria-hidden="true">RV</span><span>ReqVault</span></div>
        <button className="icon-button" type="button" onClick={() => setTheme(theme === "dark" ? "light" : "dark")} aria-label={theme === "dark" ? "Включить светлую тему" : "Включить тёмную тему"} title={theme === "dark" ? "Светлая тема" : "Тёмная тема"}>{theme === "dark" ? "☀" : "☾"}</button>
      </header>

      {!workspace ? (
        <StartScreen busy={busy} error={error} onCreate={() => void pickWorkspace("create")} onOpen={() => void pickWorkspace("open")} />
      ) : (
        <div className="workspace-shell">
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
          <div className="main-panel">
            {draft ? (
              <div className="request-workbench">
                <RequestEditor request={draft} relativePath={selectedPath} collection={collection} saving={busy} sending={sending} error={error} securityReport={securityReport} copyStatus={copyStatus} onChange={setDraft} onCollectionChange={setCollection} onSave={() => void persistRequest()} onDelete={() => void deleteCurrentRequest()} onSend={() => void sendCurrentRequest()} onCopyCurl={() => void copyCurl()} />
                <ResponseViewer response={response} error={httpError} loading={sending} />
              </div>
            ) : (
              <section className="editor-empty"><div><span className="empty-icon">→</span><h1>Выбери запрос</h1><p>Или создай новый запрос в текущем workspace.</p><button className="primary-button" type="button" onClick={newRequest}>Новый запрос</button></div></section>
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
    </main>
  );
}

export default App;
