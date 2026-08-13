import { useMemo, useState } from "react";
import type { WorkspaceDiagnostics } from "../types";

type Props = {
  report: WorkspaceDiagnostics | null;
  busy: boolean;
  error: string | null;
  backupId: string | null;
  onRefresh: () => void;
  onMigrate: () => Promise<void>;
  onRollback: (backupId: string) => Promise<void>;
  onClose: () => void;
};

type Filter = "all" | "error" | "warning" | "info";

export function DiagnosticsDialog({ report, busy, error, backupId, onRefresh, onMigrate, onRollback, onClose }: Props) {
  const [filter, setFilter] = useState<Filter>("all");
  const visible = useMemo(() => report?.issues.filter((issue) => filter === "all" || issue.severity === filter) ?? [], [filter, report]);
  return (
    <div className="modal-backdrop" role="presentation">
      <section className="modal-card diagnostics-modal" role="dialog" aria-modal="true" aria-labelledby="diagnostics-title">
        <header className="modal-header"><div><span className="eyebrow">Надёжность workspace</span><h2 id="diagnostics-title">Диагностика</h2></div><button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button></header>
        {report && <div className="diagnostic-metrics">
          <div><strong>{report.requests}</strong><span>запросов</span></div>
          <div><strong>{report.environments}</strong><span>окружений</span></div>
          <div className={report.errors ? "metric-error" : ""}><strong>{report.errors}</strong><span>ошибок</span></div>
          <div className={report.warnings ? "metric-warning" : ""}><strong>{report.warnings}</strong><span>предупреждений</span></div>
        </div>}
        {error && <div className="error-banner" role="alert">{error}</div>}
        {report?.migration.required && <section className="migration-card">
          <div><span className="eyebrow">Миграция {report.migration.current_version} → {report.migration.target_version}</span><strong>Нужно обновить {report.migration.files.length} файлов</strong><p>{report.migration.changes.join(" ")}</p></div>
          <button className="primary-button" type="button" disabled={busy || report.migration.warnings.length > 0} onClick={() => void onMigrate()}>Создать backup и обновить</button>
          {report.migration.warnings.map((warning) => <small key={warning}>{warning}</small>)}
        </section>}
        {backupId && <div className="rollback-card"><span>Создан backup <code>{backupId}</code></span><button className="secondary-button" type="button" disabled={busy} onClick={() => void onRollback(backupId)}>Откатить миграцию</button></div>}
        <div className="diagnostic-toolbar">
          <div role="tablist" aria-label="Фильтр диагностики">{(["all", "error", "warning", "info"] as const).map((value) => <button role="tab" aria-selected={filter === value} className={filter === value ? "active" : ""} type="button" key={value} onClick={() => setFilter(value)}>{value === "all" ? "Все" : value === "error" ? "Ошибки" : value === "warning" ? "Предупреждения" : "Информация"}</button>)}</div>
          <button className="secondary-button" type="button" onClick={onRefresh} disabled={busy}>{busy ? "Проверяю…" : "Проверить снова"}</button>
        </div>
        <div className="diagnostic-list">
          {!busy && report && visible.length === 0 && <div className="diagnostic-ok"><strong>{report.issues.length ? "По фильтру проблем нет" : "Workspace в порядке"}</strong><p>YAML, ссылки, внешние файлы и Secret Vault проверены.</p></div>}
          {visible.map((issue, index) => <article className={`diagnostic-issue issue-${issue.severity}`} key={`${issue.code}-${issue.path}-${index}`}><span className="issue-mark" aria-hidden="true">{issue.severity === "error" ? "!" : issue.severity === "warning" ? "△" : "i"}</span><div><strong>{issue.message}</strong><code>{issue.path}</code><p>{issue.remediation}</p></div></article>)}
          {busy && <div className="response-state"><span className="spinner" aria-hidden="true" /><p>Проверяю workspace…</p></div>}
        </div>
        <footer className="modal-actions"><span className="help-text">Проверка не читает значения секретов.</span><button className="secondary-button" type="button" onClick={onClose}>Закрыть</button></footer>
      </section>
    </div>
  );
}
