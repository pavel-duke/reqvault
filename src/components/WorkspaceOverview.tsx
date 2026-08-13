import type { WorkspaceSnapshot } from "../types";
import { Icon } from "./Icon";

type Props = {
  workspace: WorkspaceSnapshot;
  onNewRequest: () => void;
  onImport: () => void;
  onRun: () => void;
};

export function WorkspaceOverview({ workspace, onNewRequest, onImport, onRun }: Props) {
  return (
    <section className="workspace-overview" aria-labelledby="workspace-overview-title">
      <header className="overview-header">
        <div>
          <span className="eyebrow">Рабочая область</span>
          <h1 id="workspace-overview-title">{workspace.config.name}</h1>
          <p>Создайте первый запрос или импортируйте готовую спецификацию API.</p>
        </div>
        <span className="overview-local"><Icon name="shield" /> Данные хранятся локально</span>
      </header>

      <div className="overview-stats" aria-label="Сводка workspace">
        <article><strong>{workspace.requests.length}</strong><span>запросов</span></article>
        <article><strong>{workspace.environments.length}</strong><span>окружений</span></article>
        <article><strong>{workspace.config.production_guard.enabled ? "On" : "Off"}</strong><span>Production Guard</span></article>
      </div>

      <div className="overview-grid">
        <article className="overview-primary-card">
          <span className="overview-card-icon"><Icon name="file" /></span>
          <div>
            <span className="eyebrow">Быстрый старт</span>
            <h2>Подготовьте первый запрос</h2>
            <p>Метод, URL, авторизация, тело и проверки находятся в одном редакторе.</p>
          </div>
          <button className="primary-button" type="button" onClick={onNewRequest}>Новый запрос</button>
        </article>
        <article className="overview-card">
          <Icon name="download" />
          <div><h2>Импортировать API</h2><p>OpenAPI, Postman Collection, cURL или ReqVault bundle.</p></div>
          <button className="secondary-button" type="button" onClick={onImport}>Выбрать файл</button>
        </article>
        <article className="overview-card">
          <Icon name="activity" />
          <div><h2>Проверить коллекцию</h2><p>Запустите запросы и assertions последовательно или параллельно.</p></div>
          <button className="secondary-button" type="button" onClick={onRun} disabled={workspace.requests.length === 0}>Открыть runner</button>
        </article>
      </div>

      <footer className="overview-hints">
        <span><kbd>Alt N</kbd> новый запрос</span>
        <span><kbd>Ctrl K</kbd> быстрый поиск</span>
        <span><kbd>Ctrl Enter</kbd> отправить запрос</span>
      </footer>
    </section>
  );
}
