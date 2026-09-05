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
    command === "cloud_status"
      ? Promise.resolve({ configured: false, connected: false })
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
    expect(screen.getByText(messages.ja.setup)).toBeTruthy();
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
        ["bootstrap", "run_sync_diagnostic"].includes(name),
      ),
    ).toBe(true);
    expect(
      (screen.getByRole("button", { name: /Start sync/ }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });
  it("clears previous success before a failed retry", async () => {
    api.native = true;
    let runs = 0;
    api.invoke.mockImplementation(async (command: string) => {
      if (command === "bootstrap")
        return { settings: defaults("en"), agents: [], trayAvailable: false };
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
