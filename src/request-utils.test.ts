import { describe, expect, it } from "vitest";
import { collectionFromPath, emptyRequest, recordToRows, rowsToRecord } from "./request-utils";

describe("request-utils", () => {
  it("создаёт пустой GET-запрос с безопасными настройками", () => {
    const request = emptyRequest();
    expect(request.method).toBe("GET");
    expect(request.timeout_ms).toBe(30_000);
    expect(request.follow_redirects).toBe(true);
    expect(request.auth).toEqual({ type: "none" });
  });

  it("преобразует только включённые строки с именем", () => {
    const record = rowsToRecord([
      { name: "Accept", value: "application/json", enabled: true },
      { name: "Disabled", value: "value", enabled: false },
      { name: "", value: "ignored", enabled: true },
    ]);
    expect(record).toEqual({ Accept: "application/json" });
    expect(recordToRows(record)).toEqual([
      { name: "Accept", value: "application/json", enabled: true },
    ]);
  });

  it("получает имя коллекции из пути", () => {
    expect(collectionFromPath("requests/users/get-user.yaml")).toBe("users");
    expect(collectionFromPath(null)).toBe("Общее");
  });
});
