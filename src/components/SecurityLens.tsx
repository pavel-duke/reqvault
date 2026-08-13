import type { SecurityReport } from "../types";

type Props = {
  report: SecurityReport | null;
  copyStatus: string | null;
  onCopy: () => void;
};

export function SecurityLens({ report, copyStatus, onCopy }: Props) {
  return (
    <aside className="security-lens">
      <div className="security-title">
        <div><span className="eyebrow">Перед отправкой</span><h2>Проверка запроса</h2></div>
        <button className="secondary-button" type="button" onClick={onCopy}>Копировать как cURL</button>
      </div>
      {copyStatus && <div className="copy-status" role="status">{copyStatus}</div>}
      {report ? (
        <>
          <dl className="security-stats">
            <div><dt>HTTPS</dt><dd className={report.https ? "safe-value" : ""}>{report.https ? "да" : "нет"}</dd></div>
            <div><dt>Host</dt><dd title={report.host}>{report.host}</dd></div>
            <div><dt>Секретов</dt><dd>{report.secrets}</dd></div>
            <div><dt>В headers</dt><dd>{report.in_headers}</dd></div>
            <div><dt>В query</dt><dd>{report.in_query}</dd></div>
          </dl>
          {report.warnings.length > 0 && (
            <div className="security-warnings">
              {report.warnings.map((warning) => <p key={warning}><span aria-hidden="true">!</span>{warning}</p>)}
            </div>
          )}
          {report.warnings.length === 0 && <p className="security-clear">Явных проблем не найдено.</p>}
        </>
      ) : (
        <p className="inline-empty">Укажи URL, чтобы проверить запрос.</p>
      )}
    </aside>
  );
}
