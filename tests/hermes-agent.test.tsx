import { afterEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import App from "../src/App";
import NativeSessions, { sessionMessages } from "../src/NativeSessions";
import { resumeCommand } from "../src/Continuation";
import { names, defaults } from "../src/model";
import {
  ops,
  continuationIssueText,
  continuationFactText,
  continuationIssue,
  continuationFacts,
} from "../src/operations-i18n";
import { messages } from "../src/i18n";

const api = vi.hoisted(() => ({ native: false, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  isTauri: () => api.native,
  invoke: (command: string, ...args: unknown[]) =>
    command === "operations_view"
      ? Promise.resolve({ history: [], devices: [] })
      : command === "wizard_get"
        ? Promise.resolve(null)
        : api.invoke(command, ...args),
}));
afterEach(() => {
  cleanup();
  api.native = false;
  api.invoke.mockReset();
});

it("names the Hermes Agent source with its backend id", () => {
  expect(names.hermes).toBe("Hermes Agent");
});

it("localizes the exact verified fact codes emitted by the backend", () => {
  for (const locale of ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const) {
    for (const fact of ["workspace", "attachments", "project", "memory"]) {
      expect(Object.keys(continuationFacts[locale])).toContain(fact);
      expect(continuationFactText(fact, locale)).not.toContain(ops[locale].contUnknown);
    }
  }
});

it("localizes recipient credential and application prerequisites", () => {
  for (const locale of ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const) {
    for (const code of ["workspace_dependency_credential", "workspace_dependency_app"]) {
      expect(Object.keys(continuationIssue[locale])).toContain(code);
      expect(continuationIssueText(code, locale)).not.toContain(code);
    }
  }
});

it("builds a Hermes resume command scoped to HERMES_HOME and its project", () => {
  expect(
    resumeCommand("hermes", "/profiles/new", "session-1", "/projects/local", false),
  ).toBe(
    "(cd -- '/projects/local' && HERMES_HOME='/profiles/new' hermes '--resume' 'session-1')",
  );
  const ps = resumeCommand("hermes", "C:\\hermes home", "session-1", "D:\\p", true)!;
  expect(ps).toContain("$env:HERMES_HOME = 'C:\\hermes home'");
  expect(ps).toContain("'--resume' 'session-1'");
  expect(resumeCommand("hermes", "/profile", "bad\nid", "/project", false)).toBeNull();
  expect(resumeCommand("hermes", "/profile", "id;rm", "/project", false)).toBeNull();
});

it("describes Hermes Agent in all five locales without claiming cross-device acceptance", () => {
  for (const locale of ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const) {
    expect(sessionMessages[locale][1]).toContain("Hermes");
    expect(sessionMessages[locale][1]).toContain("state.db");
    expect(ops[locale].hermesRecovery.length).toBeGreaterThan(0);
    expect(ops[locale].hermesRecovery).toContain("HERMES_HOME");
  }
  expect(ops.en.hermesRecovery).toContain("state.db");
  // Honest status: the frontend must not assert acceptance it cannot prove.
  expect(ops.en.hermesRecovery).toMatch(/not claimed/i);
  expect(sessionMessages.en[1]).toMatch(/not claimed/i);
});

it("restores a Hermes snapshot into an isolated home and offers its resume command", async () => {
  api.invoke.mockImplementation(async (command: string) => {
    if (command === "list_received_sessions")
      return [
        {
          agent: "hermes",
          id: "a".repeat(64),
          session: "20260101_000000_abcd",
          cwd: "/project",
          mappedCwd: "/local/project",
          managedProfile: "/managed/hermes",
        },
      ];
    return undefined;
  });
  render(<NativeSessions native locale="en" running={false} />);
  fireEvent.click(screen.getByText(sessionMessages.en[2]));
  fireEvent.click(
    await screen.findByRole("button", { name: /Hermes Agent · 1 snapshots/ }),
  );
  fireEvent.click(await screen.findByText("Continue restored version"));
  expect(screen.getByText(ops.en.hermesRecovery)).toBeTruthy();
  expect(document.body.textContent).toContain("HERMES_HOME = /managed/hermes");
  expect(document.body.textContent).toContain(
    "HERMES_HOME='/managed/hermes' hermes '--resume' '20260101_000000_abcd'",
  );
});

it("keeps Hermes Agent selectable when discovery predates the backend entry", async () => {
  api.native = true;
  const s = { ...defaults("en"), deviceName: "Fallback computer" };
  api.invoke.mockImplementation(async (command: string) =>
    command === "bootstrap"
      ? {
          settings: s,
          agents: [
            { id: "codex", path: "/codex", detected: true, custom: false },
          ],
          trayAvailable: true,
        }
      : undefined,
  );
  render(<App />);
  const checkbox = (await screen.findByRole("checkbox", {
    name: "Hermes Agent",
  })) as HTMLInputElement;
  expect(checkbox.disabled).toBe(false);
  fireEvent.click(checkbox);
  fireEvent.click(screen.getByRole("button", { name: messages.en.save }));
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("save_settings", {
      settings: { ...s, selectedAgents: ["hermes"] },
    }),
  );
});

it("keeps Hermes selectable in Store discovery while hiding Agent Memory OS", async () => {
  api.native = true;
  api.invoke.mockImplementation(async (command: string) =>
    command === "bootstrap"
      ? {
          settings: { ...defaults("en"), deviceName: "Store" },
          agents: [
            { id: "codex", path: "/codex", detected: false, custom: false },
            { id: "hermes", path: "/selected-hermes", detected: false, custom: true },
            { id: "agent-memory-os", path: "/memory", detected: false, custom: false },
          ],
          memorySyncAvailable: false,
          trayAvailable: true,
        }
      : undefined,
  );
  render(<App />);
  await screen.findByText("/codex");
  expect(screen.getByRole("checkbox", { name: "Hermes Agent" })).toBeTruthy();
  expect(screen.queryByRole("checkbox", { name: "Agent Memory OS" })).toBeNull();
});

const HERMES_ID = "h".repeat(64);

async function openHermesSnapshot(item: Record<string, unknown>) {
  api.invoke.mockImplementation(async (command: string) =>
    command === "list_received_sessions" ? [item] : undefined,
  );
  render(<NativeSessions native locale="en" running={false} />);
  fireEvent.click(screen.getByText(sessionMessages.en[2]));
  fireEvent.click(
    await screen.findByRole("button", { name: /Hermes Agent · 1 snapshots/ }),
  );
}

it("blocks the continuation action on a blocked report and keeps isolated restore available", async () => {
  await openHermesSnapshot({
    agent: "hermes",
    id: HERMES_ID,
    session: "20260101_000000_block",
    cwd: "/source/project",
    mappedCwd: "/local/project",
    managedProfile: "/managed/hermes",
    continuation: {
      state: "blocked",
      blockers: ["project_mapping_required", "memory_identity_missing"],
      warnings: ["workspace_limit"],
      verified: [],
    },
  });
  expect(screen.getByText(`⛔ ${ops.en.contBlocked}`)).toBeTruthy();
  fireEvent.click(screen.getByText("Continue restored version"));
  const text = document.body.textContent ?? "";
  expect(text).toContain(`${ops.en.contReadiness}: ${ops.en.contBlocked}`);
  // Actionable, exact setup guidance — never a blanket "unsupported".
  expect(text).toContain(continuationIssueText("project_mapping_required", "en"));
  expect(text).toContain(continuationIssueText("memory_identity_missing", "en"));
  expect(text).toContain(continuationIssueText("workspace_limit", "en"));
  expect(text).toContain(ops.en.contBlockedAction);
  // Blocked must not present a ready or copy action.
  expect(text).not.toContain(ops.en.contReady);
  expect(screen.queryByText(ops.en.copyCommand)).toBeNull();
  // Safe isolated restore stays available when dependencies are missing.
  const restore = screen.getByText(sessionMessages.en[3]) as HTMLButtonElement;
  expect(restore.disabled).toBe(false);
});

it("treats a Hermes version with no readiness report as unverified, not ready", async () => {
  await openHermesSnapshot({
    agent: "hermes",
    id: HERMES_ID,
    session: "20260101_000000_legacy",
    cwd: "/source/project",
    mappedCwd: "/local/project",
    managedProfile: "/managed/hermes",
  });
  expect(screen.getByText(`⚠️ ${ops.en.contUnverified}`)).toBeTruthy();
  fireEvent.click(screen.getByText("Continue restored version"));
  const text = document.body.textContent ?? "";
  expect(text).toContain(`${ops.en.contReadiness}: ${ops.en.contUnverified}`);
  expect(text).toContain(ops.en.contLegacy);
  expect(text).not.toContain(ops.en.contReady);
  expect(text).toContain(ops.en.contUnverifiedNote);
  // Unverified warns but still offers the command for reference.
  expect(text).toContain("HERMES_HOME='/managed/hermes'");
});

it("keeps a verified continuation ready and shows its verified facts", async () => {
  await openHermesSnapshot({
    agent: "hermes",
    id: HERMES_ID,
    session: "20260101_000000_ready",
    cwd: "/source/project",
    mappedCwd: "/local/project",
    managedProfile: "/managed/hermes",
    continuation: {
      state: "ready",
      blockers: [],
      warnings: [],
      verified: ["session_store_isolated", "project_working_tree_restored"],
    },
  });
  fireEvent.click(screen.getByText("Continue restored version"));
  const text = document.body.textContent ?? "";
  expect(text).toContain(`${ops.en.contReadiness}: ${ops.en.contReady}`);
  expect(text).toContain(continuationFactText("session_store_isolated", "en"));
  expect(screen.getByText(ops.en.copyCommand)).toBeTruthy();
});

it("uses the restored isolated project path as the resume workdir with safe quoting", async () => {
  await openHermesSnapshot({
    agent: "hermes",
    id: HERMES_ID,
    session: "20260101_000000_ready",
    cwd: "/source/project",
    mappedCwd: "/local/project",
    managedProfile: "/managed/hermes",
    continuation: {
      state: "ready",
      blockers: [],
      warnings: [],
      verified: [],
      projectPath: "/restored/iso'proj",
    },
  });
  fireEvent.click(screen.getByText("Continue restored version"));
  const text = document.body.textContent ?? "";
  expect(text).toContain(`${ops.en.contProject}: /restored/iso'proj`);
  // The isolated restored path — not the receiver's mapped worktree — is used.
  expect(text).toContain(`cd -- '/restored/iso'"'"'proj'`);
  expect(text).not.toContain(`cd -- '/local/project'`);
});

it("leaves non-Hermes adapters unchanged with no continuation readiness UI", async () => {
  api.invoke.mockImplementation(async (command: string) =>
    command === "list_received_sessions"
      ? [
          {
            agent: "pi",
            id: "p".repeat(64),
            session: "session-pi",
            cwd: "/source/pi",
            mappedCwd: "/local/pi",
            managedProfile: "/managed/pi",
          },
        ]
      : undefined,
  );
  render(<NativeSessions native locale="en" running={false} />);
  fireEvent.click(screen.getByText(sessionMessages.en[2]));
  fireEvent.click(
    await screen.findByRole("button", { name: /Pi Agent · 1 snapshots/ }),
  );
  expect(document.body.textContent).not.toContain(ops.en.contReadiness);
  fireEvent.click(screen.getByText("Continue restored version"));
  const text = document.body.textContent ?? "";
  expect(text).not.toContain(ops.en.contReadiness);
  expect(text).not.toContain(ops.en.contReadyNote);
  expect(text).not.toContain(ops.en.contUnverifiedNote);
  expect(text).toContain("PI_CODING_AGENT_DIR='/managed/pi'");
  expect(text).toContain("cd -- '/local/pi'");
});

it("keeps readiness guidance and UI keys complete across all five locales", () => {
  const locales = ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const;
  const issueKeys = Object.keys(continuationIssue.en).sort();
  const factKeys = Object.keys(continuationFacts.en).sort();
  expect(issueKeys.length).toBeGreaterThan(0);
  expect(factKeys.length).toBeGreaterThan(0);
  const uiKeys = [
    "contReadiness",
    "contReady",
    "contUnverified",
    "contBlocked",
    "contBlockers",
    "contWarnings",
    "contVerified",
    "contProject",
    "contLegacy",
    "contBlockedAction",
    "contUnverifiedNote",
    "contReadyNote",
    "contUnknown",
    "recheck",
    "recheckHint",
    "setupProvider",
    "setupProviderHint",
    "acknowledgeRequirements",
    "acknowledgeHint",
    "contAcknowledged",
    "contResolved",
  ] as const;
  for (const locale of locales) {
    expect(Object.keys(continuationIssue[locale]).sort()).toEqual(issueKeys);
    expect(Object.values(continuationIssue[locale]).every(Boolean)).toBe(true);
    expect(Object.keys(continuationFacts[locale]).sort()).toEqual(factKeys);
    expect(Object.values(continuationFacts[locale]).every(Boolean)).toBe(true);
    for (const key of uiKeys) expect(ops[locale][key].length).toBeGreaterThan(0);
  }
  // The named required-setup paths always resolve to real guidance.
  for (const code of [
    "project_mapping_required",
    "project_missing_files",
    "model_credentials_required",
    "memory_identity_missing",
    "mcp_binding_required",
  ])
    for (const locale of locales)
      expect(continuationIssueText(code, locale)).toBeTruthy();
  // An unknown code stays visible instead of being hidden or dropped.
  expect(continuationIssueText("totally_unknown_code", "en")).toContain(
    "totally_unknown_code",
  );
  expect(continuationFactText("totally_unknown_fact", "ja")).toContain(
    "totally_unknown_fact",
  );
});

// Every machine code the native backend can emit into a Hermes continuation
// report (`Continuation.blockers` / `.warnings`). Sources: hermes_project,
// hermes_memory and hermes_workspace issue/dependency aggregates routed through
// `native_sessions::continuation_for` (workspace_{reason}, workspace_dependency_
// {kind}, attachment_{reason}) plus its own project/memory readiness codes.
const CONTINUATION_CODES = [
  // Project working-tree continuity (hermes_project + project_mapping).
  "project_mapping_required",
  "project_mapping_target_missing",
  "project_mapping_invalid",
  "project_mapping_conflict",
  "project_mapping_format_unsupported",
  "project_no_commit",
  "project_git_unavailable",
  "project_submodule",
  "project_selected_empty",
  "project_limit",
  "project_missing_files",
  "project_missing_lfs",
  "project_baseline_required",
  "project_non_utf8_path",
  "project_hash_mismatch",
  "project_unreadable",
  "project_invalid",
  "project_absent",
  // External memory (hermes_memory + the continuation memory namespace).
  "memory_provider_unknown",
  "memory_provider_unresolved",
  "memory_identity_missing",
  "memory_export_unauthorized",
  "memory_cli_missing",
  "memory_cli_failed",
  "memory_store_missing",
  "memory_unavailable_store",
  "memory_provider_not_installed",
  "memory_store_diverged",
  "memory_export_failed",
  "memory_export_missing",
  "memory_invalid",
  "memory_unverified",
  // Portable workspace aggregate: {reason} issues and {kind} dependencies.
  "workspace_format",
  "workspace_limit",
  "workspace_unsafe",
  "workspace_invalid",
  "workspace_missing",
  "workspace_file_count_limit",
  "workspace_oversized",
  "workspace_total_limit",
  "workspace_suspected_credential",
  "workspace_depth_limit",
  "workspace_symlink",
  "workspace_excluded_directory",
  "workspace_unsupported_or_sensitive_path",
  "workspace_unsafe_store",
  "workspace_missing_required_skill_asset",
  "workspace_unparseable_config",
  "workspace_non_text",
  "workspace_dependency_mcp",
  "workspace_dependency_model",
  "workspace_dependency_memory",
  "workspace_dependency_os",
  "workspace_dependency_env",
  "workspace_dependency_runtime",
  // Attachment namespace aggregated by continuation_for.
  "attachment_missing",
  "attachment_symlink",
  "attachment_credential",
  "attachment_outside_roots",
  "attachment_oversized",
  "attachment_limit",
  "attachment_unreadable",
  "attachment_unsupported",
  // Recipient-bound setup that is never transferred.
  "hermes_parent_missing",
  "model_credentials_required",
  "model_provider_unknown",
  "mcp_binding_required",
  "required_environment_variables",
] as const;

it("localizes every emitted Hermes continuation code without falling back", () => {
  const locales = ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const;
  for (const locale of locales) {
    for (const code of CONTINUATION_CODES) {
      const text = continuationIssueText(code, locale);
      const fallback = `${ops[locale].contUnknown}: ${code}`;
      // A localized, actionable instruction — never the raw-code fallback.
      expect(text, `${code} @ ${locale}`).toBeTruthy();
      expect(text, `${code} @ ${locale} fell back`).not.toBe(fallback);
      expect(
        text.startsWith(ops[locale].contUnknown),
        `${code} @ ${locale}`,
      ).toBe(false);
      // No sender absolute source path may leak into guidance.
      expect(
        /\/(?:Users|home|root|Volumes)\//.test(text),
        `${code} @ ${locale} leaked a POSIX path`,
      ).toBe(false);
      expect(
        /[A-Za-z]:[\\/]/.test(text),
        `${code} @ ${locale} leaked a Windows path`,
      ).toBe(false);
    }
  }
});

it("gives the required new continuation codes concrete, non-overwriting guidance", () => {
  const locales = ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const;
  const required = [
    "workspace_dependency_model",
    "workspace_dependency_memory",
    "workspace_missing_required_skill_asset",
    "memory_provider_not_installed",
    "memory_store_diverged",
    "project_mapping_required",
  ] as const;
  for (const code of required)
    for (const locale of locales)
      expect(
        continuationIssueText(code, locale),
        `${code} @ ${locale}`,
      ).not.toBe(`${ops[locale].contUnknown}: ${code}`);
  // A provider plugin that was NOT received is installed only with the
  // recipient's explicit approval — never presented as already ready.
  const plugin = continuationIssueText("memory_provider_not_installed", "en");
  expect(plugin).toMatch(/not received|plugin/i);
  expect(plugin).toMatch(/approve|approval|approved/i);
  // A diverged store is preserved and never overwritten; recapture a version.
  const diverged = continuationIssueText("memory_store_diverged", "en");
  expect(diverged).toMatch(/not overwritten|preserv/i);
  expect(diverged).toMatch(/recapture|fresh version/i);
});

// ── Explicit recheck / provider setup / acknowledgment UI ──────────────────

const RECHECK_UI_KEYS = [
  "recheck",
  "recheckHint",
  "setupProvider",
  "setupProviderHint",
  "acknowledgeRequirements",
  "acknowledgeHint",
] as const;

it("localizes the recheck, provider setup and acknowledgment actions in all five locales", () => {
  for (const locale of ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const) {
    for (const key of RECHECK_UI_KEYS) {
      expect(ops[locale][key].length).toBeGreaterThan(0);
    }
    // The acknowledgment is explicitly an acknowledgment, not a validation.
    expect(ops[locale].acknowledgeHint.length).toBeGreaterThan(0);
  }
  // The setup guidance names the trusted local operation and its no-download,
  // no-received-code guarantees.
  expect(ops.en.setupProviderHint).toContain("agent-memory hermes install");
  expect(ops.en.setupProviderHint).toMatch(/never runs received code/i);
  expect(ops.en.acknowledgeHint).toMatch(/acknowledgment, not a validation/i);
});

async function openRestoredHermesSnapshot(item: Record<string, unknown>) {
  await openHermesSnapshot(item);
  fireEvent.click(await screen.findByText("Continue restored version"));
}

it("rechecks the registered home and replaces the shown report with the backend's", async () => {
  api.invoke.mockImplementation(async (command: string) => {
    if (command === "list_received_sessions")
      return [
        {
          agent: "hermes",
          id: HERMES_ID,
          session: "20260101_000000_recheck",
          cwd: "/source/project",
          mappedCwd: "/local/project",
          managedProfile: "/managed/hermes",
          continuation: {
            state: "blocked",
            blockers: ["memory_provider_not_installed"],
            warnings: [],
            verified: [],
          },
        },
      ];
    if (command === "recheck_continuation")
      return {
        continuation: {
          state: "ready",
          blockers: [],
          warnings: [],
          verified: ["memory"],
        },
        memory: { ready: true, imported: true },
        acknowledged: [],
        providerInstalled: true,
      };
    return undefined;
  });
  render(<NativeSessions native locale="en" running={false} />);
  fireEvent.click(screen.getByText(sessionMessages.en[2]));
  fireEvent.click(
    await screen.findByRole("button", { name: /Hermes Agent · 1 snapshots/ }),
  );
  fireEvent.click(await screen.findByText("Continue restored version"));
  expect(document.body.textContent).toContain(ops.en.contBlocked);

  fireEvent.click(screen.getByText(ops.en.recheck));
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("recheck_continuation", {
      agent: "hermes",
      id: HERMES_ID,
      acknowledgeRequirements: false,
    }),
  );
  // The refreshed, authoritative report replaces the stale blocked one.
  await waitFor(() =>
    expect(document.body.textContent).toContain(
      `${ops.en.contReadiness}: ${ops.en.contReady}`,
    ),
  );
  expect(ops.en.recheckHint.length).toBeGreaterThan(0);
});

it("sets up the trusted local memory provider through the registered-home command", async () => {
  await openRestoredHermesSnapshot({
    agent: "hermes",
    id: HERMES_ID,
    session: "20260101_000000_setup",
    cwd: "/source/project",
    managedProfile: "/managed/hermes",
    continuation: {
      state: "blocked",
      blockers: ["memory_provider_not_installed"],
      warnings: [],
      verified: [],
    },
  });
  api.invoke.mockImplementation(async (command: string) =>
    command === "setup_memory_provider"
      ? {
          continuation: {
            state: "ready",
            blockers: [],
            warnings: [],
            verified: ["memory"],
          },
          providerInstalled: true,
        }
      : undefined,
  );
  fireEvent.click(screen.getByText(ops.en.setupProvider));
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("setup_memory_provider", {
      agent: "hermes",
      id: HERMES_ID,
    }),
  );
});

it("records an explicit, hash-bound requirement acknowledgment through recheck", async () => {
  await openRestoredHermesSnapshot({
    agent: "hermes",
    id: HERMES_ID,
    session: "20260101_000000_ack",
    cwd: "/source/project",
    managedProfile: "/managed/hermes",
    continuation: {
      state: "blocked",
      blockers: ["workspace_dependency_mcp"],
      warnings: [],
      verified: [],
    },
  });
  api.invoke.mockImplementation(async (command: string) =>
    command === "recheck_continuation"
      ? {
          continuation: {
            state: "unverified",
            blockers: [],
            warnings: ["workspace_dependency_mcp_acknowledged"],
            verified: [],
          },
          acknowledged: ["workspace_dependency_mcp_acknowledged"],
        }
      : undefined,
  );
  fireEvent.click(screen.getByText(ops.en.acknowledgeRequirements));
  await waitFor(() =>
    expect(api.invoke).toHaveBeenCalledWith("recheck_continuation", {
      agent: "hermes",
      id: HERMES_ID,
      acknowledgeRequirements: true,
    }),
  );
  // An acknowledgment keeps the version unverified, never ready.
  await waitFor(() =>
    expect(document.body.textContent).toContain(
      `${ops.en.contReadiness}: ${ops.en.contUnverified}`,
    ),
  );
  // The acknowledged requirement is shown as an unverified acknowledgment, not
  // as verified and not as a raw fallback code.
  expect(document.body.textContent).toContain(ops.en.contAcknowledged);
  expect(ops.en.acknowledgeHint.length).toBeGreaterThan(0);
});

it("hides the provider setup action in a build without the external channel", async () => {
  api.invoke.mockImplementation(async (command: string) =>
    command === "list_received_sessions"
      ? [
          {
            agent: "hermes",
            id: HERMES_ID,
            session: "20260101_000000_store",
            cwd: "/source/project",
            managedProfile: "/managed/hermes",
            continuation: {
              state: "blocked",
              blockers: ["memory_unavailable_store"],
              warnings: [],
              verified: [],
            },
          },
        ]
      : undefined,
  );
  render(<NativeSessions native locale="en" running={false} storeChannel />);
  fireEvent.click(screen.getByText(sessionMessages.en[2]));
  fireEvent.click(
    await screen.findByRole("button", { name: /Hermes Agent · 1 snapshots/ }),
  );
  fireEvent.click(await screen.findByText("Continue restored version"));
  expect(screen.queryByText(ops.en.setupProvider)).toBeNull();
  // The non-mutating recheck stays available.
  expect(screen.getByText(ops.en.recheck)).toBeTruthy();
});
