import { useMemo, useRef, useState } from "react";
import type { Channel } from "@tauri-apps/api/core";
import { connectStream, disconnectStream, errorMessage, sendStreamMessage } from "../api";
import { recordToRows, rowsToRecord } from "../request-utils";
import type { KeyValue, StreamEvent, WorkspaceSnapshot } from "../types";
import { KeyValueEditor } from "./KeyValueEditor";

type Props = {
  workspace: WorkspaceSnapshot;
  activeEnvironment: string;
  onClose: () => void;
};

export function StreamDialog({ workspace, activeEnvironment, onClose }: Props) {
  const [protocol, setProtocol] = useState<"websocket" | "sse">("websocket");
  const [url, setUrl] = useState("");
  const [headers, setHeaders] = useState<KeyValue[]>(() => recordToRows({ Accept: "application/json" }));
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [events, setEvents] = useState<StreamEvent[]>([]);
  const [message, setMessage] = useState("");
  const [filter, setFilter] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const channel = useRef<Channel<StreamEvent> | null>(null);

  const visibleEvents = useMemo(() => {
    const query = filter.trim().toLowerCase();
    return query ? events.filter((event) => `${event.kind} ${event.data}`.toLowerCase().includes(query)) : events;
  }, [events, filter]);

  async function connect() {
    setBusy(true);
    setError(null);
    setEvents([]);
    try {
      const environment = workspace.environments.find((item) => item.relative_path === activeEnvironment)?.environment ?? null;
      const result = await connectStream({ protocol, url, headers: rowsToRecord(headers), workspace_id: workspace.config.id, workspace_path: workspace.root_path, environment }, (event) => {
        setEvents((current) => [...current.slice(-999), event]);
        if (event.kind === "closed") setSessionId(null);
      });
      channel.current = result.channel;
      setSessionId(result.sessionId);
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    if (!sessionId) return;
    try {
      await disconnectStream(sessionId);
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  async function send() {
    if (!sessionId || !message) return;
    try {
      await sendStreamMessage(sessionId, message);
      setMessage("");
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  function close() {
    if (sessionId) void disconnectStream(sessionId).catch(() => undefined);
    channel.current = null;
    onClose();
  }

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="modal-card stream-dialog" role="dialog" aria-modal="true" aria-labelledby="stream-dialog-title">
        <div className="modal-header"><div><span className="eyebrow">Streaming</span><h2 id="stream-dialog-title">WebSocket и SSE</h2></div><button className="quiet-icon" type="button" onClick={close} aria-label="Закрыть">×</button></div>
        <div className="stream-connect">
          <label className="field"><span>Протокол</span><select value={protocol} disabled={Boolean(sessionId)} onChange={(event) => setProtocol(event.currentTarget.value as "websocket" | "sse")}><option value="websocket">WebSocket</option><option value="sse">Server-Sent Events</option></select></label>
          <label className="field stream-url"><span>URL</span><input value={url} disabled={Boolean(sessionId)} onChange={(event) => setUrl(event.currentTarget.value)} placeholder={protocol === "websocket" ? "wss://api.example.test/socket" : "https://api.example.test/events"} /></label>
          {sessionId ? <button className="danger-button" type="button" onClick={() => void disconnect()}>Отключить</button> : <button className="primary-button" type="button" disabled={busy || !url.trim()} onClick={() => void connect()}>{busy ? "Подключаю…" : "Подключить"}</button>}
        </div>
        {!sessionId && <div className="stream-headers"><strong>Заголовки</strong><KeyValueEditor rows={headers} onChange={setHeaders} emptyText="Заголовков нет" /></div>}
        {error && <div className="error-banner stream-error" role="alert">{error}</div>}
        <div className="stream-toolbar"><input value={filter} onChange={(event) => setFilter(event.currentTarget.value)} placeholder="Фильтр журнала" /><span>{visibleEvents.length} / {events.length}</span><button className="text-button" type="button" onClick={() => setEvents([])}>Очистить</button></div>
        <div className="stream-log">
          {visibleEvents.length === 0 && <div className="empty-inline">События соединения появятся здесь.</div>}
          {visibleEvents.map((event, index) => <div className={`stream-event stream-${event.kind}`} key={`${event.timestamp_ms}-${index}`}><time>{new Date(event.timestamp_ms).toLocaleTimeString()}</time><span>{event.kind}</span><pre>{event.data}</pre></div>)}
        </div>
        {protocol === "websocket" && <div className="stream-send"><textarea value={message} onChange={(event) => setMessage(event.currentTarget.value)} disabled={!sessionId} placeholder="Текстовое WebSocket-сообщение" /><button className="primary-button" type="button" disabled={!sessionId || !message} onClick={() => void send()}>Отправить</button></div>}
      </section>
    </div>
  );
}
