import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
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
      phase: "",
      version: null,
      downloaded: 0,
      total: null,
    })
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
  await waitFor(() =>
    expect(screen.getByText(t[13]).hasAttribute("disabled")).toBe(false),
  );
  expect(invoke).toHaveBeenCalledWith("update_status");
  fireEvent.click(screen.getByText(t[13]));
  fireEvent.click(await screen.findByText(t[15] + " v0.3.0"));
  expect(await screen.findByText(t[18])).toBeTruthy();
  expect(invoke).not.toHaveBeenCalledWith("restart_after_update");
});
it("uses Mac App Store update guidance in every locale and offers no installer action", async () => {
  vi.mocked(invoke).mockResolvedValue({
    phase: "app_store",
    version: null,
    downloaded: 0,
    total: null,
  });
  for (const locale of Object.keys(
    runtimeMessages,
  ) as (keyof typeof runtimeMessages)[]) {
    const { unmount } = render(
      <UpdatePanel native locale={locale} dirty={false} />,
    );
    expect(await screen.findByText(runtimeMessages[locale][25])).toBeTruthy();
    expect(screen.queryByText(runtimeMessages[locale][13])).toBeNull();
    expect(
      screen.queryByText(runtimeMessages[locale][15], { exact: false }),
    ).toBeNull();
    unmount();
  }
  expect(
    vi
      .mocked(invoke)
      .mock.calls.every(([command]) => command === "update_status"),
  ).toBe(true);
});
it("shows no feed rather than claiming the current version is latest", async () => {
  vi.mocked(invoke)
    .mockResolvedValueOnce({
      phase: "",
      version: null,
      downloaded: 0,
      total: null,
    })
    .mockResolvedValueOnce({
      phase: "unpublished",
      version: null,
      downloaded: 0,
      total: null,
    });
  render(<UpdatePanel native locale="zh-Hant" dirty={false} />);
  await waitFor(() =>
    expect(
      screen.getByText(runtimeMessages["zh-Hant"][13]).hasAttribute("disabled"),
    ).toBe(false),
  );
  fireEvent.click(screen.getByText(runtimeMessages["zh-Hant"][13]));
  expect(await screen.findByText(runtimeMessages["zh-Hant"][20])).toBeTruthy();
  expect(screen.queryByText(runtimeMessages["zh-Hant"][19])).toBeNull();
  for (const t of Object.values(runtimeMessages))
    expect(t.length).toBe(runtimeMessages.en.length);
});
