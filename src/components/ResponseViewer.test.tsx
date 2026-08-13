import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { HttpResponse } from "../types";
import { ResponseViewer } from "./ResponseViewer";

const response: HttpResponse = {
  request_id: "request-1",
  status: 200,
  status_text: "OK",
  duration_ms: 124,
  size_bytes: 43,
  headers: [{ name: "content-type", value: "application/json" }],
  body: "{\"ok\":true,\"items\":[1,2]}",
  is_json: true,
};

describe("ResponseViewer", () => {
  it("показывает метрики и форматирует JSON", () => {
    render(<ResponseViewer response={response} error={null} loading={false} />);
    expect(screen.getByText("200 OK")).toBeInTheDocument();
    expect(screen.getByText("124 мс")).toBeInTheDocument();
    expect(screen.getByText(/"ok": true/)).toBeInTheDocument();
  });

  it("переключается на заголовки ответа", async () => {
    const user = userEvent.setup();
    render(<ResponseViewer response={response} error={null} loading={false} />);
    await user.click(screen.getByRole("button", { name: /Заголовки/ }));
    expect(screen.getByText("content-type")).toBeInTheDocument();
    expect(screen.getByText("application/json")).toBeInTheDocument();
  });

  it("не ломается на пустом теле", () => {
    render(<ResponseViewer response={{ ...response, body: "", is_json: false, size_bytes: 0 }} error={null} loading={false} />);
    expect(screen.getByText("Ответ не содержит тела")).toBeInTheDocument();
  });

  it("показывает понятную ошибку подключения", () => {
    render(<ResponseViewer response={null} error={{ message: "Не удалось подключиться к api.example.test", details: "connection refused", error_type: "connection" }} loading={false} />);
    expect(screen.getByText("Проверь адрес сервера и подключение к сети.")).toBeInTheDocument();
    expect(screen.queryByText("connection refused")).not.toBeVisible();
  });
});
