import { useMemo, useState, type KeyboardEvent } from "react";
import { recordToRows, rowsToRecord } from "../request-utils";
import type { AuthConfig, BodyConfig, KeyValue, RequestFile, SecurityReport } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";
import { SecurityLens } from "./SecurityLens";

type EditorTab = "query" | "headers" | "auth" | "body";

type Props = {
  request: RequestFile;
  relativePath: string | null;
  collection: string;
  saving: boolean;
  sending: boolean;
  error: string | null;
  securityReport: SecurityReport | null;
  copyStatus: string | null;
  onChange: (request: RequestFile) => void;
  onCollectionChange: (collection: string) => void;
  onSave: () => void;
  onDelete: () => void;
  onSend: () => void;
  onCopyCurl: () => void;
};

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

function authForType(type: AuthConfig["type"]): AuthConfig {
  switch (type) {
    case "bearer": return { type, token: "{{secret:API_TOKEN}}" };
    case "basic": return { type, username: "", password: "{{secret:PASSWORD}}" };
    case "api_key_header": return { type, name: "X-API-Key", value: "{{secret:API_KEY}}" };
    case "api_key_query": return { type, name: "api_key", value: "{{secret:API_KEY}}" };
    default: return { type: "none" };
  }
}

function bodyForType(type: BodyConfig["type"]): BodyConfig {
  switch (type) {
    case "json": return { type, value: "{\n  \"key\": \"value\"\n}" };
    case "raw": return { type, value: "", content_type: "text/plain" };
    case "form_urlencoded": return { type, fields: [] };
    default: return { type: "none" };
  }
}

export function RequestEditor({
  request,
  relativePath,
  collection,
  saving,
  sending,
  error,
  securityReport,
  copyStatus,
  onChange,
  onCollectionChange,
  onSave,
  onDelete,
  onSend,
  onCopyCurl,
}: Props) {
  const [tab, setTab] = useState<EditorTab>("query");
  const headerRows = useMemo(() => recordToRows(request.headers), [request.headers]);

  function patch(patchValue: Partial<RequestFile>) {
    onChange({ ...request, ...patchValue });
  }

  function updateHeaders(rows: KeyValue[]) {
    patch({ headers: rowsToRecord(rows) });
  }

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      if (!sending && request.url.trim()) onSend();
    }
  }

  return (
    <section className="request-editor" onKeyDown={handleKeyDown}>
      <div className="request-meta-row">
        <input
          className="request-name"
          value={request.name}
          onChange={(event) => patch({ name: event.currentTarget.value })}
          aria-label="Название запроса"
        />
        <label className="compact-field">
          <span>Коллекция</span>
          <input
            value={collection}
            onChange={(event) => onCollectionChange(event.currentTarget.value)}
            disabled={relativePath !== null}
          />
        </label>
        <button className="secondary-button" type="button" onClick={onSave} disabled={saving || !request.name.trim()}>
          {saving ? "Сохраняю…" : "Сохранить"}
        </button>
        {relativePath && (
          <button className="danger-button" type="button" onClick={onDelete}>Удалить</button>
        )}
      </div>

      <div className="request-line">
        <select
          className={`method-select method-${request.method.toLowerCase()}`}
          value={request.method}
          onChange={(event) => patch({ method: event.currentTarget.value })}
          aria-label="HTTP-метод"
        >
          {METHODS.map((method) => <option value={method} key={method}>{method}</option>)}
        </select>
        <input
          className="url-input"
          value={request.url}
          onChange={(event) => patch({ url: event.currentTarget.value })}
          placeholder="https://api.example.ru/v1/users"
          aria-label="URL запроса"
        />
        <button className="send-button" type="button" onClick={onSend} disabled={sending || !request.url.trim()} title="Ctrl+Enter">
          {sending ? "Отправляю…" : "Отправить"}
        </button>
      </div>

      {error && <div className="error-banner editor-error" role="alert">{error}</div>}

      <div className="editor-tabs" role="tablist" aria-label="Настройки запроса">
        <button className={tab === "query" ? "active" : ""} type="button" onClick={() => setTab("query")}>Параметры <span>{request.query.length || ""}</span></button>
        <button className={tab === "headers" ? "active" : ""} type="button" onClick={() => setTab("headers")}>Заголовки <span>{Object.keys(request.headers).length || ""}</span></button>
        <button className={tab === "auth" ? "active" : ""} type="button" onClick={() => setTab("auth")}>Авторизация</button>
        <button className={tab === "body" ? "active" : ""} type="button" onClick={() => setTab("body")}>Тело</button>
      </div>

      <div className="tab-content">
        {tab === "query" && (
          <KeyValueEditor rows={request.query} onChange={(query) => patch({ query })} emptyText="Параметров запроса пока нет" />
        )}
        {tab === "headers" && (
          <KeyValueEditor rows={headerRows} onChange={updateHeaders} emptyText="Заголовков пока нет" />
        )}
        {tab === "auth" && (
          <div className="auth-editor">
            <label className="field">
              <span>Тип</span>
              <select value={request.auth.type} onChange={(event) => patch({ auth: authForType(event.currentTarget.value as AuthConfig["type"]) })}>
                <option value="none">Нет</option>
                <option value="bearer">Bearer Token</option>
                <option value="basic">Basic Auth</option>
                <option value="api_key_header">API Key в заголовке</option>
                <option value="api_key_query">API Key в query</option>
              </select>
            </label>
            {request.auth.type === "bearer" && (
              <label className="field"><span>Token</span><input value={request.auth.token} onChange={(event) => patch({ auth: { type: "bearer", token: event.currentTarget.value } })} placeholder="{{secret:API_TOKEN}}" /></label>
            )}
            {request.auth.type === "basic" && (
              <>
                <label className="field"><span>Имя пользователя</span><input value={request.auth.username} onChange={(event) => request.auth.type === "basic" && patch({ auth: { type: "basic", username: event.currentTarget.value, password: request.auth.password } })} /></label>
                <label className="field"><span>Пароль</span><input value={request.auth.password} onChange={(event) => request.auth.type === "basic" && patch({ auth: { type: "basic", username: request.auth.username, password: event.currentTarget.value } })} placeholder="{{secret:PASSWORD}}" /></label>
              </>
            )}
            {(request.auth.type === "api_key_header" || request.auth.type === "api_key_query") && (
              <>
                <label className="field"><span>Имя</span><input value={request.auth.name} onChange={(event) => (request.auth.type === "api_key_header" || request.auth.type === "api_key_query") && patch({ auth: { type: request.auth.type, name: event.currentTarget.value, value: request.auth.value } })} /></label>
                <label className="field"><span>Значение</span><input value={request.auth.value} onChange={(event) => (request.auth.type === "api_key_header" || request.auth.type === "api_key_query") && patch({ auth: { type: request.auth.type, name: request.auth.name, value: event.currentTarget.value } })} placeholder="{{secret:API_KEY}}" /></label>
              </>
            )}
            <p className="help-text">Для токенов и паролей используй ссылку вида <code>{"{{secret:NAME}}"}</code>.</p>
          </div>
        )}
        {tab === "body" && (
          <div className="body-editor">
            <label className="field body-type">
              <span>Тип тела</span>
              <select value={request.body.type} onChange={(event) => patch({ body: bodyForType(event.currentTarget.value as BodyConfig["type"]) })}>
                <option value="none">Нет</option>
                <option value="json">JSON</option>
                <option value="raw">Raw text</option>
                <option value="form_urlencoded">Form URL-encoded</option>
              </select>
            </label>
            {request.body.type === "json" && <textarea className="code-input" value={request.body.value} onChange={(event) => patch({ body: { type: "json", value: event.currentTarget.value } })} spellCheck={false} aria-label="JSON-тело" />}
            {request.body.type === "raw" && (
              <>
                <label className="field compact-content-type"><span>Content-Type</span><input value={request.body.content_type} onChange={(event) => request.body.type === "raw" && patch({ body: { type: "raw", value: request.body.value, content_type: event.currentTarget.value } })} /></label>
                <textarea className="code-input" value={request.body.value} onChange={(event) => request.body.type === "raw" && patch({ body: { type: "raw", value: event.currentTarget.value, content_type: request.body.content_type } })} spellCheck={false} aria-label="Текстовое тело" />
              </>
            )}
            {request.body.type === "form_urlencoded" && <KeyValueEditor rows={request.body.fields} onChange={(fields) => patch({ body: { type: "form_urlencoded", fields } })} emptyText="Полей формы пока нет" />}
          </div>
        )}
      </div>

      <div className="request-options">
        <label className="compact-field"><span>Таймаут, мс</span><input type="number" min="1" max="600000" value={request.timeout_ms} onChange={(event) => patch({ timeout_ms: Number(event.currentTarget.value) || 1 })} /></label>
        <label className="check-field"><input type="checkbox" checked={request.follow_redirects} onChange={(event) => patch({ follow_redirects: event.currentTarget.checked })} /> Следовать редиректам</label>
      </div>
      <SecurityLens report={securityReport} copyStatus={copyStatus} onCopy={onCopyCurl} />
    </section>
  );
}
