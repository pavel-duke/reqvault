import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { WorkspaceSnapshot } from "../types";
import { WorkspaceOverview } from "./WorkspaceOverview";

const workspace: WorkspaceSnapshot = {
  root_path: "C:/api",
  config: {
    format_version: 1,
    id: "workspace-id",
    name: "Payments API",
    production_guard: {
      enabled: true,
      require_https: true,
      allowed_hosts: [],
      blocked_methods: ["DELETE"],
      block_secrets_in_url: true,
      block_private_networks: true,
      block_cross_origin_redirects: true,
    },
  },
  requests: [],
  environments: [{ relative_path: "environments/local.yaml", environment: { format_version: 1, name: "local", variables: {} } }],
};

describe("WorkspaceOverview", () => {
  it("показывает понятный старт и сохраняет семантику действий", () => {
    const onNewRequest = vi.fn();
    render(<WorkspaceOverview workspace={workspace} onNewRequest={onNewRequest} onImport={vi.fn()} onRun={vi.fn()} />);

    expect(screen.getByRole("heading", { level: 1, name: "Payments API" })).toBeInTheDocument();
    expect(screen.getByText("Данные хранятся локально")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Открыть runner" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Новый запрос" }));
    expect(onNewRequest).toHaveBeenCalledOnce();
  });
});
