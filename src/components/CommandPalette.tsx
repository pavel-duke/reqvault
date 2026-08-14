import { useMemo, useState } from "react";
import type { RequestSummary } from "../types";
import { Icon, type IconName } from "./Icon";

export type PaletteAction = {
  id: string;
  label: string;
  description: string;
  keywords?: string;
  shortcut?: string;
  icon: IconName;
  onSelect: () => void;
};

type Props = {
  open: boolean;
  actions: PaletteAction[];
  requests: RequestSummary[];
  recentPaths: string[];
  onOpenRequest: (request: RequestSummary) => void;
  onClose: () => void;
};

type Result =
  | { id: string; kind: "action"; label: string; description: string; shortcut?: string; icon: IconName; run: () => void }
  | { id: string; kind: "request"; label: string; description: string; method: string; run: () => void };

function requestText(summary: RequestSummary) {
  return [
    summary.request.name,
    summary.request.url,
    summary.request.method,
    summary.relative_path,
    ...Object.keys(summary.request.headers),
  ].join(" ").toLocaleLowerCase("ru");
}

export function CommandPalette({ open, actions, requests, recentPaths, onOpenRequest, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  const results = useMemo<Result[]>(() => {
    const normalized = query.trim().toLocaleLowerCase("ru");
    const actionResults = actions
      .filter((action) => !normalized || `${action.label} ${action.description} ${action.keywords ?? ""}`.toLocaleLowerCase("ru").includes(normalized))
      .map<Result>((action) => ({
        id: `action:${action.id}`,
        kind: "action",
        label: action.label,
        description: action.description,
        shortcut: action.shortcut,
        icon: action.icon,
        run: action.onSelect,
      }));
    const recentRank = new Map(recentPaths.map((path, index) => [path, index]));
    const requestResults = requests
      .filter((summary) => normalized ? requestText(summary).includes(normalized) : recentRank.has(summary.relative_path))
      .sort((left, right) => (recentRank.get(left.relative_path) ?? 1000) - (recentRank.get(right.relative_path) ?? 1000))
      .slice(0, normalized ? 10 : 6)
      .map<Result>((summary) => ({
        id: `request:${summary.relative_path}`,
        kind: "request",
        label: summary.request.name,
        description: summary.request.url || summary.relative_path,
        method: summary.request.method,
        run: () => onOpenRequest(summary),
      }));
    return [...actionResults.slice(0, normalized ? 8 : 7), ...requestResults];
  }, [actions, onOpenRequest, query, recentPaths, requests]);

  if (!open) return null;

  const choose = (result: Result | undefined) => {
    if (!result) return;
    onClose();
    result.run();
  };

  return (
    <div className="modal-backdrop command-backdrop" role="presentation" onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section className="command-palette" role="dialog" aria-modal="true" aria-labelledby="command-palette-title">
        <div className="command-search-line">
          <Icon name="search" />
          <label className="sr-only" htmlFor="command-palette-input" id="command-palette-title">Команды и запросы</label>
          <input
            id="command-palette-input"
            autoFocus
            role="combobox"
            aria-controls="command-palette-results"
            aria-expanded="true"
            aria-activedescendant={results[activeIndex]?.id}
            value={query}
            onChange={(event) => { setQuery(event.currentTarget.value); setActiveIndex(0); }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setActiveIndex((index) => Math.min(index + 1, Math.max(0, results.length - 1)));
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setActiveIndex((index) => Math.max(0, index - 1));
              } else if (event.key === "Enter") {
                event.preventDefault();
                choose(results[activeIndex]);
              } else if (event.key === "Escape") {
                event.preventDefault();
                onClose();
              }
            }}
            placeholder="Команда, запрос, URL, метод или заголовок"
          />
          <kbd>Esc</kbd>
          <button className="sr-only" type="button" aria-label="Закрыть" onClick={onClose}>Закрыть</button>
        </div>
        <div className="command-results" id="command-palette-results" role="listbox">
          {results.length === 0 && <div className="command-empty"><strong>Ничего не найдено</strong><span>Проверь название, URL или имя заголовка.</span></div>}
          {results.map((result, index) => (
            <button
              id={result.id}
              key={result.id}
              className={`command-result ${index === activeIndex ? "active" : ""}`}
              type="button"
              role="option"
              aria-selected={index === activeIndex}
              onMouseEnter={() => setActiveIndex(index)}
              onClick={() => choose(result)}
            >
              <span className="command-result-icon">
                {result.kind === "action" ? <Icon name={result.icon} /> : <span className={`method method-${result.method.toLowerCase()}`}>{result.method}</span>}
              </span>
              <span><strong>{result.label}</strong><small>{result.description}</small></span>
              {result.kind === "action" && result.shortcut && <kbd>{result.shortcut}</kbd>}
              {result.kind === "request" && <span className="command-kind">Запрос</span>}
            </button>
          ))}
        </div>
        <footer className="command-footer"><span><kbd>↑↓</kbd> выбор</span><span><kbd>Enter</kbd> открыть</span><span>Значения Secret Vault не индексируются</span></footer>
      </section>
    </div>
  );
}
