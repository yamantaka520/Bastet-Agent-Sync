import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import MemoryPanel, { memoryMessages } from "../src/MemoryPanel";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});
it("keeps five locales and browser preview isolated", () => {
  for (const t of Object.values(memoryMessages)) expect(t.length).toBe(8);
  render(<MemoryPanel native={false} locale="en" />);
  expect((screen.getByRole("button") as HTMLButtonElement).disabled).toBe(true);
  expect(invoke).not.toHaveBeenCalled();
});
it("shows only inspection metadata and clears it after cancellation or failure", async () => {
  vi.mocked(invoke)
    .mockResolvedValueOnce({
      version: 3,
      records: 7,
      kinds: { memory: 2 },
      containsAuthorityChanges: true,
    })
    .mockResolvedValueOnce(null)
    .mockRejectedValueOnce("private diagnostic must not render");
  render(<MemoryPanel native locale="en" />);
  fireEvent.click(screen.getByRole("button"));
  expect(await screen.findByText(memoryMessages.en[4])).toBeTruthy();
  fireEvent.click(screen.getByRole("button"));
  await screen.findByRole("button");
  // Wait for cancellation to finish before issuing a fresh native action.
  const { waitFor } = await import("@testing-library/react");
  await waitFor(() => expect(screen.queryByRole("status")).toBeNull());
  await waitFor(() =>
    expect((screen.getByRole("button") as HTMLButtonElement).disabled).toBe(
      false,
    ),
  );
  fireEvent.click(screen.getByRole("button"));
  expect(await screen.findByRole("alert")).toHaveProperty(
    "textContent",
    memoryMessages.en[5],
  );
  expect(screen.queryByText("private diagnostic must not render")).toBeNull();
});
