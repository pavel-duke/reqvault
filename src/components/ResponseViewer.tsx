import { useMemo, useState } from "react";
import type { HttpError, HttpResponse } from "../types";

type Props = {
  response: HttpResponse | null;
  error: HttpError | null;
  loading: boolean;
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

export function ResponseViewer({ response, error, loading }: Props) {
  const [tab, setTab] = useState<ResponseTab>("body");
  const body = useMemo(() => (response ? prettyBody(response) : ""), [response]);

  return (
    <section className="response-viewer" aria-live="polite">
      <header className="response-heading">
        <div><span className="eyebrow">Результат</span><h2>Ответ</h2></div>
        {response && (
          <div className="response-metrics">
            <strong className={response.status < 400 ? "status-ok" : "status-error"}>{response.status} {response.status_text}</strong>
            <span>{response.duration_ms} мс</span>
            <span>{formatBytes(response.size_bytes)}</span>
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
          <div className="editor-tabs response-tabs" role="tablist" aria-label="Данные ответа">
            <button className={tab === "body" ? "active" : ""} type="button" onClick={() => setTab("body")}>Ответ</button>
            <button className={tab === "headers" ? "active" : ""} type="button" onClick={() => setTab("headers")}>Заголовки <span>{response.headers.length}</span></button>
            <button className={tab === "raw" ? "active" : ""} type="button" onClick={() => setTab("raw")}>Raw</button>
          </div>
          {tab === "headers" ? (
            <dl className="response-headers">
              {response.headers.map((header, index) => <div key={`${header.name}-${index}`}><dt>{header.name}</dt><dd>{header.value}</dd></div>)}
            </dl>
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
