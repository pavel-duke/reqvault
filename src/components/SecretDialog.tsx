import { useState, type FormEvent } from "react";

type Props = {
  names: string[];
  loading: boolean;
  error: string | null;
  onSave: (name: string, value: string) => Promise<boolean>;
  onDelete: (name: string) => void;
  onClose: () => void;
};

export function SecretDialog({ names, loading, error, onSave, onDelete, onClose }: Props) {
  const [name, setName] = useState("");
  const [value, setValue] = useState("");

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (await onSave(name, value)) {
      setName("");
      setValue("");
    }
  }

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="modal-card secret-modal" role="dialog" aria-modal="true" aria-labelledby="secret-title">
        <header className="modal-header">
          <div><span className="eyebrow">Secret Vault</span><h2 id="secret-title">Секреты</h2></div>
          <button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button>
        </header>
        <div className="secret-content">
          <p className="modal-intro">Значения хранятся через Credential Manager, Keychain или Secret Service. После сохранения ReqVault их не показывает.</p>
          <form className="secret-form" onSubmit={(event) => void submit(event)}>
            <label className="field"><span>Имя</span><input value={name} onChange={(event) => setName(event.currentTarget.value.toUpperCase())} placeholder="API_TOKEN" autoComplete="off" /></label>
            <label className="field"><span>Значение</span><input type="password" value={value} onChange={(event) => setValue(event.currentTarget.value)} placeholder="Введите токен или пароль" autoComplete="new-password" /></label>
            <button className="primary-button" type="submit" disabled={loading || !name.trim() || !value}>{loading ? "Сохраняю…" : "Сохранить"}</button>
          </form>
          {error && <div className="error-banner" role="alert">{error}</div>}
          <div className="secret-list-heading"><span>Сохранённые имена</span><span>{names.length}</span></div>
          <div className="secret-list">
            {names.length === 0 && <p className="inline-empty">Секретов пока нет</p>}
            {names.map((savedName) => (
              <div className="secret-item" key={savedName}>
                <span className="secret-symbol" aria-hidden="true">•••</span>
                <code>{savedName}</code>
                <button className="text-button" type="button" onClick={() => { setName(savedName); setValue(""); }}>Заменить</button>
                <button className="remove-row" type="button" onClick={() => onDelete(savedName)} aria-label={`Удалить ${savedName}`} title="Удалить">×</button>
              </div>
            ))}
          </div>
          <p className="help-text">В запросе используй <code>{"{{secret:API_TOKEN}}"}</code>.</p>
        </div>
      </section>
    </div>
  );
}
