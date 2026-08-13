import { useMemo, useState } from "react";
import type { HistoryEntry, HistorySettings, HistorySummary } from "../types";

type Props = {
  settings: HistorySettings;
  entries: HistorySummary[];
  busy: boolean;
  error: string | null;
  onSettingsChange: (settings: HistorySettings) => Promise<void>;
  onLoad: (id: string) => Promise<HistoryEntry>;
  onDelete: (id: string) => Promise<void>;
  onClear: () => Promise<void>;
  onClose: () => void;
};

function formatDate(value: number) {
  return new Intl.DateTimeFormat("ru-RU", { dateStyle: "short", timeStyle: "medium" }).format(value);
}

export function HistoryDialog({ settings, entries, busy, error, onSettingsChange, onLoad, onDelete, onClear, onClose }: Props) {
  const [selected, setSelected] = useState<HistoryEntry | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const visibleEntries = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("ru");
    if (!normalized) return entries;
    return entries.filter((entry) =>
      [entry.request_name, entry.method, entry.url, String(entry.status)]
        .some((value) => value.toLocaleLowerCase("ru").includes(normalized)),
    );
  }, [entries, query]);

  async function load(id: string) {
    setLocalError(null);
    try {
      setSelected(await onLoad(id));
    } catch (caught) {
      setLocalError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="modal-card history-modal" role="dialog" aria-modal="true" aria-labelledby="history-title">
        <header className="modal-header"><div><span className="eyebrow">Локальные данные</span><h2 id="history-title">История ответов</h2></div><button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button></header>
        <div className="history-settings">
          <label className="check-field"><input type="checkbox" checked={settings.enabled} disabled={busy} onChange={(event) => void onSettingsChange({ ...settings, enabled: event.currentTarget.checked })} /> Сохранять ответы на этом компьютере</label>
          <label className="compact-field"><span>Лимит</span><input type="number" min="1" max="500" value={settings.max_entries} disabled={busy} onChange={(event) => void onSettingsChange({ ...settings, max_entries: Number(event.currentTarget.value) || 1 })} /></label>
        </div>
        <p className="history-warning">История выключена по умолчанию. При включении очищенные ответы сохраняются вне workspace и не попадают в Git, но могут содержать рабочие данные API.</p>
        <label className="field history-search"><span>Поиск в истории</span><input value={query} onChange={(event) => setQuery(event.currentTarget.value)} placeholder="Запрос, URL, метод или status" /></label>
        {(error || localError) && <div className="error-banner" role="alert">{error || localError}</div>}
        <div className="history-content">
          <div className="history-list">
            {entries.length === 0 && <p className="help-text">Сохранённых ответов пока нет.</p>}
            {visibleEntries.map((entry) => <button type="button" key={entry.id} className={`history-item ${selected?.summary.id === entry.id ? "selected" : ""}`} onClick={() => void load(entry.id)}><strong><span className={`method method-${entry.method.toLowerCase()}`}>{entry.method}</span> {entry.request_name}</strong><small>{entry.status} · {entry.duration_ms} мс · {formatDate(entry.created_at_ms)}</small><span>{entry.url}</span></button>)}
            {entries.length > 0 && visibleEntries.length === 0 && <p className="help-text">По этому запросу ничего не найдено.</p>}
          </div>
          <div className="history-preview">
            {selected ? <><div className="response-metrics"><strong>{selected.summary.status} {selected.status_text}</strong><span>{selected.summary.size_bytes} Б</span></div><pre className="response-body">{selected.body || "Ответ не содержит тела"}</pre><button className="danger-button" type="button" onClick={() => void onDelete(selected.summary.id).then(() => setSelected(null))}>Удалить запись</button></> : <p className="help-text">Выберите запись слева.</p>}
          </div>
        </div>
        <footer className="modal-actions"><button className="danger-button" type="button" disabled={busy || entries.length === 0} onClick={() => void onClear().then(() => setSelected(null))}>Очистить историю</button><button className="secondary-button" type="button" onClick={onClose}>Закрыть</button></footer>
      </section>
    </div>
  );
}
