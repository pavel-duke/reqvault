import { beforeEach, describe, expect, it } from "vitest";
import { emptyRequest } from "./request-utils";
import { loadTabs, saveTabs } from "./tabs-storage";

describe("tabs-storage", () => {
  beforeEach(() => window.localStorage.clear());

  it("восстанавливает несохранённые вкладки без открытых credential", () => {
    const request = emptyRequest();
    request.auth = { type: "bearer", token: "plain-secret-must-not-stay" };
    saveTabs("workspace", "draft:1", [{
      id: "draft:1",
      relativePath: null,
      collection: "users",
      dirty: true,
      request,
      response: null,
      httpError: null,
    }]);
    const raw = window.localStorage.getItem("reqvault.tabs.workspace") ?? "";
    expect(raw).not.toContain("plain-secret-must-not-stay");
    const restored = loadTabs("workspace", []);
    expect(restored.tabs).toHaveLength(1);
    expect(restored.tabs[0].request.auth).toEqual({ type: "bearer", token: "" });
  });
});
