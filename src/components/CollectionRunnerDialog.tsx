import { useMemo, useState } from "react";
import type { CollectionRunOptions, CollectionRunReport, WorkspaceSnapshot } from "../types";

type Props = {
  workspace: WorkspaceSnapshot;
  activeEnvironment: string;
  busy: boolean;
  error: string | null;
  report: CollectionRunReport | null;
  onRun: (options: CollectionRunOptions) => void;
  onClose: () => void;
};

export function CollectionRunnerDialog({ workspace, activeEnvironment, busy, error, report, onRun, onClose }: Props) {
  const collections = useMemo(() => [...new Set(workspace.requests.map((item) => item.relative_path.split("/")[1] ?? "Общее"))].sort(), [workspace.requests]);
  const [collection, setCollection] = useState("");
  const [environment, setEnvironment] = useState(activeEnvironment);
  const [stopOnFailure, setStopOnFailure] = useState(false);

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="modal-card runner-dialog" role="dialog" aria-modal="true" aria-labelledby="runner-dialog-title">
        <div className="modal-header">
          <div><span className="eyebrow">API tests</span><h2 id="runner-dialog-title">Запуск коллекции</h2></div>
          <button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button>
        </div>
        <div className="runner-controls">
          <label className="field"><span>Коллекция</span><select value={collection} onChange={(event) => setCollection(event.currentTarget.value)}><option value="">Все запросы</option>{collections.map((name) => <option value={name} key={name}>{name}</option>)}</select></label>
          <label className="field"><span>Окружение</span><select value={environment} onChange={(event) => setEnvironment(event.currentTarget.value)}>{workspace.environments.map((item) => <option value={item.relative_path} key={item.relative_path}>{item.environment.name}</option>)}</select></label>
          <label className="check-field"><input type="checkbox" checked={stopOnFailure} onChange={(event) => setStopOnFailure(event.currentTarget.checked)} /> Остановиться после первой ошибки</label>
          <button className="primary-button" type="button" disabled={busy || workspace.requests.length === 0} onClick={() => onRun({ collection: collection || null, environment: environment || null, stop_on_failure: stopOnFailure })}>{busy ? "Выполняю…" : "Запустить"}</button>
        </div>
        {error && <div className="error-banner runner-error" role="alert">{error}</div>}
        <div className="runner-results">
          {!report && !busy && <div className="empty-inline">Добавь проверки в запросах или запусти коллекцию для базовой проверки HTTP-статусов.</div>}
          {report && <>
            <div className={`runner-summary ${report.failed ? "has-failures" : "all-passed"}`}>
              <strong>{report.failed ? `Ошибок: ${report.failed}` : "Все проверки прошли"}</strong>
              <span>Запросов: {report.total} · Успешно: {report.passed} · {report.duration_ms} мс</span>
            </div>
            <div className="runner-list">
              {report.results.map((result) => <div className={`runner-result ${result.passed ? "passed" : "failed"}`} key={result.relative_path}>
                <div className="runner-result-line"><span className="runner-mark">{result.passed ? "PASS" : "FAIL"}</span><span className={`method method-${result.method.toLowerCase()}`}>{result.method}</span><strong>{result.request_name}</strong><span>{result.status ?? "—"}</span><span>{result.duration_ms == null ? "—" : `${result.duration_ms} мс`}</span></div>
                {result.error && <p>{result.error}</p>}
                {result.assertions.filter((item) => !item.passed).map((item, index) => <p key={index}>{item.label}: ожидалось {item.expected}, получено {item.actual}</p>)}
              </div>)}
            </div>
          </>}
        </div>
        <div className="modal-actions"><button className="secondary-button" type="button" onClick={onClose}>Закрыть</button></div>
      </section>
    </div>
  );
}
