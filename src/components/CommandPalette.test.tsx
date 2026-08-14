import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { RequestSummary } from "../types";
import { CommandPalette } from "./CommandPalette";

const request: RequestSummary = {
  relative_path: "requests/users/get-user.yaml",
  request: {
    format_version: 1,
    name: "Получить пользователя",
    method: "GET",
    url: "https://api.example.test/users/42",
    headers: { "X-Trace-Token": "DO_NOT_INDEX_THIS_VALUE" },
    query: [],
    auth: { type: "none" },
    body: { type: "none" },
    timeout_ms: 30000,
    follow_redirects: true,
    transport: { proxy: { type: "none" }, custom_ca_path: "", client_certificate_path: "", client_key_path: "" },
    tests: [],
  },
};

describe("CommandPalette", () => {
  it("ищет по безопасным полям и не индексирует значения заголовков", () => {
    render(<CommandPalette open actions={[]} requests={[request]} recentPaths={[]} onOpenRequest={vi.fn()} onClose={vi.fn()} />);
    const input = screen.getByRole("combobox");
    fireEvent.change(input, { target: { value: "X-Trace-Token" } });
    expect(screen.getByText("Получить пользователя")).toBeInTheDocument();
    fireEvent.change(input, { target: { value: "DO_NOT_INDEX_THIS_VALUE" } });
    expect(screen.queryByText("Получить пользователя")).not.toBeInTheDocument();
  });

  it("выполняет выбранную команду с клавиатуры", () => {
    const run = vi.fn();
    render(<CommandPalette open actions={[{ id: "new", label: "Новый запрос", description: "Создать", icon: "file", onSelect: run }]} requests={[]} recentPaths={[]} onOpenRequest={vi.fn()} onClose={vi.fn()} />);
    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
    expect(run).toHaveBeenCalledOnce();
  });
});
