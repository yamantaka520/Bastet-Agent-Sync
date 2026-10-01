import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { resumeCommand } from "./Continuation";
import ProjectMappings, { mappingText } from "./ProjectMappings";
import NativeSessions from "./NativeSessions";
import { projectError } from "./project-errors";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
afterEach(() => {
  cleanup();
  invoke.mockReset();
});

it("builds a provider-specific resume command with a scoped cwd and profile", () => {
  for (const [agent, expected] of [
    ["codex", "CODEX_HOME='/profiles/new' codex 'resume' 'session-1'"],
    [
      "claude-code",
      "CLAUDE_CONFIG_DIR='/profiles/new' claude '--resume' 'session-1'",
    ],
    ["pi", "PI_CODING_AGENT_DIR='/profiles/new' pi '--session' 'session-1'"],
    ["grok", "GROK_HOME='/profiles/new' grok '--resume' 'session-1'"],
  ])
    expect(
      resumeCommand(
        agent,
        "/profiles/new",
        "session-1",
        "/projects/local",
        false,
      ),
    ).toBe(`(cd -- '/projects/local' && ${expected})`);
  expect(
    resumeCommand("agy", "/profile", "session", "/project", false),
  ).toBeNull();
  expect(resumeCommand("codex", "/profile", "session", "", false)).toBeNull();
});

it("quotes shell metacharacters and rejects multi-line or invalid session input", () => {
  const posix = resumeCommand(
    "pi",
    "/a'$(touch sentinel)",
    "session-1",
    "/project`name`",
    false,
  )!;
  expect(posix).toContain(`PI_CODING_AGENT_DIR='/a'"'"'$(touch sentinel)'`);
  expect(posix).toContain("cd -- '/project`name`'");
  const ps = resumeCommand(
    "codex",
    "C:\\a'b$env:SECRET",
    "session-1",
    "D:\\[project]",
    true,
  )!;
  expect(ps).toContain(
    "Set-Location -LiteralPath 'D:\\[project]' -ErrorAction Stop",
  );
  expect(ps).toContain("$env:CODEX_HOME = 'C:\\a''b$env:SECRET'");
  expect(ps).toContain("finally {");
  for (const bad of ["bad\npath", "bad\rpath", "bad\0path"]) {
    expect(
      resumeCommand("codex", bad, "session", "/project", false),
    ).toBeNull();
    expect(resumeCommand("codex", "/profile", "session", bad, true)).toBeNull();
  }
  expect(
    resumeCommand("codex", "/profile", "';whoami", "/project", false),
  ).toBeNull();
});

it("keeps source mapping when choosing a local directory and preserves it on cancellation", async () => {
  const change = vi.fn();
  invoke.mockResolvedValueOnce("/local/project").mockResolvedValueOnce(null);
  render(
    <ProjectMappings
      locale="en"
      disabled={false}
      value={[{ source: "C:\\Project", target: "" }]}
      onChange={change}
    />,
  );
  fireEvent.click(screen.getByText("Choose local project"));
  await waitFor(() =>
    expect(change).toHaveBeenCalledWith([
      { source: "C:\\Project", target: "/local/project" },
    ]),
  );
  change.mockClear();
  fireEvent.click(screen.getByText("Choose local project"));
  await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  expect(change).not.toHaveBeenCalled();
});

it("shows branches and resumes a prepared profile without restoring it again", async () => {
  invoke.mockResolvedValueOnce([
    {
      agent: "codex",
      id: "head",
      session: "session-1",
      cwd: "C:\\Project",
      mappedCwd: "/local/project",
      managedProfile: "/managed/profile",
      generation: 2,
      branchCount: 2,
      parentIds: ["base"],
      originDevice: "fixture-device",
    },
  ]);
  render(<NativeSessions native locale="en" running={false} />);
  fireEvent.click(screen.getByText("View conversation snapshots"));
  fireEvent.click(
    await screen.findByRole("button", { name: /Codex · 1 snapshots/ }),
  );
  expect(screen.getByText("Version: 3 · Branches: 2")).toBeTruthy();
  fireEvent.click(screen.getByText("Continue restored version"));
  expect(
    screen.getByText(/CODEX_HOME='\/managed\/profile' codex/),
  ).toBeTruthy();
  expect(invoke).toHaveBeenCalledTimes(1);
});

it("provides mapping guidance in every supported locale", () => {
  for (const locale of ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const) {
    expect(mappingText[locale].length).toBe(mappingText.en.length);
    for (const code of [
      "project_mapping_required",
      "project_mapping_target_missing",
      "project_mapping_conflict",
      "project_mapping_format_unsupported",
      "handoff_profile_missing",
    ])
      expect(projectError(code, locale)).toBeTruthy();
  }
});
