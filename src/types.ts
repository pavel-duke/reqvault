export type KeyValue = {
  name: string;
  value: string;
  enabled: boolean;
};

export type MultipartField =
  | { type: "text"; name: string; value: string; enabled: boolean }
  | { type: "file"; name: string; path: string; content_type: string; enabled: boolean };

export type ProxyConfig =
  | { type: "none" }
  | { type: "system" }
  | { type: "custom"; url: string; username: string; password: string };

export type TransportConfig = {
  proxy: ProxyConfig;
  custom_ca_path: string;
  client_certificate_path: string;
  client_key_path: string;
};

export type AuthConfig =
  | { type: "none" }
  | { type: "bearer"; token: string }
  | { type: "basic"; username: string; password: string }
  | { type: "digest"; username: string; password: string }
  | { type: "api_key_header"; name: string; value: string }
  | { type: "api_key_query"; name: string; value: string }
  | {
      type: "aws_sig_v4";
      access_key: string;
      secret_key: string;
      session_token: string;
      region: string;
      service: string;
    }
  | {
      type: "oauth2";
      grant_type: "authorization_code_pkce" | "client_credentials";
      authorization_url: string;
      token_url: string;
      client_id: string;
      client_secret: string;
      scopes: string;
      access_token: string;
      refresh_token: string;
    };

export type BodyConfig =
  | { type: "none" }
  | { type: "json"; value: string }
  | { type: "graphql"; query: string; variables: string; operation_name: string }
  | { type: "raw"; value: string; content_type: string }
  | { type: "form_urlencoded"; fields: KeyValue[] }
  | { type: "multipart"; fields: MultipartField[] };

export type ResponseAssertion =
  | { type: "status"; expected: number; enabled: boolean }
  | { type: "header"; name: string; operator: "exists" | "equals" | "contains"; expected: string; enabled: boolean }
  | { type: "json_path"; path: string; operator: "exists" | "equals" | "contains"; expected: string; enabled: boolean }
  | { type: "body_contains"; expected: string; enabled: boolean }
  | { type: "response_time"; max_ms: number; enabled: boolean };

export type RequestFile = {
  format_version: 1;
  name: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  query: KeyValue[];
  auth: AuthConfig;
  body: BodyConfig;
  timeout_ms: number;
  follow_redirects: boolean;
  transport: TransportConfig;
  tests: ResponseAssertion[];
};

export type EnvironmentFile = {
  format_version: 1;
  name: string;
  variables: Record<string, string>;
};

export type RequestSummary = {
  relative_path: string;
  request: RequestFile;
};

export type EnvironmentSummary = {
  relative_path: string;
  environment: EnvironmentFile;
};

export type ProductionGuard = {
  enabled: boolean;
  require_https: boolean;
  allowed_hosts: string[];
  blocked_methods: string[];
  block_secrets_in_url: boolean;
};

export type WorkspaceConfig = {
  format_version: 1;
  id: string;
  name: string;
  production_guard: ProductionGuard;
};

export type WorkspaceSnapshot = {
  root_path: string;
  config: WorkspaceConfig;
  requests: RequestSummary[];
  environments: EnvironmentSummary[];
};

export type ResponseHeader = {
  name: string;
  value: string;
};

export type HttpResponse = {
  request_id: string;
  status: number;
  status_text: string;
  duration_ms: number;
  size_bytes: number;
  headers: ResponseHeader[];
  body: string;
  is_json: boolean;
  content_type: string;
  body_kind: "json" | "text" | "html" | "image" | "binary";
  truncated: boolean;
};

export type HttpError = {
  message: string;
  details: string | null;
  error_type: string;
};

export type OAuthResult = {
  access_token_secret: string;
  refresh_token_secret: string | null;
  expires_in: number | null;
  scope: string | null;
};

export type ImportResult = {
  source: string;
  imported_requests: number;
  imported_environments: number;
  warnings: string[];
  workspace: WorkspaceSnapshot;
};

export type HistorySettings = {
  enabled: boolean;
  max_entries: number;
};

export type HistorySummary = {
  id: string;
  created_at_ms: number;
  request_name: string;
  method: string;
  url: string;
  status: number;
  duration_ms: number;
  size_bytes: number;
};

export type HistoryEntry = {
  summary: HistorySummary;
  status_text: string;
  headers: ResponseHeader[];
  body: string;
  is_json: boolean;
  content_type: string;
  body_kind: "json" | "text" | "html" | "image" | "binary";
  truncated: boolean;
};

export type CookieSummary = {
  id: string;
  name: string;
  domain: string;
  path: string;
  secure: boolean;
  http_only: boolean;
  expires_at: number | null;
};

export type DiagnosticIssue = {
  severity: "error" | "warning" | "info";
  code: string;
  path: string;
  message: string;
  remediation: string;
};

export type MigrationPlan = {
  required: boolean;
  current_version: number;
  target_version: number;
  files: string[];
  changes: string[];
  warnings: string[];
};

export type WorkspaceDiagnostics = {
  checked_at_ms: number;
  fingerprint: string;
  files: number;
  requests: number;
  environments: number;
  errors: number;
  warnings: number;
  issues: DiagnosticIssue[];
  migration: MigrationPlan;
};

export type MigrationResult = {
  backup_id: string | null;
  workspace: WorkspaceSnapshot;
};

export type SecurityReport = {
  https: boolean;
  host: string;
  secrets: number;
  in_headers: number;
  in_query: number;
  warnings: string[];
};

export type AssertionResult = {
  passed: boolean;
  label: string;
  expected: string;
  actual: string;
};

export type RequestRunResult = {
  relative_path: string;
  request_name: string;
  method: string;
  status: number | null;
  duration_ms: number | null;
  passed: boolean;
  assertions: AssertionResult[];
  error: string | null;
};

export type CollectionRunReport = {
  started_at_ms: number;
  duration_ms: number;
  total: number;
  passed: number;
  failed: number;
  results: RequestRunResult[];
};

export type CollectionRunOptions = {
  environment: string | null;
  collection: string | null;
  stop_on_failure: boolean;
};

export type StreamConnectConfig = {
  protocol: "websocket" | "sse";
  url: string;
  headers: Record<string, string>;
  workspace_id: string;
  workspace_path: string;
  environment: EnvironmentFile | null;
};

export type StreamEvent = {
  session_id: string;
  kind: string;
  timestamp_ms: number;
  data: string;
};

export type Theme = "light" | "dark";
