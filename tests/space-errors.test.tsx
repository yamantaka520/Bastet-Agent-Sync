import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import CloudPanel from "../src/CloudPanel";
import NativeSessions from "../src/NativeSessions";
import OperationsPanel from "../src/OperationsPanel";
import WorkerStatus, {
  workerError,
  type SyncStatus,
} from "../src/WorkerStatus";
import { issueText } from "../src/sync-display";
import { spaceErrorText } from "../src/space-errors";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const status: SyncStatus = {
  running: false,
  phase: "partial",
  published: 0,
  received: 0,
  applied: 0,
  lastSuccess: null,
  error: null,
  skipped: [],
};

afterEach(() => {
  cleanup();
  invoke.mockReset();
});

it("localizes legacy and precise encrypted-space errors without platform jargon", () => {
  const errors = [
    "wrong_space_or_version",
    "foreign_space",
    "unsupported_encryption_version",
    "encrypted_space_mismatch",
    "folder_has_sync_objects",
    "foreign_space_objects",
  ];
  for (const locale of ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const) {
    for (const error of errors) {
      const text = spaceErrorText(error, locale);
      expect(text).toBeTruthy();
      expect(text).not.toContain("Mac");
      expect(text).not.toContain("Windows");
    }
  }
  expect(spaceErrorText("not_a_space_error", "en")).toBeNull();
});

it("shows legacy and new errors in source and global status displays", () => {
  expect(issueText("wrong_space_or_version", "en")).toContain(
    "exact cause is unknown",
  );
  expect(issueText("foreign_space_objects", "en")).toContain(
    "Own-space objects can continue",
  );
  expect(workerError("foreign_space", "en")).toContain("different space");

  render(
    <NativeSessions
      native={false}
      locale="en"
      running={false}
      sources={[
        {
          agent: "codex",
          state: "partial",
          captured: 0,
          available: 0,
          published: 0,
          received: 0,
          restored: 0,
          issues: { foreign_space_objects: 1 },
        },
      ]}
    />,
  );
  expect(screen.getByText(/Own-space objects can continue/)).toBeTruthy();
  cleanup();

  render(
    <WorkerStatus
      native={false}
      locale="en"
      status={{ ...status, error: "foreign_space" }}
      onStatus={() => {}}
    />,
  );
  expect(
    screen.getByText(/Verify the recovery kit from the original device/),
  ).toBeTruthy();
});

it("shows the legacy recovery action in the wizard and observer warning", async () => {
  invoke.mockRejectedValueOnce("wrong_space_or_version");
  render(<CloudPanel native locale="en" />);
  expect(await screen.findByText(/exact cause is unknown/)).toBeTruthy();
  cleanup();

  render(
    <OperationsPanel
      native={false}
      locale="en"
      runtime={{ ...status, observerError: "foreign_space_objects" }}
    />,
  );
  expect(
    screen.getByText(/Objects from another encrypted space were skipped/),
  ).toBeTruthy();
});

it("keeps recovery, compatibility, and inconsistent-object actions distinct", () => {
  expect(spaceErrorText("foreign_space", "en")).toContain("only if");
  expect(spaceErrorText("unsupported_encryption_version", "en")).toContain(
    "not guaranteed",
  );
  expect(spaceErrorText("encrypted_space_mismatch", "en")).toContain(
    "Stop applying this object",
  );
  expect(spaceErrorText("folder_has_sync_objects", "en")).toContain(
    "choose an empty folder",
  );
});
