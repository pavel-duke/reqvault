import { useEffect, useMemo, useState } from "react";
import { recordToRows, rowsToRecord } from "../request-utils";
import type { EnvironmentFile, EnvironmentSummary, KeyValue } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";

type Props = {
  environments: EnvironmentSummary[];
  activePath: string;
  busy: boolean;
  error: string | null;
  onSave: (path: string | null, environment: EnvironmentFile) => void;
  onDelete: (path: string) => void;
  onClose: () => void;
};

export function EnvironmentDialog({ environments, activePath, busy, error, onSave, onDelete, onClose }: Props) {
  const selected = useMemo(
    () => environments.find((item) => item.relative_path === activePath) ?? environments[0],
    [activePath, environments],
  );
  const [relativePath, setRelativePath] = useState<string | null>(selected?.relative_path ?? null);
  const [name, setName] = useState(selected?.environment.name ?? "local");
  const [rows, setRows] = useState<KeyValue[]>(recordToRows(selected?.environment.variables ?? {}));

  useEffect(() => {
    setRelativePath(selected?.relative_path ?? null);
    setName(selected?.environment.name ?? "local");
    setRows(recordToRows(selected?.environment.variables ?? {}));
  }, [selected]);

  function choose(path: string) {
    const item = environments.find((environment) => environment.relative_path === path);
    if (!item) return;
    setRelativePath(item.relative_path);
    setName(item.environment.name);
    setRows(recordToRows(item.environment.variables));
  }

  function startNew() {
    setRelativePath(null);
    setName("testing");
    setRows([]);
  }

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="modal-card environment-modal" role="dialog" aria-modal="true" aria-labelledby="environment-title">
        <header className="modal-header">
          <div><span className="eyebrow">Workspace</span><h2 id="environment-title">Окружения</h2></div>
          <button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button>
        </header>
        <div className="environment-layout">
          <nav className="environment-list">
            {environments.map((environment) => (
              <button className={relativePath === environment.relative_path ? "selected" : ""} type="button" key={environment.relative_path} onClick={() => choose(environment.relative_path)}>{environment.environment.name}</button>
            ))}
            <button className="new-environment" type="button" onClick={startNew}>+ Новое окружение</button>
          </nav>
          <div className="environment-form">
            <label className="field"><span>Название</span><input value={name} onChange={(event) => setName(event.currentTarget.value)} /></label>
            <div className="field-label">Переменные</div>
            <KeyValueEditor rows={rows} onChange={setRows} namePlaceholder="BASE_URL" valuePlaceholder="https://api.example.ru" emptyText="Переменных пока нет" />
            <p className="help-text">Секреты здесь не хранятся. Для них используется отдельный раздел.</p>
            {error && <div className="error-banner" role="alert">{error}</div>}
            <div className="modal-actions">
              {relativePath && environments.length > 1 && <button className="danger-button" type="button" onClick={() => onDelete(relativePath)} disabled={busy}>Удалить</button>}
              <span />
              <button className="secondary-button" type="button" onClick={onClose}>Отмена</button>
              <button className="primary-button" type="button" disabled={busy || !name.trim()} onClick={() => onSave(relativePath, { format_version: 1, name, variables: rowsToRecord(rows) })}>{busy ? "Сохраняю…" : "Сохранить"}</button>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
