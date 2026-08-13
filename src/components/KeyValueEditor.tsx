import type { KeyValue } from "../types";

type Props = {
  rows: KeyValue[];
  onChange: (rows: KeyValue[]) => void;
  namePlaceholder?: string;
  valuePlaceholder?: string;
  emptyText: string;
};

export function KeyValueEditor({
  rows,
  onChange,
  namePlaceholder = "Имя",
  valuePlaceholder = "Значение",
  emptyText,
}: Props) {
  function update(index: number, patch: Partial<KeyValue>) {
    onChange(rows.map((row, rowIndex) => (rowIndex === index ? { ...row, ...patch } : row)));
  }

  function remove(index: number) {
    onChange(rows.filter((_, rowIndex) => rowIndex !== index));
  }

  return (
    <div className="key-value-editor">
      {rows.length === 0 && <p className="inline-empty">{emptyText}</p>}
      {rows.map((row, index) => (
        <div className="key-value-row" key={`${index}-${row.name}`}>
          <input
            className="row-check"
            type="checkbox"
            checked={row.enabled}
            onChange={(event) => update(index, { enabled: event.currentTarget.checked })}
            aria-label="Использовать поле"
          />
          <input
            value={row.name}
            onChange={(event) => update(index, { name: event.currentTarget.value })}
            placeholder={namePlaceholder}
            aria-label={namePlaceholder}
          />
          <input
            value={row.value}
            onChange={(event) => update(index, { value: event.currentTarget.value })}
            placeholder={valuePlaceholder}
            aria-label={valuePlaceholder}
          />
          <button
            className="remove-row"
            type="button"
            onClick={() => remove(index)}
            aria-label="Удалить строку"
            title="Удалить"
          >
            ×
          </button>
        </div>
      ))}
      <button
        className="text-button"
        type="button"
        onClick={() => onChange([...rows, { name: "", value: "", enabled: true }])}
      >
        + Добавить строку
      </button>
    </div>
  );
}
