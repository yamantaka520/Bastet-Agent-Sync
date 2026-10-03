import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import FolderPanel, { type FolderStatus } from "./FolderPanel";
import { folderMessages } from "./folder-i18n";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
afterEach(() => {
  cleanup();
  invoke.mockReset();
});

const picked: FolderStatus = {
  provider: "icloud-drive",
  path: "/fixture/iCloud Drive/Bastet Agent Sync",
  space: null,
  complete: false,
  handoff: "local-folder",
};
const ready: FolderStatus = {
  ...picked,
  space: "fixture-space",
  complete: true,
};

it("keeps a cancelled recovery save pending and completes only from a real result", async () => {
  const onChange = vi.fn();
  let prepareCount = 0;
  let current: FolderStatus = { ...picked, path: null };
  invoke.mockImplementation(
    async (command: string, args: { provider: string }) => {
      expect(args.provider).toBe("icloud-drive");
      if (command === "folder_status") return current;
      if (command === "folder_pick") return (current = picked);
      if (command === "folder_prepare")
        return ++prepareCount === 1 ? null : (current = ready);
      if (command === "folder_export_recovery") return true;
      throw new Error(command);
    },
  );
  const t = folderMessages.en;
  render(
    <FolderPanel
      native
      locale="en"
      provider="icloud-drive"
      onChange={onChange}
    />,
  );
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("folder_status", {
      provider: "icloud-drive",
    }),
  );
  fireEvent.click(screen.getByRole("button", { name: t.pick }));
  await screen.findByText(picked.path!);
  fireEvent.click(screen.getByRole("button", { name: t.prepare }));
  await waitFor(() => expect(prepareCount).toBe(1));
  expect(screen.getByText(t.pending)).toBeTruthy();
  expect(onChange).not.toHaveBeenCalledWith(ready);
  fireEvent.click(screen.getByRole("button", { name: t.prepare }));
  await screen.findByText(t.complete);
  expect(onChange).toHaveBeenCalledWith(ready);
  fireEvent.click(screen.getByRole("button", { name: t.export }));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("folder_export_recovery", {
      provider: "icloud-drive",
    }),
  );
  expect(screen.getByText(t.handoff)).toBeTruthy();
});

it("joins an existing OneDrive space and never calls native commands in preview", async () => {
  const oneDrivePicked: FolderStatus = {
    ...picked,
    provider: "onedrive-folder",
  };
  let current = oneDrivePicked;
  invoke.mockImplementation(
    async (command: string, args: { provider: string }) => {
      expect(args.provider).toBe("onedrive-folder");
      if (command === "folder_join")
        current = { ...oneDrivePicked, space: "joined", complete: true };
      return current;
    },
  );
  const t = folderMessages.en;
  const preview = render(
    <FolderPanel native={false} locale="en" provider="onedrive-folder" />,
  );
  expect(
    (screen.getByRole("button", { name: t.pick }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  expect(invoke).not.toHaveBeenCalled();
  preview.unmount();
  render(<FolderPanel native locale="en" provider="onedrive-folder" />);
  await screen.findByText(picked.path!);
  fireEvent.click(screen.getByRole("button", { name: t.join }));
  await screen.findByText("joined");
  expect(screen.getByText(t.complete)).toBeTruthy();
});

it("has complete folder guidance in all five locales", () => {
  for (const t of Object.values(folderMessages)) {
    expect(Object.keys(t).sort()).toEqual(
      Object.keys(folderMessages.en).sort(),
    );
    expect(Object.values(t).every(Boolean)).toBe(true);
  }
});

it("locks setup while syncing and explains a pending iCloud placeholder", async () => {
  invoke.mockImplementation(async (command: string) => {
    if (command === "folder_status") return picked;
    if (command === "folder_join") throw "folder_pending";
    throw new Error(command);
  });
  const t = folderMessages.en;
  const panel = render(
    <FolderPanel native locale="en" provider="icloud-drive" disabled />,
  );
  await screen.findByText(picked.path!);
  expect(
    (screen.getByRole("button", { name: t.join }) as HTMLButtonElement)
      .disabled,
  ).toBe(true);
  panel.rerender(<FolderPanel native locale="en" provider="icloud-drive" />);
  fireEvent.click(screen.getByRole("button", { name: t.join }));
  await screen.findByRole("alert");
  expect(screen.getByRole("alert").textContent).toBe(t.pendingDownload);
  expect(screen.getByText(picked.path!)).toBeTruthy();
});

it("clears stale readiness when a saved proof disappears during refresh", async () => {
  const onChange = vi.fn();
  let statusReads = 0;
  invoke.mockImplementation(async (command: string) => {
    if (command === "folder_status") {
      if (++statusReads === 1) return ready;
      throw "folder_object_missing";
    }
    if (command === "folder_export_recovery") return true;
    throw new Error(command);
  });
  const t = folderMessages.en;
  render(
    <FolderPanel
      native
      locale="en"
      provider="icloud-drive"
      onChange={onChange}
    />,
  );
  await screen.findByText(t.complete);
  fireEvent.click(screen.getByRole("button", { name: t.export }));
  await screen.findByRole("alert");
  expect(screen.getByRole("alert").textContent).toBe(t.unavailable);
  expect(screen.queryByText(t.complete)).toBeNull();
  expect(screen.getByText(t.pending)).toBeTruthy();
  expect(onChange).toHaveBeenLastCalledWith(null);
});
