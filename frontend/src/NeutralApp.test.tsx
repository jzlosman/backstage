import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import type { BackstageApi, IndexSnapshot, WorkRecord, WorkRecordDetail } from "./api";

afterEach(cleanup);

const root = { id: "root_1", path: "/Users/dev/Programming" };
const project = {
  id: "project_1",
  name: "workbench",
  rootPath: "/Users/dev/Programming/workbench",
  git: { branch: "main" },
};

function record(
  subjectId: string,
  formatId: string,
  displayName: string,
  level: WorkRecord["recognition"]["level"],
  sourcePath: string,
): WorkRecord {
  return {
    subjectId,
    locator: { projectId: project.id, formatId, adapterRecordKey: sourcePath },
    displayName,
    recognition: {
      level,
      adapterId: `${formatId}-v1`,
      adapterVersion: 1,
      evidence: ["deterministic fixture"],
    },
    sources: [{ relativePath: sourcePath, sourceModifiedUnixNanos: "10" }],
    facts:
      formatId === "openspec"
        ? [
            {
              key: "openspec.primary_status",
              label: "Status",
              value: { type: "text", value: "active" },
              provenance: { adapterId: "openspec-v1", sourcePaths: [sourcePath] },
            },
          ]
        : [],
    warnings: [],
    capabilities:
      formatId === "openspec"
        ? [
            { id: "overview", label: "Overview" },
            { id: "source", label: "Source" },
          ]
        : [{ id: "source", label: "Source" }],
    sourceModifiedUnixNanos: "10",
    fingerprint: `sha256:${subjectId}`,
  };
}

const openspec = record(
  "subject_openspec",
  "openspec",
  "ship-search",
  "recognized",
  "openspec/changes/ship-search/proposal.md",
);
const readme = record("subject_readme", "markdown", "README.md", "plain", "README.md");

const index: IndexSnapshot = {
  rootId: root.id,
  generation: 4,
  indexedAt: "today",
  configurationRevision: 0,
  warnings: [],
  projects: [
    {
      project,
      bundles: [],
      markdownDocuments: [],
      records: [openspec, readme],
      sourceCount: 2,
      registryWarnings: [],
    },
  ],
};

function withGenerationBundle(snapshot: IndexSnapshot): IndexSnapshot {
  return {
    ...snapshot,
    projects: snapshot.projects.map((indexedProject) => ({
      ...indexedProject,
      bundles: [
        {
          bundle: {
            id: "legacy_bundle_openspec",
            projectId: project.id,
            projectName: project.name,
            name: openspec.displayName,
            kind: "open_spec_change",
            recognition: { status: "recognized", detector: "openspec-v1" },
            members: [
              {
                id: "proposal",
                relativePath: openspec.sources[0]!.relativePath,
                evidence: "OpenSpec change member",
              },
            ],
          },
          progress: {
            status: "unavailable",
            progress: {
              parser: { name: "openspec-task-markers", version: "1" },
              warnings: [],
            },
          },
          fingerprint: openspec.fingerprint ?? null,
          sourceModifiedUnixNanos: openspec.sourceModifiedUnixNanos,
          warnings: [],
        },
      ],
    })),
  };
}

function generatedResult(text: string) {
  return {
    text,
    mode: "summary" as const,
    sourceFingerprint: openspec.fingerprint!,
    includedPaths: [openspec.sources[0]!.relativePath],
    generatedAt: "today",
    model: "test-model",
    promptVersion: "summary-v1",
  };
}

function detail(record: WorkRecord): WorkRecordDetail {
  const isOpenSpec = record.locator.formatId === "openspec";
  return {
    rootId: root.id,
    subjectId: record.subjectId,
    indexGeneration: index.generation,
    projectId: project.id,
    projectName: project.name,
    projectRoot: project.rootPath,
    git: project.git,
    record,
    capabilities: isOpenSpec
      ? [
          {
            capability: { id: "overview", label: "Overview" },
            blocks: [
              {
                kind: "markdown_section",
                id: "why",
                title: "Why",
                markdown: "Fresh neutral overview<script>alert(1)</script>",
                source: { relativePath: record.sources[0]!.relativePath },
              },
            ],
          },
          {
            capability: { id: "source", label: "Source" },
            blocks: [],
          },
        ]
      : [
          {
            capability: { id: "source", label: "Source" },
            blocks: [
              {
                kind: "markdown_section",
                id: "source",
                title: "README.md",
                markdown: "# Neutral README",
                source: { relativePath: "README.md" },
              },
            ],
          },
        ],
    fingerprint: record.fingerprint,
    warnings: [],
  };
}

function api(): BackstageApi {
  return {
    listRoots: vi.fn().mockResolvedValue([root]),
    chooseRoot: vi.fn().mockResolvedValue(null),
    approveRoot: vi.fn(),
    removeRoot: vi.fn(),
    listPatterns: vi.fn().mockResolvedValue({ revision: 0, patterns: [] }),
    addPattern: vi.fn(),
    removePattern: vi.fn(),
    restoreDefaultPatterns: vi.fn(),
    scanRoot: vi.fn().mockResolvedValue({
      projects: [project],
      warnings: [],
      cancelled: false,
      entriesInspected: 2,
    }),
    cancelScan: vi.fn().mockResolvedValue(false),
    getIndex: vi.fn().mockResolvedValue(index),
    getArtifactDetail: vi.fn(),
    getMarkdownDetail: vi.fn(),
    getWorkRecordDetail: vi
      .fn()
      .mockImplementation((_rootId, subjectId) =>
        Promise.resolve(detail(subjectId === openspec.subjectId ? openspec : readme)),
      ),
    getWorkRecordHandoff: vi.fn(),
    copyWorkRecordPath: vi.fn().mockResolvedValue("README.md"),
    copyWorkRecordPrompt: vi.fn().mockResolvedValue("Continue"),
    getWorkRecordAnnotation: vi.fn().mockResolvedValue({
      decision: "undecided",
      disposition: { status: "applicable" },
      favorite: false,
      todo: false,
      priority: null,
    }),
    updateWorkRecordAnnotation: vi.fn().mockResolvedValue({
      decision: "approved",
      disposition: { status: "applicable" },
      favorite: true,
      todo: false,
      priority: "high",
    }),
    getGeneratedView: vi.fn().mockResolvedValue({ status: "never_generated" }),
    requestSummary: vi.fn().mockResolvedValue({ status: "never_generated" }),
    cancelSummary: vi.fn().mockResolvedValue(false),
    copyArtifactPath: vi.fn(),
    copyMarkdownPath: vi.fn(),
    copyContinuationPrompt: vi.fn(),
    openTerminal: vi.fn(),
  };
}

describe("neutral Work Record workspace", () => {
  it("uses subject-based ledger selection and the compiled capability renderer", async () => {
    const mockApi = api();
    const user = userEvent.setup();
    const { container } = render(<App api={mockApi} />);

    await user.click(await screen.findByRole("button", { name: /ship-search/i }));

    expect(mockApi.getWorkRecordDetail).toHaveBeenCalledWith(
      root.id,
      openspec.subjectId,
      index.generation,
    );
    expect(await screen.findByText("Fresh neutral overview")).toBeInTheDocument();
    expect(container.querySelector("script")).toBeNull();
    expect(screen.queryByText("Generated Summary")).not.toBeInTheDocument();
    expect(mockApi.getGeneratedView).not.toHaveBeenCalledWith(root.id, openspec.subjectId);
    expect(mockApi.getArtifactDetail).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "All Markdown" }));
    await user.click(await screen.findByRole("button", { name: /README\.md/i }));

    expect(mockApi.getWorkRecordDetail).toHaveBeenCalledWith(
      root.id,
      readme.subjectId,
      index.generation,
    );
    expect(await screen.findByRole("heading", { name: "Neutral README" })).toBeInTheDocument();
    expect(mockApi.getMarkdownDetail).not.toHaveBeenCalled();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /README\.md/i })).toHaveAttribute(
        "aria-pressed",
        "true",
      ),
    );
  });

  it("reloads a surviving selected reader against the refreshed index generation", async () => {
    const mockApi = api();
    const user = userEvent.setup();
    let currentIndex = index;
    vi.mocked(mockApi.getIndex).mockImplementation(() => Promise.resolve(currentIndex));
    vi.mocked(mockApi.getWorkRecordDetail!).mockImplementation((_rootId, subjectId, generation) => {
      const selected = subjectId === openspec.subjectId ? openspec : readme;
      const selectedDetail = detail(selected);
      return Promise.resolve({
        ...selectedDetail,
        indexGeneration: generation,
        capabilities:
          generation === 5 && subjectId === openspec.subjectId
            ? [
                {
                  capability: { id: "overview", label: "Overview" },
                  blocks: [
                    {
                      kind: "markdown_section" as const,
                      id: "why",
                      title: "Why",
                      markdown: "Refreshed neutral overview",
                      source: { relativePath: selected.sources[0]!.relativePath },
                    },
                  ],
                },
                selectedDetail.capabilities[1]!,
              ]
            : selectedDetail.capabilities,
      });
    });
    render(<App api={mockApi} />);

    await user.click(await screen.findByRole("button", { name: /ship-search/i }));
    expect(await screen.findByText("Fresh neutral overview")).toBeInTheDocument();
    currentIndex = { ...index, generation: 5 };
    await user.click(screen.getByRole("button", { name: "Refresh approved roots" }));

    expect(await screen.findByText("Refreshed neutral overview")).toBeInTheDocument();
    expect(mockApi.getWorkRecordDetail).toHaveBeenLastCalledWith(root.id, openspec.subjectId, 5);
    expect(screen.queryByText("Fresh neutral overview")).not.toBeInTheDocument();
  });

  it("hydrates generated views under neutral subjects for stale filtering", async () => {
    const mockApi = api();
    const generatedIndex = withGenerationBundle(index);
    vi.mocked(mockApi.getIndex).mockResolvedValue(generatedIndex);
    vi.mocked(mockApi.getGeneratedView).mockResolvedValue({
      status: "stale",
      result: generatedResult("Prior summary"),
      changedInputs: [openspec.sources[0]!.relativePath],
    });
    const user = userEvent.setup();
    render(<App api={mockApi} />);

    await waitFor(() =>
      expect(mockApi.getGeneratedView).toHaveBeenCalledWith(root.id, openspec.subjectId),
    );
    await user.click(screen.getByRole("button", { name: "Stale" }));

    expect(screen.getByRole("button", { name: /ship-search/i })).toBeInTheDocument();
  });

  it("refreshes generated-summary freshness with the selected neutral reader", async () => {
    const mockApi = api();
    const user = userEvent.setup();
    let currentIndex = withGenerationBundle(index);
    let generatedStale = false;
    let staleRequests = 0;
    vi.mocked(mockApi.getIndex).mockImplementation(() => Promise.resolve(currentIndex));
    vi.mocked(mockApi.getWorkRecordDetail!).mockImplementation((_rootId, subjectId, generation) =>
      Promise.resolve({
        ...detail(subjectId === openspec.subjectId ? openspec : readme),
        indexGeneration: generation,
      }),
    );
    vi.mocked(mockApi.getGeneratedView).mockImplementation(() => {
      if (generatedStale) staleRequests += 1;
      return Promise.resolve(
        generatedStale
          ? {
              status: "stale" as const,
              result: generatedResult("Prior summary"),
              changedInputs: [openspec.sources[0]!.relativePath],
            }
          : { status: "current" as const, result: generatedResult("Current summary") },
      );
    });
    render(<App api={mockApi} />);

    await user.click(await screen.findByRole("button", { name: /ship-search/i }));
    expect(await screen.findByText("Current summary")).toBeInTheDocument();
    currentIndex = { ...currentIndex, generation: 5 };
    generatedStale = true;
    await user.click(screen.getByRole("button", { name: "Refresh approved roots" }));

    await waitFor(() => expect(staleRequests).toBeGreaterThan(0));
    expect(await screen.findByText(/source fingerprint changed/i)).toBeInTheDocument();
    expect(screen.getByText("Prior summary")).toBeInTheDocument();
  });

  it("does not let delayed refresh detail overwrite an authoritative annotation", async () => {
    const mockApi = api();
    const user = userEvent.setup();
    let detailCalls = 0;
    let resolveRefresh!: (value: WorkRecordDetail) => void;
    const delayedRefresh = new Promise<WorkRecordDetail>((resolve) => {
      resolveRefresh = resolve;
    });
    vi.mocked(mockApi.getWorkRecordDetail!).mockImplementation((_rootId, subjectId) => {
      detailCalls += 1;
      const selected = subjectId === openspec.subjectId ? openspec : readme;
      return detailCalls === 2 ? delayedRefresh : Promise.resolve(detail(selected));
    });
    render(<App api={mockApi} />);

    await user.click(await screen.findByRole("button", { name: /ship-search/i }));
    await user.click(screen.getByRole("button", { name: "Refresh approved roots" }));
    await waitFor(() => expect(mockApi.getWorkRecordDetail).toHaveBeenCalledTimes(2));
    await user.selectOptions(screen.getByLabelText("Decision"), "approved");
    expect(screen.getByLabelText("Decision")).toHaveValue("approved");
    resolveRefresh(detail(openspec));

    await waitFor(() => expect(screen.getByLabelText("Decision")).toHaveValue("approved"));
  });

  it("applies authoritative annotation responses to badges and cross-format filters", async () => {
    const mockApi = api();
    const user = userEvent.setup();
    render(<App api={mockApi} />);

    await user.click(await screen.findByRole("button", { name: /ship-search/i }));
    await user.selectOptions(screen.getByLabelText("Decision"), "approved");
    await waitFor(() =>
      expect(mockApi.updateWorkRecordAnnotation).toHaveBeenCalledWith(openspec.subjectId, {
        command: "set_decision",
        value: "approved",
      }),
    );
    expect(
      within(screen.getByRole("button", { name: /ship-search/i })).getByText("High priority"),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "All Markdown" }));
    await user.selectOptions(screen.getByLabelText("Filter by private annotation"), "favorite");

    expect(screen.getByRole("button", { name: /ship-search/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /README\.md/i })).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Filter by private annotation"), "applicable");
    expect(screen.getByRole("button", { name: /ship-search/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /README\.md/i })).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Filter by private annotation"), "undecided");
    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /README\.md/i })).toBeInTheDocument();
  });
});
