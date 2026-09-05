import {
  render,
  screen,
  fireEvent,
  waitFor,
  cleanup,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import NativeSessions, { sessionMessages } from "./NativeSessions";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
afterEach(() => {
  cleanup();
  invoke.mockReset();
});
it("distinguishes empty, partial, syncing and complete sources", () => {
  render(
    <NativeSessions
      native
      locale="en"
      running
      sources={["empty", "partial", "syncing", "complete"].map((state, i) => ({
        agent: ["pi", "codex", "agy", "grok"][i],
        state,
        captured: 0,
        available: 0,
        published: 0,
        received: 0,
        restored: 0,
        issues: (state === "partial" ? { session_conflict: 1 } : {}) as Record<
          string,
          number
        >,
      }))}
    />,
  );
  expect(screen.getByText(/No local conversations/)).toBeTruthy();
  expect(screen.getByText(/Partially complete/)).toBeTruthy();
  expect(screen.getByText(/Google Agy CLI —/).textContent).toContain("…");
  expect(
    (screen.getByText("View conversation snapshots") as HTMLButtonElement)
      .disabled,
  ).toBe(true);
});
it("restores only the selected snapshot through the native folder picker", async () => {
  invoke
    .mockResolvedValueOnce([
      {
        agent: "pi",
        id: "a".repeat(64),
        session: "session-1",
        cwd: "/project",
      },
    ])
    .mockResolvedValueOnce("/restore/profile");
  render(<NativeSessions native locale="en" running={false} />);
  fireEvent.click(screen.getByText("View conversation snapshots"));
  await screen.findByText("Restore to a new folder");
  fireEvent.click(screen.getByText("Restore to a new folder"));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("restore_received_session", {
      agent: "pi",
      id: "a".repeat(64),
    }),
  );
  expect(
    await screen.findByText("/restore/profile", { selector: "code" }),
  ).toBeTruthy();
});
it("keeps all five locale dictionaries complete", () => {
  for (const messages of Object.values(sessionMessages)) {
    expect(messages.length).toBe(sessionMessages.en.length);
    expect(messages.every((s) => s.length > 0)).toBe(true);
  }
});
