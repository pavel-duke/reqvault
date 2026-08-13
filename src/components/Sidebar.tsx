import { useMemo, useState } from "react";
import type { EnvironmentSummary, RequestSummary, WorkspaceSnapshot } from "../types";

type Props = {
  workspace: WorkspaceSnapshot;
  selectedPath: string | null;
  activeEnvironment: string;
  onSelectRequest: (summary: RequestSummary) => void;
  onNewRequest: () => void;
  onEnvironmentChange: (path: string) => void;
  onEditEnvironments: () => void;
  onEditSecrets: () => void;
  onImport: () => void;
  onImportCurl: () => void;
  onExport: () => void;
  onSettings: () => void;
  onHistory: () => void;
  onCookies: () => void;
  onDiagnostics: () => void;
  onRun: () => void;
  onStream: () => void;
  onClose: () => void;
};

function groupedRequests(requests: RequestSummary[]) {
  const groups = new Map<string, RequestSummary[]>();
  for (const request of requests) {
    const parts = request.relative_path.split("/");
    const group = parts.length > 2 ? parts[1] : "Общее";
    groups.set(group, [...(groups.get(group) ?? []), request]);
  }
  return [...groups.entries()];
}

function environmentLabel(environment: EnvironmentSummary) {
  return environment.environment.name || environment.relative_path;
}

export function Sidebar({
  workspace,
  selectedPath,
  activeEnvironment,
  onSelectRequest,
  onNewRequest,
  onEnvironmentChange,
  onEditEnvironments,
  onEditSecrets,
  onImport,
  onImportCurl,
  onExport,
  onSettings,
  onHistory,
  onCookies,
  onDiagnostics,
  onRun,
  onStream,
  onClose,
}: Props) {
  const [search, setSearch] = useState("");
  const normalizedSearch = search.trim().toLocaleLowerCase("ru");
  const filteredRequests = useMemo(() => {
    if (!normalizedSearch) return workspace.requests;
    return workspace.requests.filter((summary) =>
      [summary.request.name, summary.request.url, summary.request.method, summary.relative_path]
        .some((value) => value.toLocaleLowerCase("ru").includes(normalizedSearch)),
    );
  }, [normalizedSearch, workspace.requests]);
  const matchingEnvironments = normalizedSearch
    ? workspace.environments.filter((environment) =>
        [environment.environment.name, environment.relative_path]
          .some((value) => value.toLocaleLowerCase("ru").includes(normalizedSearch)),
      )
    : [];
  const groups = groupedRequests(filteredRequests);

  return (
    <aside className="sidebar">
      <div className="workspace-heading">
        <div>
          <span className="eyebrow">Workspace</span>
          <strong title={workspace.root_path}>{workspace.config.name}</strong>
        </div>
        <button className="quiet-icon" type="button" onClick={onClose} title="Закрыть workspace" aria-label="Закрыть workspace">×</button>
      </div>

      <div className="environment-select">
        <label htmlFor="active-environment">Окружение</label>
        <div className="select-line">
          <select
            id="active-environment"
            value={activeEnvironment}
            onChange={(event) => onEnvironmentChange(event.currentTarget.value)}
          >
            {workspace.environments.map((environment) => (
              <option key={environment.relative_path} value={environment.relative_path}>
                {environmentLabel(environment)}
              </option>
            ))}
          </select>
          <button className="quiet-icon" type="button" onClick={onEditEnvironments} title="Изменить окружения" aria-label="Изменить окружения">•••</button>
        </div>
      </div>

      <button className="vault-button" type="button" onClick={onEditSecrets}>
        <span aria-hidden="true">▣</span>
        <span><strong>Секреты</strong><small>Системное хранилище ОС</small></span>
      </button>

      <div className="sidebar-tools">
        <button className="secondary-button" type="button" onClick={onImport}>Файл</button>
        <button className="secondary-button" type="button" onClick={onImportCurl}>cURL</button>
        <button className="secondary-button" type="button" onClick={onExport}>Экспорт</button>
        <button className="secondary-button" type="button" onClick={onHistory}>История</button>
        <button className="secondary-button" type="button" onClick={onCookies}>Cookie</button>
        <button className="secondary-button" type="button" onClick={onDiagnostics}>Диагностика</button>
        <button className="secondary-button" type="button" onClick={onRun}>Запуск</button>
        <button className="secondary-button" type="button" onClick={onStream}>Потоки</button>
        <button className="secondary-button guard-button" type="button" onClick={onSettings}>
          Защита{workspace.config.production_guard.enabled ? " •" : ""}
        </button>
      </div>

      <div className="sidebar-section-heading">
        <span>Запросы</span>
        <button className="quiet-icon" type="button" data-shortcut="new-request" aria-keyshortcuts="Alt+N" onClick={onNewRequest} title="Новый запрос (Alt+N)" aria-label="Новый запрос">+</button>
      </div>

      <label className="sidebar-search">
        <span className="sr-only">Поиск запросов и окружений</span>
        <input id="workspace-search" value={search} onChange={(event) => setSearch(event.currentTarget.value)} placeholder="Поиск по запросам и URL" />
        {search && <button type="button" onClick={() => setSearch("")} aria-label="Очистить поиск">×</button>}
      </label>
      {matchingEnvironments.length > 0 && <div className="environment-results"><small>Окружения</small>{matchingEnvironments.map((environment) => <button type="button" key={environment.relative_path} onClick={() => onEnvironmentChange(environment.relative_path)}>{environmentLabel(environment)}</button>)}</div>}

      <nav className="request-tree" aria-label="Запросы workspace">
        {groups.length === 0 && (
          <div className="sidebar-empty">
            <p>{search ? "Запросы не найдены" : "Здесь пока пусто"}</p>
            {!search && <button className="text-button" type="button" onClick={onNewRequest}>Создать запрос</button>}
          </div>
        )}
        {groups.map(([group, requests]) => (
          <div className="request-group" key={group}>
            <h2>{group}</h2>
            {requests.map((summary) => (
              <button
                type="button"
                className={`request-item ${selectedPath === summary.relative_path ? "selected" : ""}`}
                key={summary.relative_path}
                onClick={() => onSelectRequest(summary)}
              >
                <span className={`method method-${summary.request.method.toLowerCase()}`}>
                  {summary.request.method}
                </span>
                <span>{summary.request.name}</span>
              </button>
            ))}
          </div>
        ))}
      </nav>
    </aside>
  );
}
