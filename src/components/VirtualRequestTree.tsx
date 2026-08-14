import { useEffect, useRef, useState } from "react";
import type { RequestSummary } from "../types";
import { Icon, type IconName } from "./Icon";

export type VirtualRequestRow =
  | { kind: "group"; key: string; label: string; icon?: IconName }
  | { kind: "request"; key: string; summary: RequestSummary; favorite: boolean };

type Props = {
  rows: VirtualRequestRow[];
  selectedPath: string | null;
  onSelectRequest: (summary: RequestSummary) => void;
  onToggleFavorite: (path: string) => void;
};

const ROW_HEIGHT = 40;
const OVERSCAN = 6;

export function VirtualRequestTree({ rows, selectedPath, onSelectRequest, onToggleFavorite }: Props) {
  const containerRef = useRef<HTMLElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(480);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const measure = () => setViewportHeight(element.clientHeight || 480);
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const end = Math.min(rows.length, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN);

  return (
    <nav
      className="request-tree virtual-request-tree"
      aria-label="Запросы workspace"
      ref={containerRef}
      onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
    >
      <div className="virtual-request-space" style={{ height: rows.length * ROW_HEIGHT }}>
        {rows.slice(start, end).map((row, offset) => (
          <div className="virtual-request-row" style={{ transform: `translateY(${(start + offset) * ROW_HEIGHT}px)` }} key={row.key}>
            {row.kind === "group" ? (
              <h2 className="virtual-group-heading">{row.icon && <Icon name={row.icon} />} {row.label}</h2>
            ) : (
              <div className="request-item-row">
                <button
                  type="button"
                  className={`request-item ${selectedPath === row.summary.relative_path ? "selected" : ""}`}
                  onClick={() => onSelectRequest(row.summary)}
                >
                  <span className={`method method-${row.summary.request.method.toLowerCase()}`}>{row.summary.request.method}</span>
                  <span>{row.summary.request.name}</span>
                </button>
                <button
                  className={`favorite-toggle ${row.favorite ? "active" : ""}`}
                  type="button"
                  onClick={() => onToggleFavorite(row.summary.relative_path)}
                  aria-label={row.favorite ? `Открепить ${row.summary.request.name}` : `Закрепить ${row.summary.request.name}`}
                  title={row.favorite ? "Убрать из избранного" : "Добавить в избранное"}
                ><Icon name="star" /></button>
              </div>
            )}
          </div>
        ))}
      </div>
    </nav>
  );
}
