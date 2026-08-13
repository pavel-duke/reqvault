import { useState } from "react";
import type { WorkspaceConfig } from "../types";

type Props = {
  config: WorkspaceConfig;
  busy: boolean;
  error: string | null;
  onSave: (config: WorkspaceConfig) => void;
  onClose: () => void;
};

export function WorkspaceSettingsDialog({ config, busy, error, onSave, onClose }: Props) {
  const [draft, setDraft] = useState(() => structuredClone(config));
  const guard = draft.production_guard;
  const patchGuard = (patch: Partial<typeof guard>) => {
    setDraft({ ...draft, production_guard: { ...guard, ...patch } });
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="modal-card settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-dialog-title">
        <div className="modal-header">
          <div>
            <span className="eyebrow">Workspace</span>
            <h2 id="settings-dialog-title">Production Guard</h2>
          </div>
          <button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button>
        </div>
        <p className="help-text">Правила проверяются в Rust перед отправкой. Они помогают не отправить опасный запрос на случайный хост или по HTTP.</p>
        {error && <div className="error-banner" role="alert">{error}</div>}
        <label className="check-field guard-master">
          <input type="checkbox" checked={guard.enabled} onChange={(event) => patchGuard({ enabled: event.currentTarget.checked })} />
          Включить Production Guard
        </label>
        <div className="guard-grid" aria-disabled={!guard.enabled}>
          <label className="check-field"><input type="checkbox" checked={guard.require_https} disabled={!guard.enabled} onChange={(event) => patchGuard({ require_https: event.currentTarget.checked })} /> Только HTTPS</label>
          <label className="check-field"><input type="checkbox" checked={guard.block_secrets_in_url} disabled={!guard.enabled} onChange={(event) => patchGuard({ block_secrets_in_url: event.currentTarget.checked })} /> Запретить секреты в URL</label>
          <label className="field">
            <span>Разрешённые хосты</span>
            <textarea value={guard.allowed_hosts.join("\n")} disabled={!guard.enabled} onChange={(event) => patchGuard({ allowed_hosts: event.currentTarget.value.split(/\r?\n|,/).map((value) => value.trim()).filter(Boolean) })} placeholder={'api.example.com\n*.service.example.com'} />
            <small>Один хост в строке. Поддерживается маска <code>*.example.com</code>. Пустой список разрешает любой хост.</small>
          </label>
          <label className="field">
            <span>Заблокированные методы</span>
            <input value={guard.blocked_methods.join(", ")} disabled={!guard.enabled} onChange={(event) => patchGuard({ blocked_methods: event.currentTarget.value.split(",").map((value) => value.trim()).filter(Boolean) })} placeholder="DELETE, PATCH" />
          </label>
        </div>
        <div className="modal-actions">
          <button className="secondary-button" type="button" onClick={onClose}>Отмена</button>
          <button className="primary-button" type="button" disabled={busy} onClick={() => onSave(draft)}>{busy ? "Сохраняю…" : "Сохранить"}</button>
        </div>
      </section>
    </div>
  );
}
