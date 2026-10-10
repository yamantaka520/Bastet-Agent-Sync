import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import PortablePanel from "../src/PortablePanel";
import ReviewPanel from "../src/ReviewPanel";
import {
  PORTABLE_ERROR_CODES,
  portableErrorText,
  portableReadinessText,
  type PortableReadiness,
} from "../src/portable-errors";
import { issueText } from "../src/sync-display";
import { ops } from "../src/operations-i18n";
import { defaults } from "../src/model";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
afterEach(() => {
  cleanup();
  vi.mocked(invoke).mockReset();
});

const LOCALES = ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as const;

// The backend emits these machine codes from the bounded multipart transport
// (src-tauri/src/portable_parts.rs and portable.rs). Every one must be a
// localized, actionable instruction, never the raw code or a generic fallback.
it("localizes every multipart transport code in all five locales", () => {
  for (const locale of LOCALES) {
    for (const code of PORTABLE_ERROR_CODES) {
      const text = portableErrorText(code, locale);
      expect(text, `${code} @ ${locale}`).toBeTruthy();
      expect(text, `${code} @ ${locale}`).not.toContain(code);
    }
  }
  expect(portableErrorText("not_a_portable_code", "en")).toBeNull();
});

// Honesty contract: the static `portable_incomplete` code message never fabricates
// a progress number (the real counts live only in `readiness`, rendered
// separately), so an incomplete transfer is described as incomplete — never
// "N of M parts" from this code, and never as a generic total sync failure.
it("describes an incomplete transfer as waiting for missing parts and not ready, with no invented counts", () => {
  const text = portableErrorText("portable_incomplete", "en")!;
  expect(text).toMatch(/not complete|incomplete|missing/i);
  expect(text).toMatch(/not ready|unavailable/i);
  expect(text).toMatch(/sync/i);
  // No fabricated progress number in the code message: the counted text is the
  // backend's `readiness`, rendered apart from this code.
  expect(text).not.toMatch(/\d/);
  // Each code has a distinct, appropriate action.
  expect(portableErrorText("portable_incomplete", "en")).not.toBe(
    portableErrorText("portable_tampered", "en"),
  );
});

it("routes multipart codes through the shared source-issue display", () => {
  for (const code of PORTABLE_ERROR_CODES) {
    expect(issueText(code, "en")).toBe(portableErrorText(code, "en"));
  }
  // Never the generic "did not complete" fallback used for unknown codes.
  expect(issueText("portable_incomplete", "en")).not.toBe(
    issueText("totally_unknown_code", "en"),
  );
});

it("surfaces an actionable incomplete-transfer message when a portable compare finds missing parts", async () => {
  vi.mocked(invoke).mockRejectedValue("portable_incomplete");
  render(
    <ReviewPanel
      locale="en"
      agent="hermes"
      id="transfer"
      portable
      disabled={false}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: ops.en.compare }));
  const alert = await screen.findByRole("alert");
  expect(alert.textContent).toBe(
    portableErrorText("portable_incomplete", "en"),
  );
  expect(alert.textContent).not.toBe(ops.en.error);
});

it("marks a received item as waiting for missing parts, never ready, when its compare finds missing parts", async () => {
  const settings = {
    ...defaults("en"),
    selectedAgents: ["hermes"],
    portable: { settings: true, skills: true },
  };
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === "portable_list")
      return [{ agent: "hermes", id: "transfer", files: 503, savedAt: 1000 }];
    if (command === "portable_compare") throw "portable_incomplete";
    return undefined;
  });
  render(
    <PortablePanel
      native
      settings={settings}
      locale="en"
      value={settings.portable}
      disabled={false}
      onChange={() => {}}
    />,
  );
  fireEvent.click(screen.getByText(ops.en.incoming));
  // The only count shown is the backend's declared file count, verbatim.
  expect(await screen.findByText(/503/)).toBeTruthy();
  // Reaching the item's compare is how the UI honestly learns it is incomplete.
  fireEvent.click(await screen.findByRole("button", { name: ops.en.compare }));
  const alert = await screen.findByRole("alert");
  expect(alert.classList.contains("portable-pending")).toBe(true);
  expect(alert.textContent).toBe(
    portableErrorText("portable_incomplete", "en"),
  );
  expect(alert.textContent).not.toBe(ops.en.error);
});

// The backend's actual readiness is rendered verbatim in all five locales: the
// exact present/expected/missing counts it returned, never a raw code and never
// an invented number.
it("localizes backend readiness counts in all five locales without a raw code", () => {
  for (const locale of LOCALES) {
    const pending = portableReadinessText(
      { complete: false, legacy: false, present: 2, expected: 5, missing: [0, 1, 2], issue: null },
      locale,
    );
    expect(pending, locale).toContain("2");
    expect(pending, locale).toContain("5");
    expect(pending, locale).not.toContain("portable_");
    expect(
      portableReadinessText({ complete: true, legacy: false, present: 1, expected: 1, missing: [], issue: null }, locale),
      locale,
    ).toBeTruthy();
    expect(
      portableReadinessText({ complete: true, legacy: true, present: 1, expected: 1, missing: [], issue: null }, locale),
      locale,
    ).toBeTruthy();
    expect(
      portableReadinessText({ complete: false, legacy: false, present: 0, expected: 0, missing: [], issue: "portable_tampered" }, locale),
      locale,
    ).toBe(portableErrorText("portable_tampered", locale));
  }
});

// Honesty contract: a complete (all parts present) transfer is NEVER labelled
// "ready" or "verified" — listing only proves presence, so the badge says the
// integrity will be verified before restore, in all five locales.
it("labels a complete transfer as received-but-unverified, never ready or verified", () => {
  for (const locale of LOCALES) {
    const text = portableReadinessText(
      { complete: true, legacy: false, present: 3, expected: 3, missing: [], issue: null },
      locale,
    );
    expect(text, locale).toBeTruthy();
    expect(text, locale).not.toMatch(/\bready\b/i);
    expect(text, locale).not.toMatch(/portable_/);
  }
  // English states the exact contract: integrity verified before restore.
  expect(
    portableReadinessText(
      { complete: true, legacy: false, present: 3, expected: 3, missing: [], issue: null },
      "en",
    ),
  ).toMatch(/integrity will be verified before restore/i);
});

// A pending transfer shows the backend's exact counts and gates compare; a
// refresh that reports the transfer complete flips the badge and re-enables
// compare (whose verification then decides whether the bytes are sound).
it("shows backend pending counts, gates compare, then becomes complete on refresh", async () => {
  const settings = {
    ...defaults("en"),
    selectedAgents: ["hermes"],
    portable: { settings: true, skills: true },
  };
  const pending: PortableReadiness = {
    complete: false,
    legacy: false,
    present: 1,
    expected: 3,
    missing: [1, 2],
    issue: null,
  };
  const complete: PortableReadiness = {
    complete: true,
    legacy: false,
    present: 3,
    expected: 3,
    missing: [],
    issue: null,
  };
  let calls = 0;
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === "portable_list") {
      calls += 1;
      return [
        { agent: "hermes", id: "transfer", files: 503, savedAt: 1000, readiness: calls === 1 ? pending : complete },
      ];
    }
    return undefined;
  });
  render(
    <PortablePanel
      native
      settings={settings}
      locale="en"
      value={settings.portable}
      disabled={false}
      onChange={() => {}}
    />,
  );
  fireEvent.click(screen.getByText(ops.en.incoming));
  const badge = await screen.findByText(/1 of 3 parts received/);
  expect(badge.className).toContain("portable-pending");
  const compare = screen.getByRole("button", { name: ops.en.compare }) as HTMLButtonElement;
  expect(compare.disabled).toBe(true);

  // Refresh: the backend now reports every part present.
  fireEvent.click(screen.getByText(ops.en.incoming));
  const completeBadge = await screen.findByText(/parts received — integrity will be verified/i);
  expect(completeBadge.className).toContain("portable-complete");
  expect(
    (screen.getByRole("button", { name: ops.en.compare }) as HTMLButtonElement).disabled,
  ).toBe(false);
});

// A malformed item is surfaced with an explicit error badge and does not hide a
// valid snapshot listed beside it.
it("surfaces a malformed item as an explicit error badge beside a valid snapshot", async () => {
  const settings = {
    ...defaults("en"),
    selectedAgents: ["hermes"],
    portable: { settings: true, skills: true },
  };
  vi.mocked(invoke).mockImplementation(async (command: string) => {
    if (command === "portable_list")
      return [
        { agent: "hermes", id: "ok", files: 12, savedAt: 1000,
          readiness: { complete: true, legacy: true, present: 1, expected: 1, missing: [], issue: null } },
        { agent: "hermes", id: "broken", files: 0, savedAt: 1000,
          readiness: { complete: false, legacy: false, present: 0, expected: 0, missing: [], issue: "portable_format" } },
      ];
    return undefined;
  });
  render(
    <PortablePanel
      native
      settings={settings}
      locale="en"
      value={settings.portable}
      disabled={false}
      onChange={() => {}}
    />,
  );
  fireEvent.click(screen.getByText(ops.en.incoming));
  const error = await screen.findByText(portableErrorText("portable_format", "en")!);
  expect(error.className).toContain("portable-error");
  expect(error.getAttribute("role")).toBe("alert");
  // The valid snapshot is still listed, and still complete.
  expect(await screen.findByText(/single-package snapshot received/i)).toBeTruthy();
});
