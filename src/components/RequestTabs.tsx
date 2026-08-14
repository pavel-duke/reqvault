import type { RequestTabState } from "../tabs-storage";

type Props = {
  tabs: RequestTabState[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onNew: () => void;
};

export function RequestTabs({ tabs, activeId, onSelect, onClose, onNew }: Props) {
  if (tabs.length === 0) return null;
  return (
    <div className="request-tabs-shell">
      <div className="request-tabs" role="tablist" aria-label="Открытые запросы">
        {tabs.map((tab) => (
          <div className={`request-tab ${tab.id === activeId ? "active" : ""}`} key={tab.id}>
            <button
              type="button"
              role="tab"
              aria-selected={tab.id === activeId}
              onClick={() => onSelect(tab.id)}
              title={tab.relativePath ?? "Несохранённый запрос"}
            >
              <span className={`method method-${tab.request.method.toLowerCase()}`}>{tab.request.method}</span>
              <span>{tab.request.name || "Без названия"}</span>
              {tab.dirty && <i aria-label="Есть несохранённые изменения" />}
            </button>
            <button className="request-tab-close" type="button" onClick={() => onClose(tab.id)} aria-label={`Закрыть ${tab.request.name || "вкладку"}`}>×</button>
          </div>
        ))}
      </div>
      <button className="request-tab-new" type="button" onClick={onNew} title="Новый запрос" aria-label="Новая вкладка">+</button>
    </div>
  );
}
