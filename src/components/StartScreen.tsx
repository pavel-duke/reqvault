import { Icon, ReqVaultMark } from "./Icon";

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
      <div className="start-layout">
        <div className="start-hero">
          <ReqVaultMark className="start-mark" />
          <span className="start-kicker">API workspace, который принадлежит вам</span>
          <h1>Проектируйте и проверяйте API без облачной зависимости</h1>
          <p>Запросы остаются в понятных YAML-файлах, секреты — в хранилище операционной системы. Работайте локально, храните проект в Git и запускайте те же проверки в CI.</p>
          {error && <div className="error-banner" role="alert">{error}</div>}
          <div className="start-actions">
            <button className="primary-button" type="button" onClick={onCreate} disabled={busy}><Icon name="folder" />{busy ? "Открываю…" : "Создать workspace"}</button>
            <button className="secondary-button" type="button" onClick={onOpen} disabled={busy}><Icon name="archive" />Открыть папку</button>
            <button className="text-button" type="button" onClick={onImport} disabled={busy}>Импортировать bundle</button>
          </div>
        </div>
        <div className="start-capabilities" aria-label="Главные возможности">
          <article><span><Icon name="key" /></span><div><strong>Secret Vault</strong><p>Credential не попадают в YAML, историю и Git.</p></div></article>
          <article><span><Icon name="shield" /></span><div><strong>Production Guard</strong><p>HTTPS, allowlist хостов и защита опасных методов.</p></div></article>
          <article><span><Icon name="activity" /></span><div><strong>API tests и CLI</strong><p>Одинаковые проверки в приложении и локальном CI.</p></div></article>
          <article><span><Icon name="pulse" /></span><div><strong>REST, GraphQL и потоки</strong><p>HTTP, WebSocket и SSE в одном workspace.</p></div></article>
          <footer><span className="status-dot" />Аккаунт и собственный backend не требуются</footer>
        </div>
      </div>
    </section>
  );
}
