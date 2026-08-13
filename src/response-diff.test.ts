import { describe, expect, it } from "vitest";
import { diffResponses } from "./response-diff";

describe("diffResponses", () => {
  it("сравнивает status, заголовки и JSON по путям", () => {
    const diff = diffResponses(
      { status: 200, status_text: "OK", headers: [{ name: "ETag", value: "one" }], body: '{"user":{"id":1,"name":"Pavel"}}', is_json: true },
      { status: 201, status_text: "Created", headers: [{ name: "ETag", value: "two" }, { name: "X-Trace", value: "42" }], body: '{"user":{"id":1,"name":"Duke"},"active":true}', is_json: true },
    );
    expect(diff.statusChanged).toBe(true);
    expect(diff.headerChanges).toEqual(expect.arrayContaining([expect.objectContaining({ path: "ETag", kind: "changed" }), expect.objectContaining({ path: "X-Trace", kind: "added" })]));
    expect(diff.bodyChanges).toEqual(expect.arrayContaining([expect.objectContaining({ path: "$.active", kind: "added" }), expect.objectContaining({ path: "$.user.name", kind: "changed" })]));
  });

  it("не сообщает изменения для одинаковых ответов", () => {
    const response = { status: 204, status_text: "No Content", headers: [], body: "", is_json: false };
    const diff = diffResponses(response, response);
    expect(diff.statusChanged).toBe(false);
    expect(diff.headerChanges).toHaveLength(0);
    expect(diff.bodyChanged).toBe(false);
  });
});
