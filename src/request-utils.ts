import type { KeyValue, RequestFile } from "./types";

export function emptyRequest(): RequestFile {
  return {
    format_version: 1,
    name: "Новый запрос",
    method: "GET",
    url: "",
    headers: { Accept: "application/json" },
    query: [],
    auth: { type: "none" },
    body: { type: "none" },
    timeout_ms: 30_000,
    follow_redirects: true,
    transport: {
      proxy: { type: "none" },
      custom_ca_path: "",
      client_certificate_path: "",
      client_key_path: "",
    },
    tests: [],
  };
}

export function recordToRows(record: Record<string, string>): KeyValue[] {
  return Object.entries(record).map(([name, value]) => ({
    name,
    value,
    enabled: true,
  }));
}

export function rowsToRecord(rows: KeyValue[]): Record<string, string> {
  return Object.fromEntries(
    rows
      .filter((row) => row.enabled && row.name.trim())
      .map((row) => [row.name.trim(), row.value]),
  );
}

export function collectionFromPath(relativePath: string | null): string {
  if (!relativePath) return "Общее";
  const parts = relativePath.split("/");
  return parts.length > 2 ? parts[1] : "Общее";
}
