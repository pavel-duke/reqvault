import type { ResponseAssertion } from "../types";

type Props = {
  assertions: ResponseAssertion[];
  onChange: (assertions: ResponseAssertion[]) => void;
};

type AssertionType = ResponseAssertion["type"];

function assertionForType(type: AssertionType): ResponseAssertion {
  switch (type) {
    case "header": return { type, name: "Content-Type", operator: "contains", expected: "application/json", enabled: true };
    case "json_path": return { type, path: "$.data.id", operator: "exists", expected: "", enabled: true };
    case "body_contains": return { type, expected: "", enabled: true };
    case "response_time": return { type, max_ms: 1000, enabled: true };
    default: return { type: "status", expected: 200, enabled: true };
  }
}

export function AssertionsEditor({ assertions, onChange }: Props) {
  function update(index: number, assertion: ResponseAssertion) {
    const next = [...assertions];
    next[index] = assertion;
    onChange(next);
  }

  return (
    <div className="assertions-editor">
      <div className="assertions-heading">
        <div>
          <strong>Проверки ответа</strong>
          <p className="help-text">Используются при запуске коллекции из приложения и CLI.</p>
        </div>
        <button className="secondary-button" type="button" onClick={() => onChange([...assertions, assertionForType("status")])}>Добавить проверку</button>
      </div>
      {assertions.length === 0 && <div className="empty-inline">Проверок пока нет. Runner всё равно считает HTTP 4xx/5xx ошибкой.</div>}
      {assertions.map((assertion, index) => (
        <div className="assertion-row" key={index}>
          <input type="checkbox" checked={assertion.enabled} onChange={(event) => update(index, { ...assertion, enabled: event.currentTarget.checked })} aria-label="Включить проверку" />
          <select value={assertion.type} onChange={(event) => update(index, assertionForType(event.currentTarget.value as AssertionType))} aria-label="Тип проверки">
            <option value="status">HTTP status</option>
            <option value="header">Заголовок</option>
            <option value="json_path">JSON path</option>
            <option value="body_contains">Тело содержит</option>
            <option value="response_time">Время ответа</option>
          </select>
          {assertion.type === "status" && <label><span>Ожидается</span><input type="number" min="100" max="599" value={assertion.expected} onChange={(event) => update(index, { ...assertion, expected: Number(event.currentTarget.value) || 200 })} /></label>}
          {assertion.type === "header" && <><label><span>Заголовок</span><input value={assertion.name} onChange={(event) => update(index, { ...assertion, name: event.currentTarget.value })} /></label><Operator value={assertion.operator} onChange={(operator) => update(index, { ...assertion, operator })} />{assertion.operator !== "exists" && <label><span>Значение</span><input value={assertion.expected} onChange={(event) => update(index, { ...assertion, expected: event.currentTarget.value })} /></label>}</>}
          {assertion.type === "json_path" && <><label><span>Путь</span><input value={assertion.path} onChange={(event) => update(index, { ...assertion, path: event.currentTarget.value })} placeholder="$.data.id" /></label><Operator value={assertion.operator} onChange={(operator) => update(index, { ...assertion, operator })} />{assertion.operator !== "exists" && <label><span>Значение</span><input value={assertion.expected} onChange={(event) => update(index, { ...assertion, expected: event.currentTarget.value })} /></label>}</>}
          {assertion.type === "body_contains" && <label className="assertion-wide"><span>Фрагмент текста</span><input value={assertion.expected} onChange={(event) => update(index, { ...assertion, expected: event.currentTarget.value })} /></label>}
          {assertion.type === "response_time" && <label><span>Не дольше, мс</span><input type="number" min="1" value={assertion.max_ms} onChange={(event) => update(index, { ...assertion, max_ms: Number(event.currentTarget.value) || 1 })} /></label>}
          <button className="quiet-icon" type="button" onClick={() => onChange(assertions.filter((_, itemIndex) => itemIndex !== index))} aria-label="Удалить проверку">×</button>
        </div>
      ))}
    </div>
  );
}

function Operator({ value, onChange }: { value: "exists" | "equals" | "contains"; onChange: (value: "exists" | "equals" | "contains") => void }) {
  return <label><span>Условие</span><select value={value} onChange={(event) => onChange(event.currentTarget.value as "exists" | "equals" | "contains")}><option value="exists">существует</option><option value="equals">равно</option><option value="contains">содержит</option></select></label>;
}
