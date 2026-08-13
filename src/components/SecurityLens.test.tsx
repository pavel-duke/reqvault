import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SecurityLens } from "./SecurityLens";

describe("SecurityLens", () => {
  it("показывает предупреждение о секрете в query", () => {
    render(
      <SecurityLens
        report={{ https: true, host: "api.example.test", secrets: 1, in_headers: 0, in_query: 1, warnings: ["Секрет используется в URL."] }}
        copyStatus={null}
        onCopy={vi.fn()}
      />,
    );
    expect(screen.getByText("api.example.test")).toBeInTheDocument();
    expect(screen.getByText("Секрет используется в URL.")).toBeInTheDocument();
  });
});
