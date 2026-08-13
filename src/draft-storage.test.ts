import { describe, expect, it } from "vitest";
import { emptyRequest } from "./request-utils";
import { sanitizeDraft } from "./draft-storage";

describe("sanitizeDraft", () => {
  it("не сохраняет credential из auth, headers, query, body и proxy", () => {
    const request = emptyRequest();
    request.url = "https://api.test/users?access_token=url-secret";
    request.headers = { Authorization: "Bearer header-secret" };
    request.query = [{ name: "api_key", value: "query-secret", enabled: true }];
    request.auth = { type: "basic", username: "pavel", password: "auth-secret" };
    request.body = { type: "json", value: '{"name":"Pavel","password":"body-secret"}' };
    request.transport.proxy = { type: "custom", url: "http://proxy", username: "pavel", password: "proxy-secret" };
    const serialized = JSON.stringify(sanitizeDraft(request));
    for (const secret of ["url-secret", "header-secret", "query-secret", "auth-secret", "body-secret", "proxy-secret"]) {
      expect(serialized).not.toContain(secret);
    }
    expect(serialized).toContain("Pavel");
  });

  it("сохраняет только ссылки Secret Vault", () => {
    const request = emptyRequest();
    request.auth = { type: "bearer", token: "{{secret:API_TOKEN}}" };
    expect(sanitizeDraft(request).auth).toEqual(request.auth);
  });
});
