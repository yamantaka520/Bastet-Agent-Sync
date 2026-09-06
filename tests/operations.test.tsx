import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import OperationsPanel, {
  ResourceControls,
  SourceProgress,
} from "../src/OperationsPanel";
import PortablePanel from "../src/PortablePanel";
import ReviewPanel from "../src/ReviewPanel";
import { grokCommand } from "../src/Continuation";
import { ops } from "../src/operations-i18n";
import { defaults } from "../src/model";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});
it("has complete five-language controls and translates in place", () => {
  const keys = Object.keys(ops.en).sort();
  for (const t of Object.values(ops)) {
    expect(Object.keys(t).sort()).toEqual(keys);
    expect(Object.values(t).every(Boolean)).toBe(true);
  }
  const change = vi.fn();
  const { rerender } = render(
    <ResourceControls locale="en" disabled={false} onChange={change} />,
  );
  fireEvent.change(screen.getByLabelText(ops.en.parallel), {
    target: { value: "6" },
  });
  expect(change).toHaveBeenLastCalledWith(
    expect.objectContaining({ parallel: 6 }),
  );
  fireEvent.change(screen.getByLabelText(ops.en.from), {
    target: { value: "22:30" },
  });
  expect(change).toHaveBeenLastCalledWith(
    expect.objectContaining({ startMinute: 1350 }),
  );
  rerender(<ResourceControls locale="zh-Hant" disabled onChange={change} />);
  expect(screen.getByText(ops["zh-Hant"].resources)).toBeTruthy();
  expect(screen.getByRole("group").hasAttribute("disabled")).toBe(true);
});
it("shows source payload progress without inventing a whole-cycle percentage", () => {
  render(
    <SourceProgress
      locale="en"
      progress={{
        stage: "upload",
        completed: 2,
        total: 4,
        bytesDone: 65536,
        bytesTotal: 131072,
        etaSeconds: 2,
      }}
    />,
  );
  expect(screen.getByText(/Current HTTP payload/)).toBeTruthy();
  expect(screen.getByText(/≈ 2s/)).toBeTruthy();
});
it("previews draft settings and allows individual files to be excluded", async () => {
  const settings = {
    ...defaults("en"),
    selectedAgents: ["codex"],
    portable: { settings: true, skills: true },
  };
  vi.mocked(invoke).mockResolvedValue([
    { agent: "codex", files: { "skills/cat/SKILL.md": "# cat" }, excluded: {} },
  ]);
  const change = vi.fn();
  render(
    <PortablePanel
      native
      settings={settings}
      locale="en"
      value={settings.portable}
      disabled={false}
      onChange={change}
    />,
  );
  fireEvent.click(screen.getByText(ops.en.preview));
  const file = await screen.findByLabelText("skills/cat/SKILL.md");
  expect(invoke).toHaveBeenCalledWith("portable_preview", { settings });
  fireEvent.click(file);
  expect(change).toHaveBeenCalledWith(
    expect.objectContaining({
      excludedPaths: { codex: ["skills/cat/SKILL.md"] },
    }),
  );
});
it("keeps conflict originals and passes the reviewed fingerprint and chosen local profile", async () => {
  const comparison = {
    files: [
      {
        path: "sessions/test.jsonl",
        state: "different",
        localBytes: 3,
        incomingBytes: 3,
        localHash: "a",
        incomingHash: "b",
        localText: "old",
        incomingText: "new",
        truncated: false,
      },
    ],
    fingerprint: "fingerprint",
    reviewed: false,
  };
  vi.mocked(invoke).mockResolvedValue(comparison);
  const restore = vi.fn();
  render(
    <ReviewPanel
      locale="en"
      agent="codex"
      id="version"
      disabled={false}
      onRestore={restore}
    />,
  );
  fireEvent.change(screen.getByRole("combobox"), {
    target: { value: "chatgpt-work" },
  });
  fireEvent.click(screen.getByRole("button", { name: ops.en.compare }));
  await screen.findByText("old");
  await screen.findByText("new");
  fireEvent.click(screen.getByText(ops.en.acknowledge));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("review_received_session", {
      agent: "codex",
      id: "version",
      fingerprint: "fingerprint",
      sourceAgent: "chatgpt-work",
    }),
  );
  await screen.findByText(ops.en.reviewed);
  fireEvent.click(screen.getByText(ops.en.keepBoth));
  expect(restore).toHaveBeenCalledOnce();
});
it("does not run native maintenance in browser preview", () => {
  render(<OperationsPanel native={false} locale="en" runtime={null} />);
  expect(invoke).not.toHaveBeenCalled();
  for (const b of screen.getAllByRole("button"))
    expect((b as HTMLButtonElement).disabled).toBe(true);
});
it("quotes continuation paths for both shells and rejects injected session ids", () => {
  expect(grokCommand("/tmp/cat's folder", "session-1", false)).toContain(
    `'/tmp/cat'"'"'s folder'`,
  );
  expect(grokCommand("C:\\cat's folder", "session-1", true)).toContain(
    "'C:\\cat''s folder'",
  );
  expect(grokCommand("/tmp", "id;rm", false)).toBeNull();
  expect(grokCommand("/tmp\ncommand", "id", false)).toBeNull();
});
it("shows persisted device/history data and safely pauses a running worker", async () => {
  vi.mocked(invoke).mockImplementation(async (command) =>
    command === "operations_view"
      ? {
          history: [
            { started: 100, finished: 105, outcome: "complete", sources: [] },
          ],
          devices: [
            {
              id: "device",
              name: "Test cat",
              os: "linux",
              version: "0.5.0",
              reportedAt: 100,
              observedAt: 110,
              outcome: "complete",
              agents: ["grok"],
            },
          ],
        }
      : undefined,
  );
  render(
    <OperationsPanel
      native
      locale="en"
      runtime={{
        running: true,
        phase: "waiting",
        published: 0,
        received: 0,
        applied: 0,
        lastSuccess: null,
        error: null,
        skipped: [],
      }}
    />,
  );
  await screen.findByText("Test cat");
  expect(screen.getByText(ops.en.devices)).toBeTruthy();
  expect((screen.getByText(ops.en.clean) as HTMLButtonElement).disabled).toBe(
    true,
  );
  fireEvent.click(screen.getByText(ops.en.pause));
  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith("sync_pause_for", { seconds: 900 }),
  );
});
