import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { emptyRequest } from "../request-utils";
import { VirtualRequestTree, type VirtualRequestRow } from "./VirtualRequestTree";

describe("VirtualRequestTree", () => {
  it("не создаёт тысячи DOM-строк одновременно", () => {
    const rows: VirtualRequestRow[] = Array.from({ length: 1_000 }, (_, index) => {
      const request = emptyRequest();
      request.name = `Request ${index}`;
      return { kind: "request", key: String(index), favorite: false, summary: { relative_path: `requests/items/${index}.yaml`, request } };
    });
    render(<VirtualRequestTree rows={rows} selectedPath={null} onSelectRequest={vi.fn()} onToggleFavorite={vi.fn()} />);
    expect(screen.getAllByRole("button").length).toBeLessThan(50);
    expect(screen.getByRole("navigation", { name: "Запросы workspace" })).toBeInTheDocument();
  });
});
