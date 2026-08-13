import { Icon } from "./Icon";

type Props = {
  guardEnabled: boolean;
  onImport: () => void;
  onImportCurl: () => void;
  onExport: () => void;
  onSettings: () => void;
  onHistory: () => void;
  onCookies: () => void;
  onDiagnostics: () => void;
  onRun: () => void;
  onStream: () => void;
};

function RailButton({ label, icon, onClick, active, shortcut }: { label: string; icon: Parameters<typeof Icon>[0]["name"]; onClick: () => void; active?: boolean; shortcut?: string }) {
  return <button className={`rail-button ${active ? "active" : ""}`} type="button" onClick={onClick} data-label={label} title={shortcut ? `${label} · ${shortcut}` : label}><Icon name={icon} /><span>{label}</span></button>;
}

export function WorkspaceRail(props: Props) {
  return (
    <nav className="workspace-rail" aria-label="Инструменты workspace">
      <div className="rail-group">
        <RailButton label="Запросы" icon="archive" active onClick={() => document.querySelector<HTMLElement>("#workspace-search")?.focus()} shortcut="Ctrl+K" />
        <RailButton label="Запуск коллекции" icon="activity" onClick={props.onRun} />
        <RailButton label="Потоки" icon="pulse" onClick={props.onStream} />
        <RailButton label="История" icon="clock" onClick={props.onHistory} />
        <RailButton label="Диагностика" icon="shield" onClick={props.onDiagnostics} />
      </div>
      <div className="rail-separator" />
      <div className="rail-group">
        <RailButton label="Cookie jar" icon="cookie" onClick={props.onCookies} />
        <RailButton label={props.guardEnabled ? "Production Guard включён" : "Production Guard"} icon="settings" onClick={props.onSettings} />
      </div>
      <div className="rail-spacer" />
      <div className="rail-group">
        <RailButton label="Импорт файла" icon="file" onClick={props.onImport} />
        <RailButton label="Импорт cURL" icon="terminal" onClick={props.onImportCurl} />
        <RailButton label="Экспорт workspace" icon="download" onClick={props.onExport} />
      </div>
    </nav>
  );
}
