import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import type { BackstageApi, IndexedBundle } from "./api";

afterEach(cleanup);

describe("ledger scale", () => {
  it("renders 20 projects and 200 bundles within the v1 interaction budget", async () => {
    const projects = Array.from({ length: 20 }, (_, index) => ({
      id: `project_${index}`,
      name: `project-${index}`,
      rootPath: `/fixture/project-${index}`,
      git: { branch: "main" },
    }));
    const bundles: IndexedBundle[] = Array.from({ length: 200 }, (_, index) => ({
      bundle: {
        id: `bundle_${index}`,
        projectId: projects[index % projects.length]!.id,
        projectName: projects[index % projects.length]!.name,
        name: `change-${String(index).padStart(3, "0")}`,
        kind: "open_spec_change",
        recognition: { status: "recognized", detector: "openspec-v1" },
        members: [
          {
            id: `artifact_${index}`,
            relativePath: `openspec/changes/change-${index}/tasks.md`,
            evidence: "OpenSpec",
          },
        ],
      },
      progress: {
        status: "available",
        progress: {
          total: 4,
          completed: 2,
          remainingCount: 2,
          tasks: [],
          remaining: [],
          parser: { name: "openspec-task-markers", version: "1" },
          warnings: [],
        },
      },
      fingerprint: `sha256:${index}`,
      sourceModifiedUnixNanos: index,
      warnings: [],
    }));
    const api: BackstageApi = {
      listRoots: vi.fn().mockResolvedValue([{ id: "root_1", path: "/fixture" }]),
      chooseRoot: vi.fn(),
      approveRoot: vi.fn(),
      removeRoot: vi.fn(),
      scanRoot: vi.fn().mockResolvedValue({
        projects,
        warnings: [],
        cancelled: false,
        entriesInspected: 1000,
      }),
      cancelScan: vi.fn().mockResolvedValue(false),
      getIndex: vi.fn().mockResolvedValue({
        rootId: "root_1",
        generation: 1,
        indexedAt: "now",
        warnings: [],
        projects: projects.map((project) => ({
          project,
          bundles: bundles.filter((bundle) => bundle.bundle.projectId === project.id),
          markdownDocuments: [],
        })),
      }),
      getArtifactDetail: vi.fn(),
      getMarkdownDetail: vi.fn(),
      getGeneratedView: vi.fn(),
      requestSummary: vi.fn(),
      cancelSummary: vi.fn().mockResolvedValue(false),
      copyArtifactPath: vi.fn(),
      copyMarkdownPath: vi.fn(),
      copyContinuationPrompt: vi.fn(),
      openTerminal: vi.fn(),
    };
    const started = performance.now();

    render(<App api={api} />);
    expect(await screen.findByText("200 records")).toBeVisible();
    const elapsed = performance.now() - started;

    expect(screen.getAllByRole("button", { name: /change-/ })).toHaveLength(200);
    expect(elapsed).toBeLessThan(1_500);
  });

  it("bounds mounted Markdown rows while keeping the complete set searchable", async () => {
    const root = { id: "root_1", path: "/fixture" };
    const project = {
      id: "project_1",
      name: "docs",
      rootPath: "/fixture/docs",
      git: { branch: "main" },
    };
    const markdownDocuments = Array.from({ length: 1_000 }, (_, index) => ({
      id: `document_${index}`,
      projectId: project.id,
      projectName: project.name,
      relativePath: `notes/${String(index).padStart(4, "0")}.md`,
      sourceModifiedUnixNanos: index,
    }));
    const api: BackstageApi = {
      listRoots: vi.fn().mockResolvedValue([root]),
      chooseRoot: vi.fn(),
      approveRoot: vi.fn(),
      removeRoot: vi.fn(),
      scanRoot: vi.fn().mockResolvedValue({
        projects: [project],
        warnings: [],
        cancelled: false,
        entriesInspected: 1_000,
      }),
      cancelScan: vi.fn().mockResolvedValue(false),
      getIndex: vi.fn().mockResolvedValue({
        rootId: root.id,
        generation: 1,
        indexedAt: "now",
        warnings: [],
        projects: [{ project, bundles: [], markdownDocuments }],
      }),
      getArtifactDetail: vi.fn(),
      getMarkdownDetail: vi.fn(),
      getGeneratedView: vi.fn(),
      requestSummary: vi.fn(),
      cancelSummary: vi.fn().mockResolvedValue(false),
      copyArtifactPath: vi.fn(),
      copyMarkdownPath: vi.fn(),
      copyContinuationPrompt: vi.fn(),
      openTerminal: vi.fn(),
    };
    render(<App api={api} />);
    fireEvent.click(await screen.findByRole("button", { name: "All Markdown" }));
    const ledger = screen.getByLabelText("Bundle ledger");

    expect(within(ledger).getByText("1000 records")).toBeVisible();
    expect(within(ledger).getByText("1000 files")).toBeVisible();
    expect(within(ledger).getAllByText("Markdown document")).toHaveLength(200);
    fireEvent.click(within(ledger).getByRole("button", { name: "Show 200 more records" }));
    expect(within(ledger).getAllByText("Markdown document")).toHaveLength(400);

    fireEvent.change(screen.getByRole("searchbox", { name: "Search all indexed work" }), {
      target: { value: "0999.md" },
    });

    expect(within(ledger).getByRole("button", { name: /0999\.md/i })).toBeVisible();
    expect(within(ledger).queryByRole("button", { name: /0000\.md/i })).not.toBeInTheDocument();
  });
});
