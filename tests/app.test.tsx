import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { messages, detectLocale, languages } from "../src/i18n";
import { defaults } from "../src/model";
import App from "../src/App";
const api = vi.hoisted(() => ({ native: false, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => api.native,
  invoke: (command: string, ...args: unknown[]) =>
    command === "wizard_get"
      ? Promise.resolve(null)
      : api.invoke(command, ...args),
}));
afterEach(() => {
  cleanup();
  api.native = false;
  api.invoke.mockReset();
});
describe("locale and native setup contracts", () => {
  it("keeps all five locales complete and chooses regional Chinese", () => {
    for (const locale of Object.keys(languages) as (keyof typeof languages)[]) {
      expect(Object.keys(messages[locale]).sort()).toEqual(
        Object.keys(messages.en).sort(),
      );
      expect(Object.values(messages[locale]).every(Boolean)).toBe(true);
    }
    expect(detectLocale("zh-TW")).toBe("zh-Hant");
    expect(detectLocale("zh-CN")).toBe("zh-Hans");
    expect(detectLocale("ja-JP")).toBe("ja");
    expect(detectLocale("ko-KR")).toBe("ko");
    expect(detectLocale("fr-FR")).toBe("en");
  });
  it("never simulates native discovery or synchronization in browser preview", () => {
    render(<App />);
    expect(screen.getByText(messages.en.preview)).toBeTruthy();
    expect(
      (screen.getByRole("button", { name: /Start sync/ }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    expect(
      (screen.getByRole("button", { name: "Save setup" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    expect(api.invoke).not.toHaveBeenCalled();
  });
  it("loads actual discovery, saves selections, and changes language", async () => {
    api.native = true;
    const s = { ...defaults("en"), deviceName: "Test computer" };
    api.invoke.mockImplementation(async (command: string) =>
      command === "bootstrap"
        ? {
            settings: s,
            agents: [
              { id: "codex", path: "/fixture", detected: true, custom: false },
            ],
            trayAvailable: true,
          }
        : undefined,
    );
    render(<App />);
    await screen.findByText("/fixture");
    fireEvent.click(screen.getByRole("checkbox", { name: "Codex" }));
    fireEvent.click(screen.getByRole("button", { name: "Save setup" }));
    await screen.findByText(messages.en.saved);
    expect(api.invoke).toHaveBeenCalledWith("save_settings", {
      settings: { ...s, selectedAgents: ["codex"] },
    });
    fireEvent.change(screen.getByLabelText("Language"), {
      target: { value: "ja" },
    });
    expect(await screen.findByText(messages.ja.setup)).toBeTruthy();
  });
  it("preserves error state and prevents save after unreadable settings", async () => {
    api.native = true;
    api.invoke.mockRejectedValue("settings_unreadable");
    render(<App />);
    await screen.findByRole("alert");
    expect(screen.getByRole("alert").textContent).toBe(
      messages.en.settings_unreadable,
    );
    expect(
      (screen.getByRole("button", { name: "Save setup" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });
  it("reports save failures without a success message", async () => {
    api.native = true;
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "bootstrap")
        return {
          settings: { ...defaults("en"), deviceName: "Test" },
          agents: [],
          trayAvailable: false,
        };
      throw "overlapping_folder";
    });
    render(<App />);
    await waitFor(() =>
      expect(
        (
          screen.getByRole("button", {
            name: "Save setup",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(false),
    );
    fireEvent.click(screen.getByRole("button", { name: "Save setup" }));
    expect((await screen.findByRole("alert")).textContent).toBe(
      messages.en.overlapping_folder,
    );
    expect(screen.queryByText(messages.en.saved)).toBeNull();
  });
});

describe("isolated synchronization check", () => {
  it("uses only the native synthetic diagnostic command and shows its results", async () => {
    api.native = true;
    api.invoke.mockImplementation(async (command: string) =>
      command === "bootstrap"
        ? {
            settings: { ...defaults("en"), deviceName: "Test" },
            agents: [],
            trayAvailable: false,
          }
        : {
            verified: true,
            transferred: 2,
            preservedBranches: 2,
            repeatTransfers: 0,
            recoveredObjects: 3,
          },
    );
    render(<App />);
    await waitFor(() =>
      expect(
        (
          screen.getByRole("button", {
            name: messages.en.runDiagnostic,
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(false),
    );
    fireEvent.click(
      screen.getByRole("button", { name: messages.en.runDiagnostic }),
    );
    expect(await screen.findByText(messages.en.diagnosticPassed)).toBeTruthy();
    expect(api.invoke).toHaveBeenCalledWith("run_sync_diagnostic");
    expect(
      api.invoke.mock.calls.every(([name]) =>
        ["bootstrap", "sync_status", "run_sync_diagnostic"].includes(name),
      ),
    ).toBe(true);
    expect(
      (screen.getByRole("button", { name: /Start sync/ }) as HTMLButtonElement)
        .disabled,
    ).toBe(false);
  });
  it("clears previous success before a failed retry", async () => {
    api.native = true;
    let runs = 0;
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "bootstrap")
        return { settings: defaults("en"), agents: [], trayAvailable: false };
      if (command !== "run_sync_diagnostic") return undefined;
      if (runs++ === 0)
        return {
          verified: true,
          transferred: 2,
          preservedBranches: 2,
          repeatTransfers: 0,
          recoveredObjects: 3,
        };
      throw "diagnostic_failed";
    });
    render(<App />);
    const button = screen.getByRole("button", {
      name: messages.en.runDiagnostic,
    });
    await waitFor(() =>
      expect((button as HTMLButtonElement).disabled).toBe(false),
    );
    fireEvent.click(button);
    await screen.findByText(messages.en.diagnosticPassed);
    fireEvent.click(button);
    await screen.findByRole("alert");
    expect(screen.queryByText(messages.en.diagnosticPassed)).toBeNull();
  });
});

it("starts the real worker and allows pause even with skipped sources", async () => {
  api.native = true;
  api.invoke.mockImplementation(async (command: string) => {
    if (command === "bootstrap")
      return {
        settings: {
          ...defaults("en"),
          deviceName: "Fixture",
          selectedAgents: ["codex", "agent-memory-os"],
        },
        agents: [],
        trayAvailable: true,
        version: "0.2.1",
        revision: "fixture",
      };
    if (command === "sync_start")
      return {
        running: true,
        phase: "starting",
        published: 0,
        received: 0,
        applied: 0,
        skipped: ["codex"],
        lastSuccess: null,
        error: null,
      };
  });
  render(<App />);
  await screen.findByText("v0.2.1 · fixture");
  fireEvent.click(screen.getByRole("button", { name: /Start sync/ }));
  await screen.findByText(/Skipped this cycle/);
  fireEvent.click(screen.getByRole("button", { name: /Pause sync/ }));
  expect(api.invoke).toHaveBeenCalledWith("sync_start");
  expect(api.invoke).toHaveBeenCalledWith("sync_pause");
});

it("switches and persists all five languages while sync is running without saving sync settings", async () => {
  api.native = true;
  const saved = {
    ...defaults("en"),
    deviceName: "Test computer",
    selectedAgents: ["codex"],
  };
  api.invoke.mockImplementation(async (command, args) => {
    if (command === "bootstrap")
      return { settings: { ...saved }, agents: [], trayAvailable: true };
    if (command === "sync_status")
      return {
        running: true,
        phase: "syncing",
        published: 0,
        received: 0,
        applied: 0,
        lastSuccess: null,
        error: null,
        skipped: [],
      };
    if (command === "save_locale") saved.locale = args.locale;
  });
  const mounted = render(<App />);
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "Save setup" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true),
  );
  for (const locale of ["ja", "zh-Hant", "zh-Hans", "en", "ko"] as const) {
    const select = screen.getByLabelText(messages[saved.locale].language);
    await waitFor(() =>
      expect((select as HTMLSelectElement).disabled).toBe(false),
    );
    fireEvent.change(select, { target: { value: locale } });
    await waitFor(() => expect(document.documentElement.lang).toBe(locale));
    expect(api.invoke).toHaveBeenCalledWith("save_locale", { locale });
  }
  expect(
    api.invoke.mock.calls.some(([cmd]) =>
      ["save_settings", "sync_pause", "sync_start"].includes(cmd),
    ),
  ).toBe(false);
  mounted.unmount();
  render(<App />);
  await waitFor(() =>
    expect(
      (screen.getByLabelText(messages.ko.language) as HTMLSelectElement).value,
    ).toBe("ko"),
  );
});
it("retains current language on persistence failure", async () => {
  api.native = true;
  api.invoke.mockImplementation(async (command) => {
    if (command === "bootstrap")
      return {
        settings: { ...defaults("en"), deviceName: "Test" },
        agents: [],
        trayAvailable: true,
      };
    if (command === "save_locale") throw "save_failed";
  });
  render(<App />);
  const select = screen.getByLabelText("Language");
  await waitFor(() =>
    expect((select as HTMLSelectElement).disabled).toBe(false),
  );
  fireEvent.change(select, { target: { value: "ja" } });
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("save_locale", { locale: "ja" }),
  );
  await waitFor(() =>
    expect((select as HTMLSelectElement).disabled).toBe(false),
  );
  expect((select as HTMLSelectElement).value).toBe("en");
  expect(document.documentElement.lang).toBe("en");
});
