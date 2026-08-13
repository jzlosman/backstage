import { describe, expect, it, vi } from "vitest";

import { DEFAULT_LAYOUT, loadPaneLayout, normalizePaneLayout, savePaneLayout } from "./layout";

describe("pane layout persistence", () => {
  it("clamps unsafe stored widths while preserving collapse state", () => {
    const storage = {
      getItem: vi
        .fn()
        .mockReturnValue(
          JSON.stringify({ projectWidth: 9999, ledgerWidth: 10, ledgerCollapsed: true }),
        ),
    };

    expect(loadPaneLayout(storage)).toEqual({
      projectWidth: 360,
      ledgerWidth: 280,
      ledgerCollapsed: true,
    });
  });

  it("falls back when app-owned preference data is malformed", () => {
    expect(loadPaneLayout({ getItem: () => "not-json" })).toEqual(DEFAULT_LAYOUT);
  });

  it("persists normalized widths", () => {
    const setItem = vi.fn();

    savePaneLayout(
      normalizePaneLayout({ projectWidth: 100, ledgerWidth: 900, ledgerCollapsed: false }),
      { setItem },
    );

    expect(setItem).toHaveBeenCalledWith(
      "backstage.pane-layout.v1",
      JSON.stringify({ projectWidth: 190, ledgerWidth: 560, ledgerCollapsed: false }),
    );
  });
});
