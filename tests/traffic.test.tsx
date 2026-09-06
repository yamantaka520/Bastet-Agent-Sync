import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import TrafficStatus, { formatBytes } from "../src/TrafficStatus";
import type { Locale } from "../src/i18n";
afterEach(cleanup);
it("shows independent rates and session totals in every locale", () => {
  for (const locale of ["zh-Hant", "zh-Hans", "en", "ja", "ko"] as Locale[]) {
    const view = render(
      <TrafficStatus
        locale={locale}
        traffic={{
          uploaded: 1048576,
          downloaded: 2048,
          uploadRate: 1024,
          downloadRate: 0,
        }}
      />,
    );
    expect(screen.getByText("1 KiB/s")).toBeTruthy();
    expect(screen.getByText("0 B/s")).toBeTruthy();
    expect(view.container.textContent).toContain("1 MiB");
    expect(view.container.textContent).toContain("2 KiB");
    view.rerender(
      <TrafficStatus
        locale={locale}
        traffic={{
          uploaded: 1048576,
          downloaded: 2048,
          uploadRate: 0,
          downloadRate: 0,
        }}
      />,
    );
    expect(screen.getAllByText("0 B/s")).toHaveLength(2);
    cleanup();
  }
});
it("does not invent zero traffic before the backend returns a sample", () => {
  render(<TrafficStatus locale="en" />);
  expect(screen.getByText("Waiting for traffic data —")).toBeTruthy();
  expect(screen.queryByText("0 B/s")).toBeNull();
  expect(formatBytes(1536, "en")).toBe("1.5 KiB");
});
