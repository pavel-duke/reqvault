import { useMemo, useState, type KeyboardEvent } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { recordToRows, rowsToRecord } from "../request-utils";
import type { AuthConfig, BodyConfig, KeyValue, MultipartField, ProxyConfig, RequestFile, SecurityReport } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";
import { SecurityLens } from "./SecurityLens";
import { AssertionsEditor } from "./AssertionsEditor";

type EditorTab = "query" | "headers" | "auth" | "body" | "tests" | "transport";

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
  onAuthorizeOAuth: () => void;
  onRefreshOAuth: () => void;
  oauthBusy: boolean;
  oauthStatus: string | null;
};

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

function authForType(type: AuthConfig["type"]): AuthConfig {
  switch (type) {
    case "bearer": return { type, token: "{{secret:API_TOKEN}}" };
    case "basic": return { type, username: "", password: "{{secret:PASSWORD}}" };
    case "api_key_header": return { type, name: "X-API-Key", value: "{{secret:API_KEY}}" };
    case "api_key_query": return { type, name: "api_key", value: "{{secret:API_KEY}}" };
    case "oauth2": return {
      type,
      grant_type: "authorization_code_pkce",
      authorization_url: "",
      token_url: "",
      client_id: "",
      client_secret: "{{secret:OAUTH_CLIENT_SECRET}}",
      scopes: "",
      access_token: "{{secret:OAUTH_ACCESS_TOKEN}}",
      refresh_token: "{{secret:OAUTH_REFRESH_TOKEN}}",
    };
    default: return { type: "none" };
  }
}

function bodyForType(type: BodyConfig["type"]): BodyConfig {
  switch (type) {
    case "json": return { type, value: "{\n  \"key\": \"value\"\n}" };
    case "raw": return { type, value: "", content_type: "text/plain" };
    case "form_urlencoded": return { type, fields: [] };
    case "multipart": return { type, fields: [] };
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
  onAuthorizeOAuth,
  onRefreshOAuth,
  oauthBusy,
  oauthStatus,
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

  async function pickPath(kind: "file" | "certificate" | "key"): Promise<string | null> {
    const filters = kind === "certificate"
      ? [{ name: "Сертификаты PEM", extensions: ["pem", "crt", "cer"] }]
      : kind === "key"
        ? [{ name: "Приватные ключи PEM", extensions: ["pem", "key"] }]
        : undefined;
    const selected = await open({ multiple: false, directory: false, filters });
    return typeof selected === "string" ? selected : null;
  }

  function updateMultipart(index: number, field: MultipartField) {
    if (request.body.type !== "multipart") return;
    const fields = [...request.body.fields];
    fields[index] = field;
    patch({ body: { type: "multipart", fields } });
  }

  function updateProxy(proxy: ProxyConfig) {
    patch({ transport: { ...request.transport, proxy } });
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
        <button className={tab === "tests" ? "active" : ""} type="button" onClick={() => setTab("tests")}>Проверки <span>{request.tests.length || ""}</span></button>
        <button className={tab === "transport" ? "active" : ""} type="button" onClick={() => setTab("transport")}>Сеть</button>
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
                <option value="oauth2">OAuth 2.0</option>
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
            {request.auth.type === "oauth2" && (
              <div className="oauth-editor">
                <label className="field"><span>Grant type</span><select value={request.auth.grant_type} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, grant_type: event.currentTarget.value as "authorization_code_pkce" | "client_credentials" } })}><option value="authorization_code_pkce">Authorization Code + PKCE</option><option value="client_credentials">Client Credentials</option></select></label>
                {request.auth.grant_type === "authorization_code_pkce" && <label className="field"><span>Authorization URL</span><input value={request.auth.authorization_url} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, authorization_url: event.currentTarget.value } })} placeholder="https://id.example.test/oauth/authorize" /></label>}
                <label className="field"><span>Token URL</span><input value={request.auth.token_url} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, token_url: event.currentTarget.value } })} placeholder="https://id.example.test/oauth/token" /></label>
                <label className="field"><span>Client ID</span><input value={request.auth.client_id} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, client_id: event.currentTarget.value } })} /></label>
                <label className="field"><span>Client secret</span><input value={request.auth.client_secret} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, client_secret: event.currentTarget.value } })} placeholder="{{secret:OAUTH_CLIENT_SECRET}}" /></label>
                <label className="field"><span>Scopes через пробел</span><input value={request.auth.scopes} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, scopes: event.currentTarget.value } })} /></label>
                <label className="field"><span>Access token secret</span><input value={request.auth.access_token} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, access_token: event.currentTarget.value } })} /></label>
                <label className="field"><span>Refresh token secret</span><input value={request.auth.refresh_token} onChange={(event) => request.auth.type === "oauth2" && patch({ auth: { ...request.auth, refresh_token: event.currentTarget.value } })} /></label>
                <div className="inline-actions">
                  <button className="secondary-button" type="button" onClick={onAuthorizeOAuth} disabled={oauthBusy}>{oauthBusy ? "Выполняю OAuth…" : "Получить токен"}</button>
                  <button className="secondary-button" type="button" onClick={onRefreshOAuth} disabled={oauthBusy || !request.auth.refresh_token.trim()}>Обновить token</button>
                </div>
                {oauthStatus && <p className="help-text">{oauthStatus}</p>}
              </div>
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
                <option value="multipart">Multipart form-data</option>
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
            {request.body.type === "multipart" && (
              <div className="multipart-editor">
                {request.body.fields.map((field, index) => (
                  <div className="multipart-row" key={index}>
                    <input type="checkbox" checked={field.enabled} onChange={(event) => updateMultipart(index, { ...field, enabled: event.currentTarget.checked })} aria-label="Включить поле" />
                    <select value={field.type} onChange={(event) => updateMultipart(index, event.currentTarget.value === "file" ? { type: "file", name: field.name, path: "", content_type: "", enabled: field.enabled } : { type: "text", name: field.name, value: "", enabled: field.enabled })}><option value="text">Текст</option><option value="file">Файл</option></select>
                    <input value={field.name} onChange={(event) => updateMultipart(index, { ...field, name: event.currentTarget.value })} placeholder="Имя поля" />
                    {field.type === "text" ? <input value={field.value} onChange={(event) => updateMultipart(index, { ...field, value: event.currentTarget.value })} placeholder="Значение" /> : <><input value={field.path} onChange={(event) => updateMultipart(index, { ...field, path: event.currentTarget.value })} placeholder="Путь к файлу" /><button className="secondary-button" type="button" onClick={() => void pickPath("file").then((path) => path && field.type === "file" && updateMultipart(index, { ...field, path }))}>Выбрать</button></>}
                    <button className="quiet-icon" type="button" onClick={() => request.body.type === "multipart" && patch({ body: { type: "multipart", fields: request.body.fields.filter((_, itemIndex) => itemIndex !== index) } })}>×</button>
                  </div>
                ))}
                <div className="inline-actions"><button className="secondary-button" type="button" onClick={() => request.body.type === "multipart" && patch({ body: { type: "multipart", fields: [...request.body.fields, { type: "text", name: "", value: "", enabled: true }] } })}>Добавить текст</button><button className="secondary-button" type="button" onClick={() => request.body.type === "multipart" && patch({ body: { type: "multipart", fields: [...request.body.fields, { type: "file", name: "file", path: "", content_type: "", enabled: true }] } })}>Добавить файл</button></div>
              </div>
            )}
          </div>
        )}
        {tab === "tests" && <AssertionsEditor assertions={request.tests} onChange={(tests) => patch({ tests })} />}
        {tab === "transport" && (
          <div className="transport-editor">
            <label className="field"><span>Proxy</span><select value={request.transport.proxy.type} onChange={(event) => updateProxy(event.currentTarget.value === "system" ? { type: "system" } : event.currentTarget.value === "custom" ? { type: "custom", url: "", username: "", password: "{{secret:PROXY_PASSWORD}}" } : { type: "none" })}><option value="none">Не использовать</option><option value="system">Системный proxy</option><option value="custom">Указать вручную</option></select></label>
            {request.transport.proxy.type === "custom" && <><label className="field"><span>Proxy URL</span><input value={request.transport.proxy.url} onChange={(event) => request.transport.proxy.type === "custom" && updateProxy({ ...request.transport.proxy, url: event.currentTarget.value })} placeholder="http://proxy.example.test:8080" /></label><label className="field"><span>Имя пользователя</span><input value={request.transport.proxy.username} onChange={(event) => request.transport.proxy.type === "custom" && updateProxy({ ...request.transport.proxy, username: event.currentTarget.value })} /></label><label className="field"><span>Пароль</span><input value={request.transport.proxy.password} onChange={(event) => request.transport.proxy.type === "custom" && updateProxy({ ...request.transport.proxy, password: event.currentTarget.value })} /></label></>}
            <label className="field path-field"><span>Custom CA (PEM)</span><div><input value={request.transport.custom_ca_path} onChange={(event) => patch({ transport: { ...request.transport, custom_ca_path: event.currentTarget.value } })} /><button className="secondary-button" type="button" onClick={() => void pickPath("certificate").then((path) => path && patch({ transport: { ...request.transport, custom_ca_path: path } }))}>Выбрать</button></div></label>
            <label className="field path-field"><span>Клиентский сертификат (PEM)</span><div><input value={request.transport.client_certificate_path} onChange={(event) => patch({ transport: { ...request.transport, client_certificate_path: event.currentTarget.value } })} /><button className="secondary-button" type="button" onClick={() => void pickPath("certificate").then((path) => path && patch({ transport: { ...request.transport, client_certificate_path: path } }))}>Выбрать</button></div></label>
            <label className="field path-field"><span>Приватный ключ (PEM)</span><div><input value={request.transport.client_key_path} onChange={(event) => patch({ transport: { ...request.transport, client_key_path: event.currentTarget.value } })} /><button className="secondary-button" type="button" onClick={() => void pickPath("key").then((path) => path && patch({ transport: { ...request.transport, client_key_path: path } }))}>Выбрать</button></div></label>
            <p className="help-text">ReqVault не отключает проверку TLS. Для mTLS нужны отдельные PEM-файлы сертификата и незашифрованного приватного ключа.</p>
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
