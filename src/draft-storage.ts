import type { AuthConfig, BodyConfig, KeyValue, ProxyConfig, RequestFile } from "./types";

export type StoredDraft = {
  relativePath: string | null;
  collection: string;
  updatedAt: number;
  request: RequestFile;
};

const SECRET_REFERENCE = /^\s*\{\{secret:[A-Za-z][A-Za-z0-9_.-]*\}\}\s*$/;
const SENSITIVE_NAME = /(authorization|cookie|token|secret|password|passwd|api[-_]?key|session)/i;

function credential(value: string): string {
  return !value.trim() || SECRET_REFERENCE.test(value) ? value : "";
}

function sanitizeAuth(auth: AuthConfig): AuthConfig {
  switch (auth.type) {
    case "bearer": return { ...auth, token: credential(auth.token) };
    case "basic": return { ...auth, password: credential(auth.password) };
    case "digest": return { ...auth, password: credential(auth.password) };
    case "api_key_header":
    case "api_key_query": return { ...auth, value: credential(auth.value) };
    case "oauth2": return {
      ...auth,
      client_secret: credential(auth.client_secret),
      access_token: credential(auth.access_token),
      refresh_token: credential(auth.refresh_token),
    };
    case "aws_sig_v4": return {
      ...auth,
      access_key: credential(auth.access_key),
      secret_key: credential(auth.secret_key),
      session_token: credential(auth.session_token),
    };
    default: return auth;
  }
}

function sanitizeProxy(proxy: ProxyConfig): ProxyConfig {
  return proxy.type === "custom" ? { ...proxy, password: credential(proxy.password) } : proxy;
}

function sanitizePairs(fields: KeyValue[]): KeyValue[] {
  return fields.map((field) => SENSITIVE_NAME.test(field.name) ? { ...field, value: credential(field.value) } : field);
}

function sanitizeJson(value: string): string {
  try {
    const parsed = JSON.parse(value) as unknown;
    const walk = (item: unknown, key = ""): unknown => {
      if (SENSITIVE_NAME.test(key)) return "***REDACTED***";
      if (Array.isArray(item)) return item.map((value) => walk(value));
      if (item && typeof item === "object") {
        return Object.fromEntries(Object.entries(item).map(([name, value]) => [name, walk(value, name)]));
      }
      return item;
    };
    return JSON.stringify(walk(parsed), null, 2);
  } catch {
    return "";
  }
}

function sanitizeBody(body: BodyConfig): BodyConfig {
  switch (body.type) {
    case "json": return { ...body, value: sanitizeJson(body.value) };
    case "graphql": return { ...body, variables: sanitizeJson(body.variables) };
    case "raw": return { ...body, value: "" };
    case "form_urlencoded": return { ...body, fields: sanitizePairs(body.fields) };
    case "multipart": return {
      ...body,
      fields: body.fields.map((field) => field.type === "text" && SENSITIVE_NAME.test(field.name)
        ? { ...field, value: credential(field.value) }
        : field),
    };
    default: return body;
  }
}

function sanitizeUrl(value: string): string {
  return value.replace(/([?&](?:access_token|token|api_key|key|secret|password|session)=)[^&#]*/gi, "$1***REDACTED***");
}

export function sanitizeDraft(request: RequestFile): RequestFile {
  return {
    ...structuredClone(request),
    url: sanitizeUrl(request.url),
    headers: Object.fromEntries(Object.entries(request.headers).map(([name, value]) => [
      name,
      SENSITIVE_NAME.test(name) ? credential(value) : value.replace(/\b(Bearer|Basic)\s+[^\s,;]+/gi, "$1 ***REDACTED***"),
    ])),
    query: sanitizePairs(request.query),
    auth: sanitizeAuth(request.auth),
    body: sanitizeBody(request.body),
    transport: { ...request.transport, proxy: sanitizeProxy(request.transport.proxy) },
  };
}

export function draftStorageKey(workspaceId: string): string {
  return `reqvault.draft.${workspaceId}`;
}
