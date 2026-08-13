import { useMemo, useState } from "react";
import type { CookieSummary } from "../types";

type Props = {
  cookies: CookieSummary[];
  busy: boolean;
  error: string | null;
  onDelete: (id: string) => Promise<void>;
  onClear: () => Promise<void>;
  onClose: () => void;
};

function expiryLabel(value: number | null) {
  if (value === null) return "До закрытия workspace";
  return new Intl.DateTimeFormat("ru-RU", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(value * 1000);
}

export function CookieDialog({ cookies, busy, error, onDelete, onClear, onClose }: Props) {
  const [query, setQuery] = useState("");
  const visible = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("ru");
    if (!normalized) return cookies;
    return cookies.filter((cookie) =>
      [cookie.name, cookie.domain, cookie.path].some((value) =>
        value.toLocaleLowerCase("ru").includes(normalized),
      ),
    );
  }, [cookies, query]);

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="modal-card cookie-modal" role="dialog" aria-modal="true" aria-labelledby="cookie-title">
        <header className="modal-header">
          <div><span className="eyebrow">Сессия workspace</span><h2 id="cookie-title">Cookie jar</h2></div>
          <button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button>
        </header>
        <p className="modal-intro">Cookie принимаются из Set-Cookie и автоматически отправляются подходящему домену и пути. Значения скрыты и хранятся только в памяти.</p>
        <label className="field cookie-search"><span>Поиск</span><input value={query} onChange={(event) => setQuery(event.currentTarget.value)} placeholder="Имя, домен или путь" autoFocus /></label>
        {error && <div className="error-banner" role="alert">{error}</div>}
        <div className="cookie-table" role="table" aria-label="Cookie текущего workspace">
          <div className="cookie-row cookie-head" role="row"><span>Имя</span><span>Домен и путь</span><span>Свойства</span><span>Срок</span><span /></div>
          {visible.map((cookie) => (
            <div className="cookie-row" role="row" key={cookie.id}>
              <strong>{cookie.name}</strong>
              <span><code>{cookie.domain}</code><small>{cookie.path}</small></span>
              <span className="cookie-flags">{cookie.secure && <b>Secure</b>}{cookie.http_only && <b>HttpOnly</b>}{!cookie.secure && !cookie.http_only && <small>Обычная</small>}</span>
              <small>{expiryLabel(cookie.expires_at)}</small>
              <button className="text-button danger-text" type="button" disabled={busy} onClick={() => void onDelete(cookie.id)}>Удалить</button>
            </div>
          ))}
          {!busy && visible.length === 0 && <p className="cookie-empty">{cookies.length ? "Ничего не найдено." : "Серверы ещё не установили cookie для этого workspace."}</p>}
          {busy && <p className="cookie-empty">Загружаю cookie…</p>}
        </div>
        <footer className="modal-actions">
          <button className="danger-button" type="button" disabled={busy || cookies.length === 0} onClick={() => void onClear()}>Очистить cookie</button>
          <button className="secondary-button" type="button" onClick={onClose}>Закрыть</button>
        </footer>
      </section>
    </div>
  );
}
