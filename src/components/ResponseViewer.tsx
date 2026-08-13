import { useMemo, useState } from "react";
import type { HttpError, HttpResponse } from "../types";

type Props = {
  response: HttpResponse | null;
  error: HttpError | null;
  loading: boolean;
  onExport?: (format: "body" | "http" | "har") => void;
  onSaveFixture?: () => void;
  onCompare?: () => void;
  actionStatus?: string | null;
};

type ResponseTab = "body" | "headers" | "raw";

function formatBytes(value: number): string {
  if (value < 1024) return `${value} Б`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} КБ`;
  return `${(value / (1024 * 1024)).toFixed(1)} МБ`;
}

function prettyBody(response: HttpResponse): string {
  if (!response.is_json) return response.body;
  try {
    return JSON.stringify(JSON.parse(response.body), null, 2);
  } catch {
    return response.body;
  }
}

function graphqlErrors(response: HttpResponse | null): string[] {
  if (!response?.is_json) return [];
  try {
    const value = JSON.parse(response.body) as { errors?: unknown };
    if (!Array.isArray(value.errors)) return [];
    return value.errors.map((item) => {
      if (!item || typeof item !== "object") return String(item);
      const error = item as { message?: unknown; path?: unknown };
      const message = typeof error.message === "string" ? error.message : "GraphQL error";
      const path = Array.isArray(error.path) ? ` (${error.path.join(".")})` : "";
      return `${message}${path}`;
    });
  } catch {
    return [];
  }
}

export function ResponseViewer({ response, error, loading, onExport, onSaveFixture, onCompare, actionStatus }: Props) {
  const [tab, setTab] = useState<ResponseTab>("body");
  const body = useMemo(() => (response ? prettyBody(response) : ""), [response]);
  const graphQLErrors = useMemo(() => graphqlErrors(response), [response]);

  return (
    <section className="response-viewer" aria-live="polite">
      <header className="response-heading">
        <div><span className="eyebrow">Результат</span><h2>Ответ</h2></div>
        {response && (
          <div className="response-heading-side">
            <div className="response-metrics">
              <strong className={response.status < 400 ? "status-ok" : "status-error"}>{response.status} {response.status_text}</strong>
              <span>{response.duration_ms} мс</span>
              <span>{formatBytes(response.size_bytes)}</span>
              <span>{response.content_type}</span>
            </div>
            {(onExport || onSaveFixture) && <div className="response-actions">
              {onExport && <details><summary>Экспорт</summary><div className="response-menu"><button type="button" onClick={() => onExport("body")}>Только тело</button><button type="button" onClick={() => onExport("http")}>Полный HTTP</button><button type="button" onClick={() => onExport("har")}>Безопасный HAR</button></div></details>}
              {onCompare && <button className="secondary-button" type="button" onClick={onCompare}>Сравнить</button>}
              {onSaveFixture && <button className="secondary-button" type="button" onClick={onSaveFixture} disabled={response.truncated}>В fixture</button>}
            </div>}
          </div>
        )}
      </header>

      {loading ? (
        <div className="response-state"><span className="spinner" aria-hidden="true" /> <p>Жду ответ сервера…</p></div>
      ) : error ? (
        <div className="response-error" role="alert">
          <h3>{error.message}</h3>
          <p>{error.error_type === "connection" ? "Проверь адрес сервера и подключение к сети." : "Проверь параметры запроса и попробуй ещё раз."}</p>
          {error.details && <details><summary>Технические подробности</summary><pre>{error.details}</pre></details>}
        </div>
      ) : response ? (
        <>
          {actionStatus && <div className="success-banner response-action-status">{actionStatus}</div>}
          {response.truncated && <div className="response-warning" role="status"><strong>Показан только безопасный preview до 8 МБ.</strong> Полный ответ не загружался в память. Экспорт и fixture содержат только полученную часть.</div>}
          {graphQLErrors.length > 0 && <div className="graphql-errors" role="alert"><strong>GraphQL вернул errors</strong><ul>{graphQLErrors.map((message, index) => <li key={index}>{message}</li>)}</ul></div>}
          <div className="editor-tabs response-tabs" role="tablist" aria-label="Данные ответа">
            <button className={tab === "body" ? "active" : ""} type="button" onClick={() => setTab("body")}>Ответ</button>
            <button className={tab === "headers" ? "active" : ""} type="button" onClick={() => setTab("headers")}>Заголовки <span>{response.headers.length}</span></button>
            <button className={tab === "raw" ? "active" : ""} type="button" onClick={() => setTab("raw")}>Raw</button>
          </div>
          {tab === "headers" ? (
            <dl className="response-headers">
              {response.headers.map((header, index) => <div key={`${header.name}-${index}`}><dt>{header.name}</dt><dd>{header.value}</dd></div>)}
            </dl>
          ) : tab === "body" && response.body_kind === "image" ? (
            <div className="response-media"><img src={`data:${response.content_type};base64,${response.body}`} alt="Ответ API" /></div>
          ) : tab === "body" && response.body_kind === "html" ? (
            <div className="response-html"><iframe title="HTML preview ответа" sandbox="" srcDoc={response.body} /></div>
          ) : tab === "body" && response.body_kind === "binary" ? (
            <div className="response-binary"><span aria-hidden="true">01</span><div><strong>Бинарный ответ</strong><p>{response.content_type} · {formatBytes(response.size_bytes)}</p><small>Данные хранятся в памяти как Base64. Используйте экспорт тела, чтобы сохранить исходные байты.</small></div></div>
          ) : (
            <pre className="response-body">{tab === "body" ? (body || "Ответ не содержит тела") : (response.body || "")}</pre>
          )}
        </>
      ) : (
        <div className="response-state"><span className="response-placeholder" aria-hidden="true">↯</span><p>Ответ появится здесь после отправки запроса.</p></div>
      )}
    </section>
  );
}
