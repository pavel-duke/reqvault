type Props = {
  busy: boolean;
  error: string | null;
  onCreate: () => void;
  onOpen: () => void;
  onImport: () => void;
};

export function StartScreen({ busy, error, onCreate, onOpen, onImport }: Props) {
  return (
    <section className="start-screen">
      <div className="start-card">
        <span className="start-mark" aria-hidden="true">RV</span>
        <h1>Открой workspace</h1>
        <p>
          Запросы хранятся в обычных YAML-файлах. Токены и пароли остаются
          в системном хранилище ОС.
        </p>
        {error && <div className="error-banner" role="alert">{error}</div>}
        <div className="start-actions">
          <button className="primary-button" type="button" onClick={onCreate} disabled={busy}>
            {busy ? "Открываю…" : "Создать workspace"}
          </button>
          <button className="secondary-button" type="button" onClick={onOpen} disabled={busy}>
            Открыть папку
          </button>
          <button className="text-button" type="button" onClick={onImport} disabled={busy}>
            Импортировать bundle
          </button>
        </div>
      </div>
    </section>
  );
}
