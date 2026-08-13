import { useState } from "react";

type Props = {
  busy: boolean;
  error: string | null;
  onImport: (command: string) => void;
  onClose: () => void;
};

export function CurlImportDialog({ busy, error, onImport, onClose }: Props) {
  const [command, setCommand] = useState("");

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="modal-card curl-dialog" role="dialog" aria-modal="true" aria-labelledby="curl-dialog-title">
        <div className="modal-header">
          <div>
            <span className="eyebrow">Импорт</span>
            <h2 id="curl-dialog-title">Команда cURL</h2>
          </div>
          <button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button>
        </div>
        <p className="help-text">Вставь команду из документации или DevTools. Токены, cookie и пароли не попадут в YAML.</p>
        {error && <div className="error-banner" role="alert">{error}</div>}
        <textarea
          className="code-input curl-input"
          value={command}
          onChange={(event) => setCommand(event.currentTarget.value)}
          placeholder={'curl -X POST "https://api.example.test/users" \\\n  -H "Content-Type: application/json" \\\n  -d \'{"name":"Pavel"}\''}
          spellCheck={false}
          autoFocus
        />
        <div className="modal-actions">
          <button className="secondary-button" type="button" onClick={onClose}>Отмена</button>
          <button className="primary-button" type="button" disabled={busy || !command.trim()} onClick={() => onImport(command)}>
            {busy ? "Импортирую…" : "Импортировать"}
          </button>
        </div>
      </section>
    </div>
  );
}
