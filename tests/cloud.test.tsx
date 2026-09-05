import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import CloudPanel, { cloudMessages } from "../src/CloudPanel";
import { invoke } from "@tauri-apps/api/core";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
afterEach(cleanup);
beforeEach(() => vi.mocked(invoke).mockReset());
it("keeps all five cloud locale dictionaries aligned", () => {
  for (const messages of Object.values(cloudMessages))
    expect(Object.keys(messages).sort()).toEqual(
      Object.keys(cloudMessages.en).sort(),
    );
});
it("does not access native commands in browser preview", () => {
  render(<CloudPanel native={false} locale="en" />);
  expect(
    (screen.getByText(cloudMessages.en.connect) as HTMLButtonElement).disabled,
  ).toBe(true);
  expect(invoke).not.toHaveBeenCalled();
});
it("keeps login unavailable without client and clears a failed diagnostic retry", async () => {
  vi.mocked(invoke).mockImplementation(async (command) =>
    command === "cloud_status"
      ? { configured: false, connected: false }
      : { verified: true, recoveryVerified: true, tamperRejected: true },
  );
  render(<CloudPanel native locale="en" />);
  expect(
    (screen.getByText(cloudMessages.en.connect) as HTMLButtonElement).disabled,
  ).toBe(true);
  fireEvent.click(screen.getByText(cloudMessages.en.check));
  await screen.findByText(cloudMessages.en.passed);
  expect(invoke).toHaveBeenCalledWith("run_crypto_diagnostic");
  vi.mocked(invoke).mockRejectedValueOnce("crypto_check_failed");
  fireEvent.click(screen.getByText(cloudMessages.en.check));
  await screen.findByRole("alert");
  expect(screen.queryByText(cloudMessages.en.passed)).toBeNull();
});
it("requires an explicit click before connecting or creating a cloud folder", async () => {
  vi.mocked(invoke).mockImplementation(async (command) =>
    command === "cloud_status"
      ? { configured: true, connected: false }
      : command === "connect_google"
        ? []
        : { id: "new-id", name: "Bastet Agent Sync" },
  );
  render(<CloudPanel native locale="en" />);
  await waitFor(() =>
    expect(
      (screen.getByText(cloudMessages.en.connect) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
  expect(invoke).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByText(cloudMessages.en.connect));
  await screen.findByText(cloudMessages.en.empty);
  expect(invoke).not.toHaveBeenCalledWith(
    "create_google_folder",
    expect.anything(),
  );
  fireEvent.click(screen.getByText(cloudMessages.en.create));
  await screen.findByText("(new-id)");
  expect(invoke).toHaveBeenCalledWith("create_google_folder", {
    name: "Bastet Agent Sync",
  });
});
