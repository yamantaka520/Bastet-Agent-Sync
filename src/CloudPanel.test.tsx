import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import CloudPanel, { type WizardView } from "./CloudPanel";
import type { Locale } from "./i18n";
import { wizardMessages } from "./wizard-i18n";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const view: WizardView = {
  wizard: {
    schema: 1,
    session: "fixture",
    mode: "guided",
    page: 0,
    clientId: null,
    clientSource: null,
    authorized: false,
    folderId: null,
    folderName: null,
    binding: null,
    recoverySaved: false,
    proofVerified: false,
    complete: false,
  },
  buildConfigured: true,
  connected: false,
  folders: [],
};

afterEach(() => {
  cleanup();
  invoke.mockReset();
  vi.useRealTimers();
});

it("shows Store completion guidance without Agent Memory OS in every locale", async () => {
  const completeView: WizardView = {
    ...view,
    wizard: {
      ...view.wizard,
      page: 4,
      clientId: "fixture-client",
      authorized: true,
      folderId: "fixture-folder",
      folderName: "Fixture folder",
      binding: {
        folder: "fixture-folder",
        space: "fixture-space",
        proof: "proof",
      },
      recoverySaved: true,
      proofVerified: true,
      complete: true,
    },
  };
  invoke.mockResolvedValue(completeView);
  for (const locale of Object.keys(wizardMessages) as Locale[]) {
    const t = wizardMessages[locale];
    const panel = render(<CloudPanel native locale={locale} storeChannel />);
    await act(async () => {});
    expect(screen.getByText(t.completeHintStore)).toBeTruthy();
    expect(t.completeHintStore).not.toContain("Agent Memory OS");
    panel.rerender(<CloudPanel native locale={locale} storeChannel={false} />);
    expect(screen.getByText(t.completeHint)).toBeTruthy();
    panel.unmount();
  }
});

it("recovers a busy startup read promptly without showing a stale generic error", async () => {
  vi.useFakeTimers();
  let reads = 0;
  invoke.mockImplementation((command: string) => {
    expect(command).toBe("wizard_get");
    return ++reads === 1 ? Promise.reject("sync_busy") : Promise.resolve(view);
  });
  render(<CloudPanel native locale="en" />);
  await act(async () => {});
  expect(screen.queryByRole("alert")).toBeNull();

  await act(async () => {
    await vi.advanceTimersByTimeAsync(1000);
  });
  expect(reads).toBe(2);
  expect(
    screen
      .getByRole("button", { name: wizardMessages.en.guided })
      .hasAttribute("disabled"),
  ).toBe(false);
  expect(screen.queryByRole("alert")).toBeNull();
});

it("restores a saved folder on startup retry without overwriting later edits", async () => {
  vi.useFakeTimers();
  const saved: WizardView = {
    ...view,
    wizard: {
      ...view.wizard,
      mode: "manual",
      page: 2,
      clientId: "fixture-client",
      authorized: true,
      folderId: "saved-folder",
      folderName: "Saved folder",
    },
    folders: [{ id: "remote-option", name: "Remote option" }],
  };
  let reads = 0;
  invoke.mockImplementation((command: string) => {
    expect(command).toBe("wizard_get");
    return ++reads === 1 ? Promise.reject("sync_busy") : Promise.resolve(saved);
  });
  render(<CloudPanel native locale="en" />);
  await act(async () => {});
  await act(async () => {
    await vi.advanceTimersByTimeAsync(1000);
  });
  const folderInput = screen.getByLabelText(
    wizardMessages.en.folderId,
  ) as HTMLInputElement;
  expect(folderInput.value).toBe("saved-folder");
  expect(
    screen.getByRole("button", { name: /Remote option remote-option/ }),
  ).toBeTruthy();

  fireEvent.change(folderInput, { target: { value: "unsaved-edit" } });
  await act(async () => {
    await vi.advanceTimersByTimeAsync(15000);
  });
  expect(folderInput.value).toBe("unsaved-edit");
});

it("keeps a real startup error visible after a later background read succeeds", async () => {
  vi.useFakeTimers();
  let reads = 0;
  invoke.mockImplementation((command: string) => {
    expect(command).toBe("wizard_get");
    return ++reads === 1
      ? Promise.reject("unexpected_failure")
      : Promise.resolve(view);
  });
  render(<CloudPanel native locale="en" />);
  await act(async () => {});
  expect(screen.getByRole("alert").textContent).toBe(wizardMessages.en.error);

  await act(async () => {
    await vi.advanceTimersByTimeAsync(1000);
  });
  expect(reads).toBe(2);
  expect(screen.getByRole("alert").textContent).toBe(wizardMessages.en.error);
});

it("does not clear an action error when the wizard background poll succeeds", async () => {
  vi.useFakeTimers();
  invoke.mockImplementation((command: string) =>
    command === "wizard_get"
      ? Promise.resolve(view)
      : Promise.reject("client_credentials_unavailable"),
  );
  render(<CloudPanel native locale="en" />);
  await act(async () => {});
  await act(async () => {
    fireEvent.click(
      screen.getByRole("button", { name: wizardMessages.en.useBuild }),
    );
  });
  expect(screen.getByRole("alert").textContent).toBe(
    wizardMessages.en.clientStoreError,
  );

  await act(async () => {
    await vi.advanceTimersByTimeAsync(15000);
  });
  expect(screen.getByRole("alert").textContent).toBe(
    wizardMessages.en.clientStoreError,
  );
});
