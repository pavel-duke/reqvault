export type KeyValue = {
  name: string;
  value: string;
  enabled: boolean;
};

export type AuthConfig =
  | { type: "none" }
  | { type: "bearer"; token: string }
  | { type: "basic"; username: string; password: string }
  | { type: "api_key_header"; name: string; value: string }
  | { type: "api_key_query"; name: string; value: string };

export type BodyConfig =
  | { type: "none" }
  | { type: "json"; value: string }
  | { type: "raw"; value: string; content_type: string }
  | { type: "form_urlencoded"; fields: KeyValue[] };

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

export type WorkspaceSnapshot = {
  root_path: string;
  config: {
    format_version: 1;
    id: string;
    name: string;
  };
  requests: RequestSummary[];
  environments: EnvironmentSummary[];
};

export type Theme = "light" | "dark";
