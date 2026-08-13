import type { EnvironmentSummary, RequestSummary, WorkspaceSnapshot } from "../types";

type Props = {
  workspace: WorkspaceSnapshot;
  selectedPath: string | null;
  activeEnvironment: string;
  onSelectRequest: (summary: RequestSummary) => void;
  onNewRequest: () => void;
  onEnvironmentChange: (path: string) => void;
  onEditEnvironments: () => void;
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
  onClose,
}: Props) {
  const groups = groupedRequests(workspace.requests);

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

      <div className="sidebar-section-heading">
        <span>Запросы</span>
        <button className="quiet-icon" type="button" onClick={onNewRequest} title="Новый запрос" aria-label="Новый запрос">+</button>
      </div>

      <nav className="request-tree" aria-label="Запросы workspace">
        {groups.length === 0 && (
          <div className="sidebar-empty">
            <p>Здесь пока пусто</p>
            <button className="text-button" type="button" onClick={onNewRequest}>Создать запрос</button>
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
