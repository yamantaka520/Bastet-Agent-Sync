import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import OperationsPanel from "./OperationsPanel";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

afterEach(() => {
  cleanup();
  invoke.mockReset();
});

it("recovers the operations startup view after transient wizard lock contention", async () => {
  invoke
    .mockRejectedValueOnce("sync_busy")
    .mockResolvedValueOnce({ history: [], devices: [] });
  render(<OperationsPanel native locale="en" runtime={null} />);
  await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  expect(invoke.mock.calls.map(([command]) => command)).toEqual([
    "operations_view",
    "operations_view",
  ]);
  expect(screen.queryByRole("alert")).toBeNull();
});

it("keeps a real startup read failure visible without retrying it", async () => {
  invoke.mockRejectedValue("store_unavailable");
  render(<OperationsPanel native locale="en" runtime={null} />);
  expect(await screen.findByRole("alert")).toBeTruthy();
  expect(invoke).toHaveBeenCalledTimes(1);
});

it("shows a local-language busy message when the bounded read retry is exhausted", async () => {
  invoke.mockRejectedValue("sync_busy");
  render(<OperationsPanel native locale="ja" runtime={null} />);
  expect((await screen.findByRole("alert")).textContent).toContain(
    "ローカル同期データが一時的に使用中",
  );
  expect(invoke).toHaveBeenCalledTimes(3);
});
