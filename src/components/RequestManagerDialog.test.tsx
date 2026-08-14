import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { emptyRequest } from "../request-utils";
import { RequestManagerDialog } from "./RequestManagerDialog";

describe("RequestManagerDialog", () => {
  it("передаёт выбранные пути в массовую операцию", () => {
    const request = emptyRequest();
    request.name = "Пользователи";
    const onMove = vi.fn();
    render(<RequestManagerDialog requests={[{ relative_path: "requests/users/list.yaml", request }]} busy={false} error={null} onMove={onMove} onDuplicate={vi.fn()} onRename={vi.fn()} onClose={vi.fn()} />);
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.change(screen.getByLabelText("Целевая коллекция"), { target: { value: "Archive" } });
    fireEvent.click(screen.getByRole("button", { name: "Переместить" }));
    expect(onMove).toHaveBeenCalledWith(["requests/users/list.yaml"], "Archive");
  });
});
