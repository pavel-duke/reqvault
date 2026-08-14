import { useMemo, useState } from "react";
import type { RequestSummary } from "../types";

type Props = {
  requests: RequestSummary[];
  busy: boolean;
  error: string | null;
  onMove: (paths: string[], collection: string) => void;
  onDuplicate: (paths: string[], collection: string) => void;
  onRename: (path: string, name: string) => void;
  onClose: () => void;
};

function requestCollection(path: string) {
  const parts = path.split("/");
  return parts.length > 2 ? parts[1] : "Общее";
}

export function RequestManagerDialog({ requests, busy, error, onMove, onDuplicate, onRename, onClose }: Props) {
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [collection, setCollection] = useState("Общее");
  const [renameValue, setRenameValue] = useState("");
  const normalizedSearch = search.trim().toLocaleLowerCase("ru");
  const filtered = useMemo(() => requests.filter((summary) =>
    !normalizedSearch || [summary.request.name, summary.request.method, summary.request.url, summary.relative_path]
      .some((value) => value.toLocaleLowerCase("ru").includes(normalizedSearch))), [normalizedSearch, requests]);
  const collections = useMemo(() => [...new Set(requests.map((summary) => requestCollection(summary.relative_path)))].sort(), [requests]);
  const selectedPaths = [...selected];
  const selectedRequest = selectedPaths.length === 1 ? requests.find((summary) => summary.relative_path === selectedPaths[0]) ?? null : null;

  function syncRenameValue(next: Set<string>) {
    const path = next.size === 1 ? [...next][0] : null;
    setRenameValue(path ? requests.find((summary) => summary.relative_path === path)?.request.name ?? "" : "");
  }

  function toggle(path: string) {
    const next = new Set(selected);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setSelected(next);
    syncRenameValue(next);
  }

  function selectFiltered() {
    const next = new Set([...selected, ...filtered.map((summary) => summary.relative_path)]);
    setSelected(next);
    syncRenameValue(next);
  }

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="modal-card request-manager-dialog" role="dialog" aria-modal="true" aria-labelledby="request-manager-title">
        <header className="modal-header">
          <div><span className="eyebrow">Workspace</span><h2 id="request-manager-title">Управление запросами</h2></div>
          <button className="quiet-icon" type="button" onClick={onClose} aria-label="Закрыть">×</button>
        </header>

        <div className="request-manager-toolbar">
          <label className="field"><span>Поиск</span><input value={search} onChange={(event) => setSearch(event.currentTarget.value)} placeholder="Название, метод, URL или путь" /></label>
          <div className="request-manager-selection"><strong>Выбрано: {selected.size}</strong><button className="text-button" type="button" onClick={selectFiltered} disabled={filtered.length === 0}>Выбрать найденные</button><button className="text-button" type="button" onClick={() => { setSelected(new Set()); setRenameValue(""); }} disabled={selected.size === 0}>Сбросить</button></div>
        </div>

        <div className="request-manager-list" aria-label="Запросы для массовой операции">
          {filtered.map((summary) => (
            <label className="request-manager-row" key={summary.relative_path}>
              <input type="checkbox" checked={selected.has(summary.relative_path)} onChange={() => toggle(summary.relative_path)} />
              <span className={`method method-${summary.request.method.toLowerCase()}`}>{summary.request.method}</span>
              <span><strong>{summary.request.name}</strong><small>{summary.relative_path}</small></span>
            </label>
          ))}
          {filtered.length === 0 && <div className="sidebar-empty"><p>Ничего не найдено</p></div>}
        </div>

        <div className="request-manager-actions">
          <div className="request-batch-panel">
            <label className="field"><span>Целевая коллекция</span><input list="request-collections" value={collection} onChange={(event) => setCollection(event.currentTarget.value)} /></label>
            <datalist id="request-collections">{collections.map((item) => <option value={item} key={item} />)}</datalist>
            <div className="inline-actions">
              <button className="secondary-button" type="button" disabled={busy || selected.size === 0 || !collection.trim()} onClick={() => onDuplicate(selectedPaths, collection)}>Дублировать</button>
              <button className="primary-button" type="button" disabled={busy || selected.size === 0 || !collection.trim()} onClick={() => onMove(selectedPaths, collection)}>Переместить</button>
            </div>
            <p className="help-text">Файлы не перезаписываются: при совпадении имени ReqVault добавит номер.</p>
          </div>
          <div className="request-batch-panel">
            <label className="field"><span>Переименовать выбранный запрос</span><input value={renameValue} onChange={(event) => setRenameValue(event.currentTarget.value)} disabled={!selectedRequest} placeholder="Выберите один запрос" /></label>
            <button className="secondary-button" type="button" disabled={busy || !selectedRequest || !renameValue.trim()} onClick={() => selectedRequest && onRename(selectedRequest.relative_path, renameValue)}>Переименовать</button>
            <p className="help-text">Меняется отображаемое название. Путь YAML остаётся стабильным для Git.</p>
          </div>
        </div>

        {error && <div className="error-banner request-manager-error" role="alert">{error}</div>}
        <footer className="modal-actions"><span /><button className="secondary-button" type="button" onClick={onClose}>Закрыть</button></footer>
      </section>
    </div>
  );
}
