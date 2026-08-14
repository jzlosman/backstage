import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
const open = vi.fn();
const writeText = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText }));

describe("frontend backend contract", () => {
  beforeEach(() => {
    vi.resetModules();
    invoke.mockReset();
    open.mockReset();
    writeText.mockReset();
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {},
      configurable: true,
    });
  });

  it("invokes the planning-pattern commands with the backend argument names", async () => {
    invoke.mockResolvedValue({ revision: 4, patterns: [] });
    const { runtimeApi } = await import("./api");
    const patternApi = runtimeApi as unknown as {
      listPatterns(): Promise<unknown>;
      addPattern(expression: string): Promise<unknown>;
      removePattern(id: string): Promise<unknown>;
      restoreDefaultPatterns(): Promise<unknown>;
    };

    await patternApi.listPatterns();
    await patternApi.addPattern("^docs/plans/.*\\.md$");
    await patternApi.removePattern("pattern_1");
    await patternApi.restoreDefaultPatterns();

    expect(invoke.mock.calls).toEqual([
      ["list_patterns"],
      ["add_pattern", { expression: "^docs/plans/.*\\.md$" }],
      ["remove_pattern", { id: "pattern_1" }],
      ["restore_default_patterns"],
    ]);
  });

  it("returns the authoritative root-removal inventory", async () => {
    const inventory = {
      roots: [{ id: "root_2", path: "/retained" }],
      indexes: [
        {
          rootId: "root_2",
          generation: 8,
          configurationRevision: 3,
          indexedAt: "2026-08-14T12:00:00Z",
          projects: [],
          warnings: [],
        },
      ],
    };
    invoke.mockResolvedValue(inventory);
    const { runtimeApi } = await import("./api");

    await expect(runtimeApi.removeRoot("root_1")).resolves.toEqual(inventory);
    expect(invoke).toHaveBeenCalledWith("remove_root", { rootId: "root_1" });
  });
});
