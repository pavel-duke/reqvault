import type { ResponseHeader } from "./types";

export type ComparableResponse = {
  status: number;
  status_text: string;
  headers: ResponseHeader[];
  body: string;
  is_json: boolean;
};

export type ValueChange = {
  kind: "added" | "removed" | "changed";
  path: string;
  before: string | null;
  after: string | null;
};

export type ResponseDiff = {
  statusChanged: boolean;
  beforeStatus: string;
  afterStatus: string;
  headerChanges: ValueChange[];
  bodyChanges: ValueChange[];
  bodyChanged: boolean;
  truncated: boolean;
};

const MAX_CHANGES = 250;

function printable(value: unknown): string {
  const serialized = typeof value === "string" ? value : JSON.stringify(value);
  if (serialized === undefined) return "undefined";
  return serialized.length > 180 ? `${serialized.slice(0, 177)}…` : serialized;
}

function headerMap(headers: ResponseHeader[]) {
  const result = new Map<string, { name: string; value: string }>();
  for (const header of headers) {
    const key = header.name.toLocaleLowerCase("en");
    const existing = result.get(key);
    result.set(key, { name: header.name, value: existing ? `${existing.value}, ${header.value}` : header.value });
  }
  return result;
}

function diffHeaders(before: ResponseHeader[], after: ResponseHeader[]): ValueChange[] {
  const left = headerMap(before);
  const right = headerMap(after);
  const names = [...new Set([...left.keys(), ...right.keys()])].sort();
  const changes: ValueChange[] = [];
  for (const key of names) {
    const oldValue = left.get(key);
    const newValue = right.get(key);
    if (!oldValue && newValue) changes.push({ kind: "added", path: newValue.name, before: null, after: newValue.value });
    else if (oldValue && !newValue) changes.push({ kind: "removed", path: oldValue.name, before: oldValue.value, after: null });
    else if (oldValue?.value !== newValue?.value) changes.push({ kind: "changed", path: newValue?.name ?? oldValue?.name ?? key, before: oldValue?.value ?? null, after: newValue?.value ?? null });
  }
  return changes;
}

function walkJson(before: unknown, after: unknown, path: string, changes: ValueChange[]) {
  if (changes.length >= MAX_CHANGES || Object.is(before, after)) return;
  const beforeObject = before !== null && typeof before === "object";
  const afterObject = after !== null && typeof after === "object";
  if (beforeObject && afterObject) {
    const left = before as Record<string, unknown>;
    const right = after as Record<string, unknown>;
    const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
    for (const key of keys) {
      if (changes.length >= MAX_CHANGES) return;
      const childPath = Array.isArray(before) || Array.isArray(after) ? `${path}[${key}]` : `${path}.${key}`;
      if (!(key in left)) changes.push({ kind: "added", path: childPath, before: null, after: printable(right[key]) });
      else if (!(key in right)) changes.push({ kind: "removed", path: childPath, before: printable(left[key]), after: null });
      else walkJson(left[key], right[key], childPath, changes);
    }
    return;
  }
  changes.push({ kind: "changed", path, before: printable(before), after: printable(after) });
}

export function diffResponses(before: ComparableResponse, after: ComparableResponse): ResponseDiff {
  const headerChanges = diffHeaders(before.headers, after.headers);
  const bodyChanges: ValueChange[] = [];
  if (before.is_json && after.is_json) {
    try {
      walkJson(JSON.parse(before.body), JSON.parse(after.body), "$", bodyChanges);
    } catch {
      // Некорректный JSON сравнивается ниже как обычный текст.
    }
  }
  const bodyChanged = before.body !== after.body;
  if (bodyChanged && bodyChanges.length === 0) {
    bodyChanges.push({
      kind: "changed",
      path: "$body",
      before: `${before.body.length} символов`,
      after: `${after.body.length} символов`,
    });
  }
  return {
    statusChanged: before.status !== after.status || before.status_text !== after.status_text,
    beforeStatus: `${before.status} ${before.status_text}`.trim(),
    afterStatus: `${after.status} ${after.status_text}`.trim(),
    headerChanges,
    bodyChanges,
    bodyChanged,
    truncated: bodyChanges.length >= MAX_CHANGES,
  };
}
