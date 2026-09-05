import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import CloudPanel, { type WizardView } from "../src/CloudPanel";
import { wizardMessages } from "../src/wizard-i18n";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const t = wizardMessages.en;
const initial = (): WizardView => ({
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
});
afterEach(cleanup);
beforeEach(() => vi.mocked(invoke).mockReset());
it("keeps five dictionaries complete and avoids native calls in browser preview", () => {
  for (const locale of Object.values(wizardMessages)) {
    expect(Object.keys(locale).sort()).toEqual(Object.keys(t).sort());
    expect(locale.steps).toHaveLength(5);
  }
  render(<CloudPanel native={false} locale="en" />);
  expect((screen.getByText(t.diagnostic) as HTMLButtonElement).disabled).toBe(
    true,
  );
  expect(invoke).not.toHaveBeenCalled();
});
it("restores the saved step, switches manual mode without losing progress and confirms restart", async () => {
  let saved = initial();
  Object.assign(saved.wizard, {
    clientId: "fixture.apps.googleusercontent.com",
    clientSource: "imported",
    authorized: true,
    page: 2,
  });
  vi.mocked(invoke).mockImplementation(async (cmd, args) => {
    if (cmd === "wizard_navigate") Object.assign(saved.wizard, args);
    if (cmd === "wizard_restart") saved = initial();
    return structuredClone(saved);
  });
  const mounted = render(<CloudPanel native locale="en" />);
  await screen.findByRole("heading", { name: "3. Sync folder" });
  fireEvent.click(screen.getByText(t.manual));
  await screen.findByRole("heading", { name: "5. Review & finish" });
  expect(invoke).toHaveBeenCalledWith("wizard_navigate", {
    mode: "manual",
    page: 2,
  });
  mounted.unmount();
  render(<CloudPanel native locale="en" />);
  await screen.findByRole("heading", { name: "5. Review & finish" });
  fireEvent.click(screen.getByText(t.restart));
  expect(invoke).not.toHaveBeenCalledWith("wizard_restart");
  fireEvent.click(screen.getByText(t.confirmRestart));
  await screen.findByRole("heading", { name: "1. Login configuration" });
  expect(saved.wizard.clientId).toBeNull();
});
it("completes setup only after real command results and preserves progress on cancellation/failure", async () => {
  const saved = initial();
  let cancelExport = true;
  let failPublish = true;
  vi.mocked(invoke).mockImplementation(async (cmd, args) => {
    const action = (args as { action?: string })?.action;
    if (cmd === "wizard_execute") {
      const w = saved.wizard;
      if (action === "use_build")
        Object.assign(w, {
          clientId: "fixture.apps.googleusercontent.com",
          clientSource: "build",
          page: 1,
        });
      if (action === "connect") Object.assign(w, { authorized: true, page: 2 });
      if (action === "create_folder")
        Object.assign(w, {
          folderId: "folder",
          folderName: "Bastet Agent Sync",
          page: 3,
        });
      if (action === "prepare_key")
        w.binding = { folder: "folder", space: "space", proof: "proof" };
      if (action === "export_recovery") {
        if (cancelExport) cancelExport = false;
        else w.recoverySaved = true;
      }
      if (action === "publish_proof") {
        if (failPublish) {
          failPublish = false;
          throw "network_unavailable";
        }
        Object.assign(w, { proofVerified: true, page: 4 });
      }
      if (action === "finish") w.complete = true;
    }
    return structuredClone(saved);
  });
  render(<CloudPanel native locale="en" />);
  fireEvent.click(await screen.findByText(t.useBuild));
  fireEvent.click(await screen.findByText(t.connect));
  fireEvent.click(await screen.findByText(t.createFolder));
  fireEvent.click(await screen.findByText(t.prepareKey));
  await waitFor(() =>
    expect(
      (screen.getByText(t.exportRecovery) as HTMLButtonElement).disabled,
    ).toBe(false),
  );
  expect((screen.getByText(t.publishProof) as HTMLButtonElement).disabled).toBe(
    true,
  );
  fireEvent.click(screen.getByText(t.exportRecovery));
  await waitFor(() =>
    expect(
      (screen.getByText(t.exportRecovery) as HTMLButtonElement).disabled,
    ).toBe(false),
  );
  expect(saved.wizard.recoverySaved).toBe(false);
  fireEvent.click(screen.getByText(t.exportRecovery));
  await waitFor(() =>
    expect(
      (screen.getByText(t.publishProof) as HTMLButtonElement).disabled,
    ).toBe(false),
  );
  fireEvent.click(screen.getByText(t.publishProof));
  await screen.findByRole("alert");
  expect(saved.wizard.proofVerified).toBe(false);
  expect(saved.wizard.recoverySaved).toBe(true);
  fireEvent.click(screen.getByText(t.publishProof));
  fireEvent.click(await screen.findByText(t.finish));
  await screen.findByText(`✓ ${t.complete}`);
  expect(invoke).toHaveBeenCalledWith("wizard_execute", {
    action: "finish",
    input: "",
    locale: "en",
  });
  expect(screen.getByText(t.completeHint)).toBeTruthy();
});
it("manual settings use explicit verified actions and cannot bypass missing prerequisites", async () => {
  const saved = initial();
  saved.wizard.mode = "manual";
  saved.buildConfigured = false;
  vi.mocked(invoke).mockResolvedValue(saved);
  render(<CloudPanel native locale="en" />);
  await screen.findByRole("heading", { name: "5. Review & finish" });
  expect((screen.getByText(t.useBuild) as HTMLButtonElement).disabled).toBe(
    true,
  );
  expect((screen.getByText(t.finish) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByText(t.createFolder) as HTMLButtonElement).disabled).toBe(
    true,
  );
  fireEvent.change(screen.getByLabelText(t.folderId), {
    target: { value: "manual-folder" },
  });
  expect(invoke).toHaveBeenCalledTimes(1);
});
it("keeps corrupt progress visible as an error and permits explicit restart", async () => {
  vi.mocked(invoke)
    .mockRejectedValueOnce("wizard_corrupt")
    .mockResolvedValue(initial());
  render(<CloudPanel native locale="en" />);
  await screen.findByRole("alert");
  fireEvent.click(screen.getByText(t.restart));
  fireEvent.click(screen.getByText(t.confirmRestart));
  await screen.findByRole("heading", { name: "1. Login configuration" });
});
it("clears stale diagnostic success when retry fails", async () => {
  vi.mocked(invoke).mockImplementation(async (cmd) =>
    cmd === "wizard_get"
      ? initial()
      : { verified: true, recoveryVerified: true, tamperRejected: true },
  );
  render(<CloudPanel native locale="en" />);
  fireEvent.click(screen.getByText(t.diagnostic));
  await screen.findByText(t.diagnosticPassed);
  vi.mocked(invoke).mockRejectedValueOnce("crypto_check_failed");
  fireEvent.click(screen.getByText(t.diagnostic));
  await screen.findByRole("alert");
  expect(screen.queryByText(t.diagnosticPassed)).toBeNull();
});
