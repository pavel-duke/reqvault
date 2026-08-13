import { useMemo, useState } from "react";
import { diffResponses, type ValueChange } from "../response-diff";
import type { HistoryEntry, HistorySummary, HttpResponse } from "../types";

type Props = {
  current: HttpResponse;
  entries: HistorySummary[];
  busy: boolean;
  error: string | null;
  onLoad: (id: string) => Promise<HistoryEntry>;
  onClose: () => void;
};

function formatDate(value: number) {
  return new Intl.DateTimeFormat("ru-RU", { dateStyle: "short", timeStyle: "short" }).format(value);
}

function Changes({ title, changes }: { title: string; changes: ValueChange[] }) {
  return (
    <section className="diff-section">
      <h3>{title} <span>{changes.length}</span></h3>
      {changes.length === 0 ? <p className="diff-empty">Изменений нет</p> : <div className="diff-list">{changes.map((change, index) => (
        <div className={`diff-item diff-${change.kind}`} key={`${change.path}-${index}`}>
          <code>{change.path}</code>
          <span className="diff-kind">{change.kind === "added" ? "Добавлено" : change.kind === "removed" ? "Удалено" : "Изменено"}</span>
          <div><small>Было</small><pre>{change.before ?? "—"}</pre></div>
          <div><small>Стало</small><pre>{change.after ?? "—"}</pre></div>
        </div>
      ))}</div>}
    </section>
  );
}

export function ResponseCompareDialog({ current, entries, busy, error, onLoad, onClose }: Props) {
  const [selected, setSelected] = useState<HistoryEntry | null>(null);
  const [localBusy, setLocalBusy] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const diff = useMemo(() => selected ? diffResponses({
    status: selected.summary.status,
    status_text: selected.status_text,
    headers: selected.headers,
    body: selected.body,
    is_json: selected.is_json,
  }, current) : null, [current, selected]);

  async function select(id: string) {
    setLocalBusy(true);
    setLocalError(null);
    try {
      setSelected(await onLoad(id));
    } catch (caught) {
      setLocalError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLocalBusy(false);
    }
  }

  const noChanges = diff && !diff.statusChanged && diff.headerChanges.length === 0 && !diff.bodyChanged;
  return (
    <div className="modal-backdrop" role="presentation">
      <section className="modal-card compare-modal" role="dialog" aria-modal="true" aria-labelledby="compare-title">
        <header className="modal-header"><div><span className="eyebrow">Структурный анализ</span><h2 id="compare-title">Сравнение ответов</h2></div><button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button></header>
        <div className="compare-layout">
          <aside className="compare-history">
            <strong>Предыдущий ответ</strong>
            <p>Выберите сохранённую запись. Текущий ответ используется как новая версия.</p>
            {entries.map((entry) => <button className={selected?.summary.id === entry.id ? "selected" : ""} type="button" key={entry.id} onClick={() => void select(entry.id)}><span><b className={`method method-${entry.method.toLowerCase()}`}>{entry.method}</b>{entry.request_name}</span><small>{entry.status} · {formatDate(entry.created_at_ms)}</small></button>)}
            {!busy && entries.length === 0 && <div className="compare-empty"><p>В истории пока нет ответов.</p><small>Включите историю и отправьте запрос повторно.</small></div>}
          </aside>
          <div className="compare-result">
            {(error || localError) && <div className="error-banner" role="alert">{error || localError}</div>}
            {(busy || localBusy) && <div className="response-state"><span className="spinner" aria-hidden="true" /><p>Готовлю сравнение…</p></div>}
            {!busy && !localBusy && !diff && <div className="response-state"><span className="response-placeholder" aria-hidden="true">⇄</span><p>Выберите ответ слева.</p></div>}
            {diff && <>
              <div className={`status-diff ${diff.statusChanged ? "changed" : "same"}`}><span>{diff.beforeStatus}</span><b>→</b><span>{diff.afterStatus}</span></div>
              {noChanges && <div className="compare-identical"><strong>Ответы совпадают</strong><p>Status, заголовки и тело не изменились.</p></div>}
              <Changes title="Заголовки" changes={diff.headerChanges} />
              <Changes title="JSON / тело" changes={diff.bodyChanges} />
              {diff.truncated && <p className="response-warning">Показаны первые 250 изменений.</p>}
            </>}
          </div>
        </div>
        <footer className="modal-actions"><button className="secondary-button" type="button" onClick={onClose}>Закрыть</button></footer>
      </section>
    </div>
  );
}
