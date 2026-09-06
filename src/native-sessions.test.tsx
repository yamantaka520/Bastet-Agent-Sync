import {
  render,
  screen,
  fireEvent,
  waitFor,
  cleanup,
  within,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { syncDisplay, savedTime } from "./sync-display";
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
  expect(screen.getByText(/No local conversations to sync/)).toBeTruthy();
  expect(screen.getByText(/Some items need attention/)).toBeTruthy();
  expect(screen.getByText(/Syncing, please wait/)).toBeTruthy();
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
  fireEvent.click(
    await screen.findByRole("button", { name: /Pi Agent · 1 snapshots/ }),
  );
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

it("groups and collapses snapshots, sorts newest first and handles old timestamps", async () => {
  invoke.mockResolvedValue([
    { agent: "pi", id: "a", session: "old", cwd: "/old", localSavedAt: 1000 },
    { agent: "codex", id: "c", session: "legacy", cwd: "/legacy" },
    { agent: "pi", id: "b", session: "new", cwd: "/new", localSavedAt: 2000 },
  ]);
  render(<NativeSessions native locale="en" running={false} />);
  fireEvent.click(screen.getByText("View conversation snapshots"));
  const toggle = await screen.findByRole("button", {
    name: /Pi Agent · 2 snapshots/,
  });
  expect(toggle.getAttribute("aria-expanded")).toBe("false");
  expect(screen.queryByText("Restore to a new folder")).toBeNull();
  fireEvent.click(toggle);
  const group = toggle.closest("section")!;
  expect(within(group).getAllByRole("listitem")[0].textContent).toContain(
    "new",
  );
  expect(within(group).getAllByRole("listitem")[0].textContent).toContain(
    savedTime(2000, "en"),
  );
  fireEvent.click(screen.getByText("Expand all"));
  expect(screen.getByText(/Time unavailable/)).toBeTruthy();
  fireEvent.click(screen.getByText("Collapse all"));
  expect(screen.queryByText("Restore to a new folder")).toBeNull();
  for (const values of Object.values(syncDisplay))
    expect(values.length).toBe(syncDisplay.en.length);
  expect(savedTime(Number.NaN, "en")).toBe("Time unavailable");
});
it("keeps unknown and paused states honest and explains errors before technical codes", () => {
  render(
    <NativeSessions
      native
      locale="en"
      running={false}
      sources={[
        {
          agent: "pi",
          state: "syncing",
          captured: 0,
          available: 0,
          published: 0,
          received: 0,
          restored: 0,
          issues: {},
        },
        {
          agent: "codex",
          state: "unrecognized",
          captured: 0,
          available: 0,
          published: 0,
          received: 0,
          restored: 0,
          issues: { bundle_limit: 1 },
        },
      ]}
    />,
  );
  expect(screen.getByText(/Paused/)).toBeTruthy();
  expect(screen.getByText(/Waiting to sync/)).toBeTruthy();
  expect(screen.getByText(/Data exceeds the current limit/)).toBeTruthy();
  expect(screen.queryByText(/This cycle is complete/)).toBeNull();
});
