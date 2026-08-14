import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { emptyRequest } from "../request-utils";
import { RequestTabs } from "./RequestTabs";

describe("RequestTabs", () => {
  it("показывает dirty-состояние и переключает вкладку", () => {
    const onSelect = vi.fn();
    const request = emptyRequest();
    request.name = "Пользователи";
    render(<RequestTabs tabs={[{ id: "one", relativePath: "requests/users.yaml", collection: "Общее", dirty: true, request, response: null, httpError: null }]} activeId="one" onSelect={onSelect} onClose={vi.fn()} onNew={vi.fn()} />);
    expect(screen.getByLabelText("Есть несохранённые изменения")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: /Пользователи/ }));
    expect(onSelect).toHaveBeenCalledWith("one");
  });
});
