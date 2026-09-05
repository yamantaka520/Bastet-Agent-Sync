import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import UpdatePanel from "../src/UpdatePanel";
import { runtimeMessages } from "../src/runtime-i18n";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});
it("checks explicitly, installs only after a click, then offers restart", async () => {
  const t = runtimeMessages.en;
  vi.mocked(invoke)
    .mockResolvedValueOnce({
      phase: "available",
      version: "0.3.0",
      downloaded: 0,
      total: null,
    })
    .mockResolvedValueOnce({
      phase: "installed",
      version: "0.3.0",
      downloaded: 100,
      total: 100,
    });
  render(<UpdatePanel native locale="en" dirty={false} />);
  expect(invoke).not.toHaveBeenCalled();
  fireEvent.click(screen.getByText(t[13]));
  fireEvent.click(await screen.findByText(t[15] + " v0.3.0"));
  expect(await screen.findByText(t[18])).toBeTruthy();
  expect(invoke).not.toHaveBeenCalledWith("restart_after_update");
});
it("shows no feed rather than claiming the current version is latest", async () => {
  vi.mocked(invoke).mockResolvedValue({
    phase: "unpublished",
    version: null,
    downloaded: 0,
    total: null,
  });
  render(<UpdatePanel native locale="zh-Hant" dirty={false} />);
  fireEvent.click(screen.getByText(runtimeMessages["zh-Hant"][13]));
  expect(await screen.findByText(runtimeMessages["zh-Hant"][20])).toBeTruthy();
  expect(screen.queryByText(runtimeMessages["zh-Hant"][19])).toBeNull();
  for (const t of Object.values(runtimeMessages))
    expect(t.length).toBe(runtimeMessages.en.length);
});
