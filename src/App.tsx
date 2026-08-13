import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  createWorkspace,
  errorMessage,
  openWorkspace,
  removeEnvironment,
  removeRequest,
  saveEnvironment,
  saveRequest,
} from "./api";
import { EnvironmentDialog } from "./components/EnvironmentDialog";
import { RequestEditor } from "./components/RequestEditor";
import { Sidebar } from "./components/Sidebar";
import { StartScreen } from "./components/StartScreen";
import { collectionFromPath, emptyRequest } from "./request-utils";
import type { EnvironmentFile, RequestFile, RequestSummary, Theme, WorkspaceSnapshot } from "./types";
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [environmentsOpen, setEnvironmentsOpen] = useState(false);
  const [environmentError, setEnvironmentError] = useState<string | null>(null);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem("reqvault.theme", theme);
  }, [theme]);

  useEffect(() => {
    const path = window.localStorage.getItem(LAST_WORKSPACE_KEY);
    if (!path) return;
    setBusy(true);
    openWorkspace(path)
      .then((snapshot) => applyWorkspace(snapshot))
      .catch(() => window.localStorage.removeItem(LAST_WORKSPACE_KEY))
      .finally(() => setBusy(false));
  }, []);

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
  }

  function newRequest() {
    setSelectedPath(null);
    setDraft(emptyRequest());
    setCollection("Общее");
    setError(null);
  }

  async function persistRequest() {
    if (!workspace || !draft) return;
    setBusy(true);
    setError(null);
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
            onClose={closeWorkspace}
          />
          <div className="main-panel">
            {draft ? (
              <RequestEditor request={draft} relativePath={selectedPath} collection={collection} saving={busy} error={error} onChange={setDraft} onCollectionChange={setCollection} onSave={() => void persistRequest()} onDelete={() => void deleteCurrentRequest()} />
            ) : (
              <section className="editor-empty"><div><span className="empty-icon">→</span><h1>Выбери запрос</h1><p>Или создай новый запрос в текущем workspace.</p><button className="primary-button" type="button" onClick={newRequest}>Новый запрос</button></div></section>
            )}
          </div>
        </div>
      )}

      {workspace && environmentsOpen && (
        <EnvironmentDialog environments={workspace.environments} activePath={activeEnvironment} busy={busy} error={environmentError} onSave={(path, environment) => void persistEnvironment(path, environment)} onDelete={(path) => void deleteSelectedEnvironment(path)} onClose={() => setEnvironmentsOpen(false)} />
      )}
    </main>
  );
}

export default App;
