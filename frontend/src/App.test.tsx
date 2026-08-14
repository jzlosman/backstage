import "@testing-library/jest-dom/vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import type {
  ArtifactDetail,
  BackstageApi,
  GeneratedView,
  IndexSnapshot,
  IndexedBundle,
  MarkdownDetail,
  OpenSpecView,
  Project,
  TaskFact,
} from "./api";
import * as markdown from "./markdown";

afterEach(() => {
  cleanup();
  localStorage.clear();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((next, fail) => {
    resolve = next;
    reject = fail;
  });
  return { promise, resolve, reject };
}

function api(overrides: Partial<BackstageApi> = {}): BackstageApi {
  return {
    listRoots: vi.fn().mockResolvedValue([]),
    chooseRoot: vi.fn().mockResolvedValue(null),
    approveRoot: vi.fn(),
    removeRoot: vi.fn(),
    listPatterns: vi.fn().mockResolvedValue({ revision: 0, patterns: [] }),
    addPattern: vi.fn(),
    removePattern: vi.fn(),
    restoreDefaultPatterns: vi.fn(),
    scanRoot: vi.fn(),
    cancelScan: vi.fn().mockResolvedValue(false),
    getIndex: vi.fn().mockResolvedValue(null),
    getArtifactDetail: vi.fn(),
    getMarkdownDetail: vi.fn(),
    getGeneratedView: vi.fn().mockResolvedValue({ status: "never_generated" }),
    requestSummary: vi.fn(),
    cancelSummary: vi.fn().mockResolvedValue(false),
    copyArtifactPath: vi.fn(),
    copyMarkdownPath: vi.fn(),
    copyContinuationPrompt: vi.fn(),
    openTerminal: vi.fn(),
    ...overrides,
  };
}

function artifactWorkspace(kind: "open_spec_change" | "possible_artifact" = "open_spec_change") {
  const root = { id: "root_1", path: "/Users/dev/Programming" };
  const project: Project = {
    id: "project_1",
    name: "workbench",
    rootPath: "/Users/dev/Programming/workbench",
    git: { branch: "main" },
  };
  const member = {
    id: "artifact_1",
    relativePath: kind === "open_spec_change" ? "openspec/changes/ship-search/tasks.md" : "PLAN.md",
    evidence: kind === "open_spec_change" ? "OpenSpec" : "Candidate filename",
  };
  const bundle: IndexedBundle = {
    bundle: {
      id: "bundle_1",
      projectId: project.id,
      projectName: project.name,
      name: kind === "open_spec_change" ? "ship-search" : "PLAN.md",
      kind,
      recognition:
        kind === "open_spec_change"
          ? { status: "recognized", detector: "openspec-v1" }
          : { status: "possible", reason: "Candidate filename" },
      members: [member],
    },
    progress: {
      status: "available",
      progress: {
        total: 1,
        completed: 0,
        remainingCount: 1,
        tasks: [],
        remaining: [],
        parser: { name: "openspec-task-markers", version: "1" },
        warnings: [],
      },
    },
    fingerprint: "sha256:new",
    sourceModifiedUnixNanos: 1,
    warnings: [],
  };
  const index: IndexSnapshot = {
    rootId: root.id,
    generation: 1,
    indexedAt: "today",
    configurationRevision: 0,
    warnings: [],
    projects: [{ project, bundles: [bundle], markdownDocuments: [] }],
  };
  const detail: ArtifactDetail = {
    rootId: root.id,
    artifactId: member.id,
    bundleId: bundle.bundle.id,
    projectId: project.id,
    projectName: project.name,
    projectRoot: project.rootPath,
    git: project.git,
    bundleName: bundle.bundle.name,
    bundleKind: kind,
    recognition: bundle.bundle.recognition,
    members: bundle.bundle.members,
    relativePath: member.relativePath,
    absolutePath: `${project.rootPath}/${member.relativePath}`,
    sourceModifiedUnixNanos: bundle.sourceModifiedUnixNanos,
    markdown: "# Tasks\n\nKeep the interface recoverable.",
    progress: bundle.progress,
    fingerprint: bundle.fingerprint,
    warnings: [],
  };
  const mockApi = api({
    listRoots: vi.fn().mockResolvedValue([root]),
    scanRoot: vi.fn().mockResolvedValue({
      projects: [project],
      warnings: [],
      cancelled: false,
      entriesInspected: 10,
    }),
    getIndex: vi.fn().mockResolvedValue(index),
    getArtifactDetail: vi.fn().mockResolvedValue(detail),
  });
  return { root, project, bundle, index, detail, mockApi };
}

function markdownWorkspace() {
  const base = artifactWorkspace();
  const readme = {
    id: "document_readme",
    projectId: base.project.id,
    projectName: base.project.name,
    relativePath: "README.md",
    sourceModifiedUnixNanos: 3,
  };
  const bundleDocument = {
    id: base.bundle.bundle.members[0]!.id,
    projectId: base.project.id,
    projectName: base.project.name,
    relativePath: base.bundle.bundle.members[0]!.relativePath,
    sourceModifiedUnixNanos: 2,
  };
  const docsProject: Project = {
    id: "project_docs",
    name: "docs-only",
    rootPath: "/Users/dev/Programming/docs-only",
    git: { branch: "main" },
  };
  const guide = {
    id: "document_guide",
    projectId: docsProject.id,
    projectName: docsProject.name,
    relativePath: "notes/architecture-guide.md",
    sourceModifiedUnixNanos: 4,
  };
  const index: IndexSnapshot = {
    ...base.index,
    projects: [
      {
        project: base.project,
        bundles: [base.bundle],
        markdownDocuments: [readme, bundleDocument],
      },
      { project: docsProject, bundles: [], markdownDocuments: [guide] },
    ],
  };
  const detailById: Record<string, MarkdownDetail> = {
    [readme.id]: {
      rootId: base.root.id,
      documentId: readme.id,
      projectId: base.project.id,
      projectName: base.project.name,
      projectRoot: base.project.rootPath,
      git: base.project.git,
      relativePath: readme.relativePath,
      absolutePath: `${base.project.rootPath}/${readme.relativePath}`,
      sourceModifiedUnixNanos: readme.sourceModifiedUnixNanos,
      markdown: "# Workbench README\n\nOrdinary repository notes.",
    },
    [guide.id]: {
      rootId: base.root.id,
      documentId: guide.id,
      projectId: docsProject.id,
      projectName: docsProject.name,
      projectRoot: docsProject.rootPath,
      git: docsProject.git,
      relativePath: guide.relativePath,
      absolutePath: `${docsProject.rootPath}/${guide.relativePath}`,
      sourceModifiedUnixNanos: guide.sourceModifiedUnixNanos,
      markdown: "# Architecture guide\n\nDocumentation-only project.",
    },
  };
  const mockApi = api({
    listRoots: vi.fn().mockResolvedValue([base.root]),
    scanRoot: vi.fn().mockResolvedValue({
      projects: [base.project, docsProject],
      warnings: [],
      cancelled: false,
      entriesInspected: 20,
    }),
    getIndex: vi.fn().mockResolvedValue(index),
    getArtifactDetail: vi.fn().mockResolvedValue(base.detail),
    getMarkdownDetail: vi
      .fn()
      .mockImplementation(async (_rootId, documentId) => detailById[documentId]!),
  });
  return { ...base, docsProject, readme, guide, index, detailById, mockApi };
}

function structuredOpenSpecWorkspace() {
  const base = artifactWorkspace();
  const proposal = {
    id: "artifact_proposal",
    relativePath: "openspec/changes/ship-search/proposal.md",
    evidence: "OpenSpec",
  };
  const design = {
    id: "artifact_design",
    relativePath: "openspec/changes/ship-search/design.md",
    evidence: "OpenSpec",
  };
  const members = [proposal, design, ...base.bundle.bundle.members];
  const bundle = { ...base.bundle, bundle: { ...base.bundle.bundle, members } };
  const index = {
    ...base.index,
    projects: [{ project: base.project, bundles: [bundle], markdownDocuments: [] }],
  };
  const taskFacts: TaskFact[] = [
    { text: "Parse sections", completed: true, location: { line: 5, column: 3 } },
    { text: "Build overview", completed: false, location: { line: 6, column: 3 } },
  ];
  const openSpecView: OpenSpecView = {
    overview: [
      {
        kind: "why",
        sourcePath: proposal.relativePath,
        markdown: "Developers need the purpose before source metadata.",
      },
      {
        kind: "what_changes",
        sourcePath: proposal.relativePath,
        markdown: "- Add an overview\n- Keep exact source available",
      },
      {
        kind: "decisions",
        sourcePath: design.relativePath,
        markdown: "### Parse locally\n\nKeep observed facts deterministic.",
      },
    ],
    taskGroups: [
      {
        title: "1. Foundation",
        sourcePath: base.detail.relativePath,
        tasks: taskFacts,
      },
    ],
  };
  const detail = {
    ...base.detail,
    members,
    progress: {
      status: "available" as const,
      progress: {
        total: 2,
        completed: 1,
        remainingCount: 1,
        tasks: taskFacts,
        remaining: taskFacts.filter((task) => !task.completed),
        parser: { name: "openspec-task-markers", version: "1" },
        warnings: [],
      },
    },
    openSpecView,
  } satisfies ArtifactDetail;
  const sourceMarkdown = new Map([
    [proposal.id, "# Proposal\n\n## Why\n\nDevelopers need context."],
    [design.id, "# Design\n\n## Decisions\n\nParse locally."],
    [base.detail.artifactId, base.detail.markdown],
  ]);
  const mockApi = api({
    listRoots: vi.fn().mockResolvedValue([base.root]),
    scanRoot: vi.fn().mockResolvedValue({
      projects: [base.project],
      warnings: [],
      cancelled: false,
      entriesInspected: 10,
    }),
    getIndex: vi.fn().mockResolvedValue(index),
    getArtifactDetail: vi.fn().mockImplementation(async (_rootId, artifactId) => ({
      ...detail,
      artifactId,
      relativePath:
        members.find((member) => member.id === artifactId)?.relativePath ?? detail.relativePath,
      markdown: sourceMarkdown.get(artifactId) ?? detail.markdown,
    })),
    getGeneratedView: vi.fn().mockResolvedValue({ status: "never_generated" }),
  });
  return { ...base, bundle, index, detail, mockApi };
}

function twoRootWorkspace() {
  const first = artifactWorkspace();
  const secondRoot = { id: "root_2", path: "/Users/dev/Retained" };
  const secondProject: Project = {
    id: "project_2",
    name: "retained-workbench",
    rootPath: "/Users/dev/Retained/workbench",
    git: { branch: "main" },
  };
  const secondMember = {
    id: "artifact_2",
    relativePath: "openspec/changes/retained-change/tasks.md",
    evidence: "OpenSpec",
  };
  const secondBundle: IndexedBundle = {
    ...first.bundle,
    bundle: {
      ...first.bundle.bundle,
      id: "bundle_2",
      projectId: secondProject.id,
      projectName: secondProject.name,
      name: "retained-change",
      members: [secondMember],
    },
    sourceModifiedUnixNanos: 2,
  };
  const secondIndex: IndexSnapshot = {
    ...first.index,
    rootId: secondRoot.id,
    projects: [{ project: secondProject, bundles: [secondBundle], markdownDocuments: [] }],
  };
  const secondDetail: ArtifactDetail = {
    ...first.detail,
    rootId: secondRoot.id,
    artifactId: secondMember.id,
    bundleId: secondBundle.bundle.id,
    projectId: secondProject.id,
    projectName: secondProject.name,
    projectRoot: secondProject.rootPath,
    bundleName: secondBundle.bundle.name,
    members: [secondMember],
    relativePath: secondMember.relativePath,
    absolutePath: `${secondProject.rootPath}/${secondMember.relativePath}`,
    sourceModifiedUnixNanos: secondBundle.sourceModifiedUnixNanos,
  };
  const mockApi = api({
    listRoots: vi.fn().mockResolvedValue([first.root, secondRoot]),
    scanRoot: vi.fn().mockImplementation(async (rootId) => ({
      projects: rootId === first.root.id ? [first.project] : [secondProject],
      warnings: [],
      cancelled: false,
      entriesInspected: 10,
    })),
    getIndex: vi
      .fn()
      .mockImplementation(async (rootId) => (rootId === first.root.id ? first.index : secondIndex)),
    getArtifactDetail: vi
      .fn()
      .mockImplementation(async (_rootId, artifactId) =>
        artifactId === secondMember.id ? secondDetail : first.detail,
      ),
  });
  return { ...first, secondRoot, secondProject, secondBundle, secondIndex, secondDetail, mockApi };
}

describe("App root discovery", () => {
  it("renders the selected Backstage mark without replacing the accessible title", () => {
    const { container } = render(<App api={api()} />);

    expect(screen.getByLabelText("Backstage artifact control tower")).toBeVisible();
    const mark = container.querySelector("img.brand-mark");
    expect(mark).toHaveAttribute("alt", "");
    expect(mark).toHaveAttribute("src", expect.stringContaining("data:image/svg+xml"));
  });

  it("keeps the project rail visible on first run and explains root approval", async () => {
    render(<App api={api()} />);

    expect(
      await screen.findByRole("heading", { name: "Choose where Backstage can look" }),
    ).toBeVisible();
    expect(screen.getByLabelText("Project registry")).toBeVisible();
    expect(screen.queryByText("Approved roots")).not.toBeInTheDocument();
    expect(screen.getByText(/read-only/i)).toBeVisible();
  });

  it("approves a selected directory then scans it", async () => {
    const root = { id: "root_1", path: "/Users/dev/Programming" };
    const mockApi = api({
      chooseRoot: vi.fn().mockResolvedValue(root.path),
      approveRoot: vi.fn().mockResolvedValue(root),
      scanRoot: vi.fn().mockResolvedValue({
        projects: [],
        warnings: [],
        cancelled: false,
        entriesInspected: 12,
      }),
    });
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: "Approve a root" }));

    await waitFor(() => expect(mockApi.approveRoot).toHaveBeenCalledWith(root.path));
    await waitFor(() => expect(mockApi.scanRoot).toHaveBeenCalledWith(root.id));
    expect(await screen.findByText("No planning work found")).toBeVisible();
  });

  it("shows indexed projects and a warning-bearing ready state", async () => {
    const { project, mockApi } = artifactWorkspace();
    vi.mocked(mockApi.scanRoot).mockResolvedValue({
      projects: [project],
      warnings: [
        { code: "git_unavailable", path: "/tmp/other", message: "Git metadata unavailable" },
      ],
      cancelled: false,
      entriesInspected: 48,
    });

    render(<App api={mockApi} />);

    expect(
      await screen.findByRole("button", { name: /workbench.*1 planning file/i }),
    ).toBeVisible();
    expect(screen.getAllByText("Ready with 1 warning")).toHaveLength(2);
  });

  it("restores command palette focus and supports pane keyboard shortcuts", async () => {
    render(<App api={api()} />);
    await screen.findByRole("heading", { name: "Choose where Backstage can look" });
    const trigger = screen.getByRole("button", { name: "Open command palette" });

    await userEvent.click(trigger);
    expect(screen.getByRole("dialog", { name: "Command palette" })).toBeVisible();
    expect(screen.getByRole("textbox", { name: "Search commands" })).toHaveFocus();

    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(trigger).toHaveFocus());

    fireEvent.keyDown(window, { key: "3", altKey: true });
    expect(screen.getByLabelText("Reading desk")).toHaveFocus();
  });

  it("keeps the ledger visible after restoring a collapsed preference without a selection", async () => {
    const { mockApi } = artifactWorkspace();
    localStorage.setItem(
      "backstage.pane-layout.v1",
      JSON.stringify({ projectWidth: 244, ledgerWidth: 354, ledgerCollapsed: true }),
    );

    render(<App api={mockApi} />);

    expect(await screen.findByRole("button", { name: /ship-search/i })).toBeVisible();
    expect(screen.getByLabelText("Artifact workspace")).not.toHaveClass("ledger-is-collapsed");
  });

  it("returns from narrow artifact detail to the bundle ledger", async () => {
    const { mockApi } = artifactWorkspace();
    vi.stubGlobal("innerWidth", 680);
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));

    expect(await screen.findByRole("heading", { name: "ship-search" })).toBeVisible();
    expect(screen.getByLabelText("Reading desk")).toHaveFocus();
    expect(screen.getByLabelText("Artifact workspace")).toHaveClass("ledger-is-collapsed");
    await userEvent.click(screen.getByRole("button", { name: "Show bundle ledger" }));
    expect(screen.getByLabelText("Artifact workspace")).not.toHaveClass("ledger-is-collapsed");
  });

  it("contains command palette focus in both directions", async () => {
    render(<App api={api()} />);
    await screen.findByRole("heading", { name: "Choose where Backstage can look" });
    await userEvent.click(screen.getByRole("button", { name: "Open command palette" }));
    const dialog = screen.getByRole("dialog", { name: "Command palette" });
    const input = within(dialog).getByRole("textbox", { name: "Search commands" });
    const commands = within(dialog)
      .getAllByRole("button")
      .filter(
        (button): button is HTMLButtonElement =>
          button instanceof HTMLButtonElement && !button.disabled,
      );
    const last = commands.at(-1)!;

    last.focus();
    await userEvent.tab();
    expect(input).toHaveFocus();

    input.focus();
    await userEvent.tab({ shift: true });
    expect(last).toHaveFocus();
  });

  it("supports keyboard pane resizing and the advertised refresh shortcut", async () => {
    const { root, mockApi } = artifactWorkspace();
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    const separator = screen.getByRole("separator", { name: "Resize project registry" });

    separator.focus();
    fireEvent.keyDown(separator, { key: "ArrowRight" });
    expect(separator).toHaveAttribute("aria-valuenow", "252");

    vi.mocked(mockApi.scanRoot).mockClear();
    fireEvent.keyDown(window, { key: "r", metaKey: true });
    await waitFor(() => expect(mockApi.scanRoot).toHaveBeenCalledWith(root.id));
  });

  it("runs the refresh shortcut from non-editable command palette controls", async () => {
    const { root, mockApi } = artifactWorkspace();
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await userEvent.click(screen.getByRole("button", { name: "Open command palette" }));
    const refreshCommand = within(
      screen.getByRole("dialog", { name: "Command palette" }),
    ).getByRole("button", { name: /Refresh approved roots/ });
    refreshCommand.focus();
    vi.mocked(mockApi.scanRoot).mockClear();

    const propagated = fireEvent.keyDown(refreshCommand, { key: "r", ctrlKey: true });

    expect(propagated).toBe(false);
    await waitFor(() => expect(mockApi.scanRoot).toHaveBeenCalledWith(root.id));
  });

  it("shows deterministic project file counts and clear candidate language", async () => {
    const { mockApi } = artifactWorkspace("possible_artifact");
    render(<App api={mockApi} />);

    expect(
      await screen.findByRole("button", { name: /workbench.*1 planning file/i }),
    ).toBeVisible();
    expect(screen.getByText("Planning candidate")).toBeVisible();
    expect(screen.getByText(/Matched configured planning filename/)).toBeVisible();
  });

  it("opens recognized OpenSpec changes on a deterministic overview", async () => {
    const { mockApi } = structuredOpenSpecWorkspace();
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));

    expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("heading", { name: "Why this change" })).toBeVisible();
    expect(screen.getByText("Developers need the purpose before source metadata.")).toBeVisible();
    expect(screen.getByRole("heading", { name: "What changes" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Parse locally" })).toBeVisible();
    expect(screen.getByLabelText("Pi-generated Summary")).toBeVisible();
    expect(screen.queryByText("Fingerprint")).not.toBeInTheDocument();

    const overviewTab = screen.getByRole("tab", { name: "Overview" });
    overviewTab.focus();
    fireEvent.keyDown(overviewTab, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: /Tasks/ })).toHaveFocus();
    expect(screen.getByRole("tab", { name: /Tasks/ })).toHaveAttribute("aria-selected", "true");
  });

  it("shows every grouped OpenSpec task including completed work", async () => {
    const { mockApi } = structuredOpenSpecWorkspace();
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));

    await userEvent.click(screen.getByRole("tab", { name: /Tasks/ }));

    expect(screen.getByRole("heading", { name: "1. Foundation" })).toBeVisible();
    expect(screen.getByText("Parse sections")).toBeVisible();
    expect(screen.getByText("Build overview")).toBeVisible();
    expect(screen.getByText("1 complete · 1 remaining")).toBeVisible();
  });

  it("keeps exact OpenSpec source available and preserves Source during member loading", async () => {
    const { mockApi } = structuredOpenSpecWorkspace();
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await userEvent.click(screen.getByRole("tab", { name: "Source" }));

    await userEvent.click(screen.getByRole("button", { name: "proposal.md" }));

    await waitFor(() =>
      expect(mockApi.getArtifactDetail).toHaveBeenCalledWith("root_1", "artifact_proposal"),
    );
    expect(screen.getByRole("tab", { name: "Source" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("heading", { name: "Proposal" })).toBeVisible();
    await userEvent.click(screen.getByText("Source details"));
    expect(screen.getByText("Fingerprint")).toBeVisible();
  });

  it("ignores a stale source-member response after a newer member is selected", async () => {
    const { detail, mockApi } = structuredOpenSpecWorkspace();
    let resolveProposal: (value: ArtifactDetail) => void = () => undefined;
    const proposalResponse = new Promise<ArtifactDetail>((resolve) => {
      resolveProposal = resolve;
    });
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(async (_rootId, artifactId) => {
      if (artifactId === "artifact_proposal") return proposalResponse;
      const member = detail.members.find((candidate) => candidate.id === artifactId)!;
      return {
        ...detail,
        artifactId,
        relativePath: member.relativePath,
        markdown: artifactId === "artifact_design" ? "# Design" : detail.markdown,
      };
    });
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await userEvent.click(screen.getByRole("tab", { name: "Source" }));

    await userEvent.click(screen.getByRole("button", { name: "proposal.md" }));
    await userEvent.click(screen.getByRole("button", { name: "design.md" }));
    expect(await screen.findByRole("heading", { name: "Design" })).toBeVisible();

    await act(async () =>
      resolveProposal({
        ...detail,
        artifactId: "artifact_proposal",
        relativePath: "openspec/changes/ship-search/proposal.md",
        markdown: "# Proposal",
      }),
    );

    expect(screen.getByRole("heading", { name: "Design" })).toBeVisible();
    expect(screen.getByRole("button", { name: "design.md" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("opens readable artifact detail when the generated view lookup fails", async () => {
    const { mockApi } = structuredOpenSpecWorkspace();
    vi.mocked(mockApi.getGeneratedView)
      .mockResolvedValueOnce({ status: "never_generated" })
      .mockRejectedValueOnce(new Error("Generated view unavailable"));
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));

    expect(await screen.findByRole("heading", { name: "ship-search" })).toBeVisible();
    expect(screen.getByText("Never generated")).toBeVisible();
    expect(
      screen.getByText(/Generated summary unavailable: Generated view unavailable/),
    ).toBeVisible();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("does not show a completed Summary beneath a different selected bundle", async () => {
    const { bundle, detail, index, mockApi } = structuredOpenSpecWorkspace();
    const secondMember = {
      id: "artifact_second",
      relativePath: "openspec/changes/second-change/tasks.md",
      evidence: "OpenSpec",
    };
    const secondBundle: IndexedBundle = {
      ...bundle,
      bundle: {
        ...bundle.bundle,
        id: "bundle_second",
        name: "second-change",
        members: [secondMember],
      },
    };
    vi.mocked(mockApi.getIndex).mockResolvedValue({
      ...index,
      projects: [
        {
          project: index.projects[0]!.project,
          bundles: [bundle, secondBundle],
          markdownDocuments: [],
        },
      ],
    });
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(async (_rootId, artifactId) =>
      artifactId === secondMember.id
        ? {
            ...detail,
            artifactId,
            bundleId: secondBundle.bundle.id,
            bundleName: secondBundle.bundle.name,
            members: [secondMember],
            relativePath: secondMember.relativePath,
          }
        : detail,
    );
    let resolveSummary: (value: GeneratedView) => void = () => undefined;
    vi.mocked(mockApi.requestSummary).mockImplementation(
      () =>
        new Promise<GeneratedView>((resolve) => {
          resolveSummary = resolve;
        }),
    );
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await userEvent.click(screen.getByRole("button", { name: "Generate Summary" }));
    await waitFor(() => expect(mockApi.requestSummary).toHaveBeenCalledWith("root_1", "bundle_1"));

    await userEvent.click(screen.getByRole("button", { name: /second-change/i }));
    expect(await screen.findByRole("heading", { name: "second-change" })).toBeVisible();

    await act(async () =>
      resolveSummary({
        status: "current",
        result: {
          text: "Summary for the first bundle only",
          mode: "summary",
          sourceFingerprint: "sha256:first",
          includedPaths: [detail.relativePath],
          generatedAt: "today",
          model: "test-model",
          promptVersion: "summary-v1",
        },
      }),
    );

    expect(screen.queryByText("Summary for the first bundle only")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "second-change" })).toBeVisible();
  });

  it("defaults to a readable proposal when task progress is unavailable", async () => {
    const { bundle, detail, index, mockApi } = structuredOpenSpecWorkspace();
    const unavailableProgress = {
      status: "unavailable" as const,
      progress: {
        parser: { name: "openspec-task-markers", version: "1" },
        warnings: [{ line: 1, message: "tasks.md could not be parsed" }],
      },
    };
    vi.mocked(mockApi.getIndex).mockResolvedValue({
      ...index,
      projects: [
        {
          ...index.projects[0]!,
          bundles: [{ ...bundle, progress: unavailableProgress }],
        },
      ],
    });
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(async (_rootId, artifactId) => {
      if (artifactId === "artifact_1") throw new Error("tasks.md is unreadable");
      const member = detail.members.find((candidate) => candidate.id === artifactId)!;
      return {
        ...detail,
        artifactId,
        relativePath: member.relativePath,
        markdown: "# Proposal\n\nReadable proposal text.",
        progress: unavailableProgress,
      };
    });
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));

    await waitFor(() =>
      expect(mockApi.getArtifactDetail).toHaveBeenCalledWith("root_1", "artifact_proposal"),
    );
    await userEvent.click(screen.getByRole("tab", { name: "Source" }));
    expect(screen.getByRole("heading", { name: "Proposal" })).toBeVisible();

    await userEvent.click(screen.getByRole("button", { name: "tasks.md" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("tasks.md is unreadable");
    expect(screen.getByRole("heading", { name: "Proposal" })).toBeVisible();
    expect(screen.getByRole("button", { name: "proposal.md" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("recovers from missing OpenSpec overview sections and task facts", async () => {
    const { detail, mockApi } = structuredOpenSpecWorkspace();
    vi.mocked(mockApi.getArtifactDetail).mockResolvedValue({
      ...detail,
      progress: {
        status: "unavailable",
        progress: {
          parser: { name: "openspec-task-markers", version: "1" },
          warnings: [{ line: 3, message: "unsupported task marker state" }],
        },
      },
      openSpecView: { overview: [], taskGroups: [] },
    });
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));

    expect(screen.getByRole("heading", { name: "No overview sections found" })).toBeVisible();
    expect(screen.getByRole("tab", { name: "Tasks" })).toBeVisible();
    expect(screen.queryByRole("tab", { name: "Tasks 0" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: "Tasks" }));
    expect(screen.getByRole("heading", { name: "Structured tasks unavailable" })).toBeVisible();
    expect(screen.getByText(/unsupported task marker state/)).toBeVisible();
  });

  it("keeps planning candidates in the plain source reader", async () => {
    const { mockApi } = artifactWorkspace("possible_artifact");
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /plan\.md/i }));

    expect(screen.queryByRole("tab", { name: "Overview" })).not.toBeInTheDocument();
    expect(screen.getByLabelText("Rendered artifact Markdown")).toBeVisible();
  });

  it("opens ordinary Markdown in the local generic reader without planning or Pi controls", async () => {
    const { mockApi, readme } = markdownWorkspace();
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: "All Markdown" }));
    vi.mocked(mockApi.getGeneratedView).mockClear();

    await userEvent.click(screen.getByRole("button", { name: /README\.md/i }));

    await waitFor(() =>
      expect(mockApi.getMarkdownDetail).toHaveBeenCalledWith("root_1", readme.id),
    );
    expect(await screen.findByRole("heading", { name: "README.md" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Workbench README" })).toBeVisible();
    expect(screen.getByLabelText("Rendered Markdown document")).toBeVisible();
    expect(screen.getByLabelText("Reading desk")).toHaveFocus();
    expect(screen.queryByRole("tab", { name: "Overview" })).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Pi-generated Summary")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /continuation prompt/i })).not.toBeInTheDocument();
    expect(mockApi.getGeneratedView).not.toHaveBeenCalled();
  });

  it("copies the selected ordinary Markdown path and reports success", async () => {
    const { detailById, mockApi, readme, root } = markdownWorkspace();
    vi.mocked(mockApi.copyMarkdownPath).mockResolvedValue(detailById[readme.id]!.absolutePath);
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: "All Markdown" }));
    await userEvent.click(screen.getByRole("button", { name: /README\.md/i }));

    await userEvent.click(await screen.findByRole("button", { name: "Copy path" }));

    expect(mockApi.copyMarkdownPath).toHaveBeenCalledWith(root.id, readme.id);
    expect(screen.getByRole("status")).toHaveTextContent("Markdown path copied");
  });

  it("reports a Markdown path-copy failure without showing success", async () => {
    const { mockApi } = markdownWorkspace();
    vi.mocked(mockApi.copyMarkdownPath).mockRejectedValue(
      new Error("Markdown path is no longer safely available"),
    );
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: "All Markdown" }));
    await userEvent.click(screen.getByRole("button", { name: /README\.md/i }));

    await userEvent.click(await screen.findByRole("button", { name: "Copy path" }));

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Markdown path is no longer safely available",
    );
    expect(screen.queryByText("Markdown path copied")).not.toBeInTheDocument();
  });

  it("ignores a delayed Markdown response after returning to Plan files", async () => {
    const { detailById, mockApi, readme } = markdownWorkspace();
    let resolveDocument: (value: MarkdownDetail) => void = () => undefined;
    vi.mocked(mockApi.getMarkdownDetail).mockImplementation(
      () =>
        new Promise<MarkdownDetail>((resolve) => {
          resolveDocument = resolve;
        }),
    );
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: "All Markdown" }));
    await userEvent.click(screen.getByRole("button", { name: /README\.md/i }));
    await waitFor(() => expect(mockApi.getMarkdownDetail).toHaveBeenCalled());

    await userEvent.click(screen.getByRole("button", { name: "Plan files" }));
    expect(await screen.findByRole("heading", { name: "ship-search" })).toBeVisible();
    await act(async () => resolveDocument(detailById[readme.id]!));

    expect(screen.getByRole("heading", { name: "ship-search" })).toBeVisible();
    expect(screen.queryByRole("heading", { name: "README.md" })).not.toBeInTheDocument();
  });

  it("ignores a delayed bundle response after a Markdown document is selected", async () => {
    const { detail, mockApi } = markdownWorkspace();
    let resolveArtifact: (value: ArtifactDetail) => void = () => undefined;
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(
      () =>
        new Promise<ArtifactDetail>((resolve) => {
          resolveArtifact = resolve;
        }),
    );
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await userEvent.click(screen.getByRole("button", { name: "All Markdown" }));
    await userEvent.click(screen.getByRole("button", { name: /README\.md/i }));
    expect(await screen.findByRole("heading", { name: "README.md" })).toBeVisible();

    await act(async () => resolveArtifact(detail));

    expect(screen.getByRole("heading", { name: "README.md" })).toBeVisible();
    expect(screen.queryByRole("heading", { name: "ship-search" })).not.toBeInTheDocument();
  });

  it("defaults to Plan files and keeps ordinary Markdown out of planning navigation", async () => {
    const { mockApi } = markdownWorkspace();

    render(<App api={mockApi} />);

    expect(await screen.findByRole("button", { name: "Plan files" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(
      await screen.findByRole("button", { name: /workbench.*1 planning file/i }),
    ).toBeVisible();
    expect(screen.queryByRole("button", { name: /docs-only/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /README\.md/i })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /architecture-guide\.md/i }),
    ).not.toBeInTheDocument();
  });

  it("reveals every unique Markdown file and Markdown-only project in All Markdown", async () => {
    const { mockApi } = markdownWorkspace();
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: "All Markdown" }));

    expect(screen.getByRole("button", { name: /workbench.*2 Markdown files/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /docs-only.*1 Markdown file/i })).toBeVisible();
    expect(screen.getByRole("button", { name: "All Work, 2 projects" })).toBeVisible();
    expect(screen.getByText("3 files")).toBeVisible();
    expect(screen.getByRole("button", { name: /ship-search/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /README\.md/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /architecture-guide\.md/i })).toBeVisible();
    expect(screen.getAllByRole("button", { name: /ship-search|tasks\.md/i })).toHaveLength(1);

    await userEvent.type(
      screen.getByRole("searchbox", { name: "Search all indexed work" }),
      "notes/architecture",
    );
    expect(screen.queryByRole("button", { name: /README\.md/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /architecture-guide\.md/i })).toBeVisible();
  });

  it("deduplicates projects and Markdown reached through overlapping approved roots", async () => {
    const { index, mockApi, root } = markdownWorkspace();
    const nestedRoot = { id: "root_2", path: `${root.path}/workbench` };
    vi.mocked(mockApi.listRoots).mockResolvedValue([root, nestedRoot]);
    vi.mocked(mockApi.getIndex).mockImplementation(async (rootId) => ({
      ...index,
      rootId,
    }));

    render(<App api={mockApi} />);

    expect(await screen.findByRole("button", { name: "All Work, 1 project" })).toBeVisible();
    expect(screen.getAllByRole("button", { name: /workbench.*1 planning file/i })).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "All Markdown" }));

    expect(screen.getByRole("button", { name: "All Work, 2 projects" })).toBeVisible();
    expect(screen.getAllByRole("button", { name: /workbench.*2 Markdown files/i })).toHaveLength(1);
    expect(screen.getAllByRole("button", { name: /docs-only.*1 Markdown file/i })).toHaveLength(1);
    expect(screen.getAllByRole("button", { name: /ship-search/i })).toHaveLength(1);
    expect(screen.getAllByRole("button", { name: /README\.md/i })).toHaveLength(1);
    expect(screen.getAllByRole("button", { name: /architecture-guide\.md/i })).toHaveLength(1);
    expect(screen.getByText("3 files")).toBeVisible();
  });

  it("reuses the selected overlap owner for a Markdown path handoff", async () => {
    const { detailById, index, mockApi, readme, root } = markdownWorkspace();
    const firstRoot = { id: "root_z", path: root.path };
    const ownerRoot = { id: "root_a", path: `${root.path}/workbench` };
    vi.mocked(mockApi.listRoots).mockResolvedValue([firstRoot, ownerRoot]);
    vi.mocked(mockApi.getIndex).mockImplementation(async (rootId) => ({ ...index, rootId }));
    vi.mocked(mockApi.getMarkdownDetail).mockImplementation(async (rootId, documentId) => ({
      ...detailById[documentId]!,
      rootId,
    }));
    vi.mocked(mockApi.copyMarkdownPath).mockResolvedValue(detailById[readme.id]!.absolutePath);

    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: "All Markdown" }));
    await userEvent.click(screen.getByRole("button", { name: /README\.md/i }));
    await userEvent.click(await screen.findByRole("button", { name: "Copy path" }));

    expect(mockApi.getMarkdownDetail).toHaveBeenCalledWith(ownerRoot.id, readme.id);
    expect(mockApi.copyMarkdownPath).toHaveBeenCalledWith(ownerRoot.id, readme.id);
  });

  it("reuses the selected overlap owner for handoffs and Summary requests", async () => {
    const { detail, index, mockApi, root } = markdownWorkspace();
    const firstRoot = { id: "root_z", path: root.path };
    const ownerRoot = { id: "root_a", path: `${root.path}/workbench` };
    vi.mocked(mockApi.listRoots).mockResolvedValue([firstRoot, ownerRoot]);
    vi.mocked(mockApi.getIndex).mockImplementation(async (rootId) => ({ ...index, rootId }));
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(async (rootId) => ({
      ...detail,
      rootId,
    }));
    vi.mocked(mockApi.requestSummary).mockResolvedValue({ status: "never_generated" });

    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await waitFor(() =>
      expect(mockApi.getArtifactDetail).toHaveBeenCalledWith(ownerRoot.id, "artifact_1"),
    );

    await userEvent.click(screen.getByRole("button", { name: "Copy path" }));
    await userEvent.click(screen.getByRole("button", { name: "Generate Summary" }));

    expect(mockApi.copyArtifactPath).toHaveBeenCalledWith(ownerRoot.id, "artifact_1");
    expect(mockApi.requestSummary).toHaveBeenCalledWith(ownerRoot.id, "bundle_1");
  });

  it("does not invent planning state for ordinary Markdown rows", async () => {
    const { mockApi } = markdownWorkspace();
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: "All Markdown" }));

    await userEvent.click(screen.getByRole("button", { name: "Active" }));

    expect(screen.getByRole("button", { name: /ship-search/i })).toBeVisible();
    expect(screen.queryByRole("button", { name: /README\.md/i })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /architecture-guide\.md/i }),
    ).not.toBeInTheDocument();
  });

  it("omits projects without indexed planning files from work navigation and counts", async () => {
    const { project, index, mockApi } = artifactWorkspace();
    const emptyProject: Project = {
      id: "project_empty",
      name: "empty-project",
      rootPath: "/Users/dev/Programming/empty-project",
      git: { branch: "main" },
    };
    vi.mocked(mockApi.scanRoot).mockResolvedValue({
      projects: [project, emptyProject],
      warnings: [],
      cancelled: false,
      entriesInspected: 20,
    });
    vi.mocked(mockApi.getIndex).mockResolvedValue({
      ...index,
      projects: [...index.projects, { project: emptyProject, bundles: [], markdownDocuments: [] }],
    });

    render(<App api={mockApi} />);

    expect(
      await screen.findByRole("button", { name: /workbench.*1 planning file/i }),
    ).toBeVisible();
    expect(screen.queryByRole("button", { name: /empty-project/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "All Work, 1 project" })).toBeVisible();
    expect(screen.getByText("1 project with planning work is in scope.")).toBeVisible();
  });

  it("does not reparse unchanged Markdown during shell-only updates", async () => {
    const renderSpy = vi.spyOn(markdown, "renderMarkdown");
    const { mockApi } = artifactWorkspace();
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await screen.findByRole("heading", { name: "ship-search" });
    const callsAfterSelection = renderSpy.mock.calls.length;

    await userEvent.click(screen.getByRole("button", { name: "Hide bundle ledger" }));

    expect(renderSpy).toHaveBeenCalledTimes(callsAfterSelection);
  });

  it("bounds pointer resize updates to one animation frame", async () => {
    const { mockApi } = artifactWorkspace();
    const frames: FrameRequestCallback[] = [];
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => {
      frames.push(callback);
      return frames.length;
    });
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    const separator = screen.getByRole("separator", { name: "Resize project registry" });

    fireEvent(separator, new MouseEvent("pointerdown", { bubbles: true, clientX: 244 }));
    fireEvent(window, new MouseEvent("pointermove", { bubbles: true, clientX: 250 }));
    fireEvent(window, new MouseEvent("pointermove", { bubbles: true, clientX: 260 }));

    expect(window.requestAnimationFrame).toHaveBeenCalledTimes(1);
    act(() => frames[0]!(0));
    expect(separator).toHaveAttribute("aria-valuenow", "260");
    fireEvent(window, new MouseEvent("pointerup", { bubbles: true, clientX: 260 }));

    fireEvent(separator, new MouseEvent("pointerdown", { bubbles: true, clientX: 260 }));
    fireEvent(window, new MouseEvent("pointermove", { bubbles: true, clientX: 265 }));
    fireEvent(window, new MouseEvent("pointerup", { bubbles: true, clientX: 280 }));
    expect(separator).toHaveAttribute("aria-valuenow", "280");
  });

  it("cancels a pending pointer resize frame on unmount", async () => {
    const { mockApi } = artifactWorkspace();
    vi.spyOn(window, "requestAnimationFrame").mockReturnValue(41);
    const cancelFrame = vi.spyOn(window, "cancelAnimationFrame");
    const { unmount } = render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    const separator = screen.getByRole("separator", { name: "Resize project registry" });

    fireEvent(separator, new MouseEvent("pointerdown", { bubbles: true, clientX: 244 }));
    fireEvent(window, new MouseEvent("pointermove", { bubbles: true, clientX: 250 }));
    unmount();

    expect(cancelFrame).toHaveBeenCalledWith(41);
  });

  it("shows indexed bundles and filters them by deterministic state", async () => {
    const root = { id: "root_1", path: "/Users/dev/Programming" };
    const mockApi = api({
      listRoots: vi.fn().mockResolvedValue([root]),
      scanRoot: vi.fn().mockResolvedValue({
        projects: [
          {
            id: "project_1",
            name: "workbench",
            rootPath: "/Users/dev/Programming/workbench",
            git: { branch: "main" },
          },
        ],
        warnings: [],
        cancelled: false,
        entriesInspected: 48,
      }),
      getIndex: vi.fn().mockResolvedValue({
        rootId: root.id,
        generation: 1,
        indexedAt: "2026-08-13T12:00:00Z",
        warnings: [],
        projects: [
          {
            project: {
              id: "project_1",
              name: "workbench",
              rootPath: "/Users/dev/Programming/workbench",
              git: { branch: "main" },
            },
            markdownDocuments: [],
            bundles: [
              {
                bundle: {
                  id: "bundle_incomplete",
                  projectId: "project_1",
                  projectName: "workbench",
                  name: "ship-search",
                  kind: "open_spec_change",
                  recognition: { status: "recognized", detector: "openspec-v1" },
                  members: [
                    {
                      id: "artifact_tasks",
                      relativePath: "openspec/changes/ship-search/tasks.md",
                      evidence: "OpenSpec change material",
                    },
                  ],
                },
                progress: {
                  status: "available",
                  progress: {
                    total: 2,
                    completed: 1,
                    remainingCount: 1,
                    tasks: [],
                    remaining: [
                      {
                        text: "Filter bundles",
                        completed: false,
                        location: { line: 3, column: 3 },
                      },
                    ],
                    parser: { name: "openspec-task-markers", version: "1" },
                    warnings: [],
                  },
                },
                fingerprint: "sha256:abc",
                sourceModifiedUnixNanos: 1,
                warnings: [],
              },
              {
                bundle: {
                  id: "bundle_warning",
                  projectId: "project_1",
                  projectName: "workbench",
                  name: "PLAN.md",
                  kind: "possible_artifact",
                  recognition: { status: "possible", reason: "Candidate filename" },
                  members: [
                    {
                      id: "artifact_plan",
                      relativePath: "PLAN.md",
                      evidence: "Candidate filename",
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
                fingerprint: "sha256:def",
                sourceModifiedUnixNanos: 1,
                warnings: ["Possible artifact requires review"],
              },
            ],
          },
        ],
      }),
    });
    render(<App api={mockApi} />);

    expect(await screen.findByRole("button", { name: /ship-search/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /PLAN.md/i })).toBeVisible();

    await userEvent.click(screen.getByRole("button", { name: /ship-search/i }));
    expect(mockApi.getArtifactDetail).toHaveBeenCalledWith(root.id, "artifact_tasks");

    await userEvent.click(screen.getByRole("button", { name: "Warning-bearing" }));

    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /PLAN.md/i })).toBeVisible();
  });

  it("opens top-level Settings from the titlebar and command palette, then restores focus", async () => {
    const { mockApi } = artifactWorkspace();
    Object.assign(mockApi, {
      listPatterns: vi.fn().mockResolvedValue({
        revision: 0,
        patterns: [
          {
            id: "pattern_plan",
            expression: "(?:^|/)(?:PLAN|plan)\\.md$",
            ordinal: 0,
            provenance: "default",
          },
        ],
      }),
      addPattern: vi.fn(),
      removePattern: vi.fn(),
      restoreDefaultPatterns: vi.fn(),
    });
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    const settingsTrigger = screen.getByRole("button", { name: "Settings" });

    await userEvent.click(settingsTrigger);

    expect(screen.getByRole("heading", { name: "Settings" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Approved roots" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Planning patterns" })).toBeVisible();
    expect(screen.queryByLabelText("Project registry")).not.toBeInTheDocument();
    expect(screen.getByText("/Users/dev/Programming")).toBeVisible();
    expect(screen.getByText("(?:^|/)(?:PLAN|plan)\\.md$")).toBeVisible();

    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(screen.getByLabelText("Project registry")).toBeVisible();
    expect(settingsTrigger).toHaveFocus();

    await userEvent.click(screen.getByRole("button", { name: "Open command palette" }));
    await userEvent.click(
      within(screen.getByRole("dialog", { name: "Command palette" })).getByRole("button", {
        name: "Open Settings",
      }),
    );
    expect(screen.getByRole("heading", { name: "Settings" })).toHaveFocus();
  });

  it("publishes a broad ledger before bounded generated inventory hydration settles", async () => {
    const { bundle, index, mockApi, project } = artifactWorkspace();
    const bundles = Array.from({ length: 24 }, (_, position): IndexedBundle => ({
      ...bundle,
      bundle: {
        ...bundle.bundle,
        id: `bundle_${position}`,
        name: `change-${String(position).padStart(2, "0")}`,
        members: [
          {
            id: `artifact_${position}`,
            relativePath: `openspec/changes/change-${position}/tasks.md`,
            evidence: "OpenSpec",
          },
        ],
      },
      sourceModifiedUnixNanos: position + 1,
    }));
    const broadIndex: IndexSnapshot = {
      ...index,
      projects: [{ ...index.projects[0]!, bundles }],
    };
    vi.mocked(mockApi.getIndex).mockResolvedValue(broadIndex);
    vi.mocked(mockApi.scanRoot).mockResolvedValue({
      projects: [project],
      warnings: [
        { code: "broad_pattern", path: project.rootPath, message: "Broad pattern matched" },
      ],
      cancelled: false,
      entriesInspected: 200,
    });
    const pending: Array<ReturnType<typeof deferred<GeneratedView>>> = [];
    let active = 0;
    let peak = 0;
    vi.mocked(mockApi.getGeneratedView).mockImplementation(() => {
      const request = deferred<GeneratedView>();
      pending.push(request);
      active += 1;
      peak = Math.max(peak, active);
      return request.promise.then(
        (view) => {
          active -= 1;
          return view;
        },
        (cause) => {
          active -= 1;
          throw cause;
        },
      );
    });
    render(<App api={mockApi} />);

    expect(await screen.findByText("24 records")).toBeVisible();
    expect(screen.getAllByText("Ready with 1 warning")).toHaveLength(2);
    expect(mockApi.getGeneratedView).toHaveBeenCalledTimes(4);
    expect(peak).toBe(4);

    let settled = 0;
    let rejected = false;
    while (settled < bundles.length) {
      await waitFor(() => expect(pending.length).toBeGreaterThan(0));
      const batch = pending.splice(0);
      await act(async () => {
        for (const request of batch) {
          if (!rejected) {
            rejected = true;
            request.reject(new Error("one generated lookup failed"));
          } else {
            request.resolve({ status: "never_generated" });
          }
        }
        await Promise.resolve();
      });
      settled += batch.length;
      if (settled < bundles.length) {
        await waitFor(() =>
          expect(mockApi.getGeneratedView).toHaveBeenCalledTimes(
            Math.min(settled + 4, bundles.length),
          ),
        );
      }
    }

    await waitFor(() => expect(active).toBe(0));
    expect(mockApi.getGeneratedView).toHaveBeenCalledTimes(bundles.length);
    expect(peak).toBeLessThanOrEqual(4);
    expect(screen.getAllByText("Ready with 1 warning")).toHaveLength(2);
    expect(screen.getByRole("button", { name: /change-23/i })).toBeVisible();
  });

  it("does not publish a delayed generated-view scan batch after final-root removal", async () => {
    const { detail, mockApi } = artifactWorkspace();
    const generatedBatch = deferred<GeneratedView>();
    const removedSummary = {
      text: "Removed summary that must not return",
      mode: "summary" as const,
      sourceFingerprint: "sha256:removed",
      includedPaths: [detail.relativePath],
      generatedAt: "today",
      model: "test-model",
      promptVersion: "summary-v1",
    };
    vi.mocked(mockApi.getGeneratedView).mockReturnValue(generatedBatch.promise);
    vi.mocked(mockApi.removeRoot).mockResolvedValue({ roots: [], indexes: [] });
    render(<App api={mockApi} />);

    await waitFor(() =>
      expect(mockApi.getGeneratedView).toHaveBeenCalledWith("root_1", "bundle_1"),
    );
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    await userEvent.click(screen.getByRole("button", { name: "Remove" }));
    await userEvent.click(
      within(screen.getByRole("alertdialog", { name: "Remove approved root?" })).getByRole(
        "button",
        { name: "Remove approval" },
      ),
    );
    expect(await screen.findByText("No folders are being scanned.")).toBeVisible();

    await act(async () =>
      generatedBatch.resolve({
        status: "stale",
        result: removedSummary,
        changedInputs: [detail.relativePath],
      }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Done" }));

    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
    expect(screen.queryByText(removedSummary.text)).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Stale" }));
    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
  });

  it("confirms root removal, uses the authoritative inventory, and suppresses delayed detail", async () => {
    const { detail, mockApi } = artifactWorkspace();
    let resolveDetail: (value: ArtifactDetail) => void = () => undefined;
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(
      () =>
        new Promise<ArtifactDetail>((resolve) => {
          resolveDetail = resolve;
        }),
    );
    vi.mocked(mockApi.removeRoot).mockResolvedValue({ roots: [], indexes: [] });
    Object.assign(mockApi, {
      listPatterns: vi.fn().mockResolvedValue({ revision: 0, patterns: [] }),
      addPattern: vi.fn(),
      removePattern: vi.fn(),
      restoreDefaultPatterns: vi.fn(),
    });
    render(<App api={mockApi} />);
    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    await userEvent.click(screen.getByRole("button", { name: "Remove" }));

    const confirmation = screen.getByRole("alertdialog", { name: "Remove approved root?" });
    expect(confirmation).toHaveTextContent(
      "Backstage forgets approval, index, and unreachable generated summaries",
    );
    expect(confirmation).toHaveTextContent("Repository files remain untouched");
    await userEvent.click(within(confirmation).getByRole("button", { name: "Cancel" }));
    expect(mockApi.removeRoot).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Remove" }));
    await userEvent.click(
      within(screen.getByRole("alertdialog", { name: "Remove approved root?" })).getByRole(
        "button",
        { name: "Remove approval" },
      ),
    );

    await waitFor(() => expect(mockApi.removeRoot).toHaveBeenCalledWith("root_1"));
    expect(await screen.findByText("No folders are being scanned.")).toBeVisible();
    await act(async () => resolveDetail(detail));
    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(screen.queryByRole("heading", { name: "ship-search" })).not.toBeInTheDocument();
  });

  it.each([
    ["completed", false],
    ["failed", true],
  ])("does not restore %s Summary state after final-root removal", async (_outcome, rejects) => {
    const { root, detail, mockApi } = artifactWorkspace();
    const previous = {
      text: "Previous summary that must stay forgotten",
      mode: "summary" as const,
      sourceFingerprint: "sha256:old",
      includedPaths: [detail.relativePath],
      generatedAt: "yesterday",
      model: "test-model",
      promptVersion: "summary-v1",
    };
    const summary = deferred<GeneratedView>();
    vi.mocked(mockApi.getGeneratedView).mockResolvedValue({
      status: "stale",
      result: previous,
      changedInputs: [detail.relativePath],
    });
    vi.mocked(mockApi.requestSummary).mockReturnValue(summary.promise);
    vi.mocked(mockApi.removeRoot).mockResolvedValue({ roots: [], indexes: [] });
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    expect(await screen.findByText(previous.text)).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "Regenerate Summary" }));
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    await userEvent.click(screen.getByRole("button", { name: "Remove" }));
    await userEvent.click(
      within(screen.getByRole("alertdialog", { name: "Remove approved root?" })).getByRole(
        "button",
        { name: "Remove approval" },
      ),
    );
    expect(await screen.findByText("No folders are being scanned.")).toBeVisible();

    await act(async () => {
      if (rejects) {
        summary.reject(new Error("late summary failure"));
      } else {
        summary.resolve({
          status: "stale",
          result: {
            ...previous,
            text: "Late generated summary that must stay forgotten",
            sourceFingerprint: "sha256:new",
            generatedAt: "today",
          },
          changedInputs: [detail.relativePath],
        });
      }
    });
    if (!rejects) {
      const rescan = deferred<Awaited<ReturnType<BackstageApi["scanRoot"]>>>();
      vi.mocked(mockApi.chooseRoot).mockResolvedValue(root.path);
      vi.mocked(mockApi.approveRoot).mockResolvedValue(root);
      vi.mocked(mockApi.scanRoot).mockReturnValueOnce(rescan.promise);
      await userEvent.click(screen.getByRole("button", { name: "Add root" }));
      expect(await screen.findByText(root.path)).toBeVisible();
    }
    await userEvent.click(screen.getByRole("button", { name: "Done" }));

    expect(
      screen.queryByText("Late generated summary that must stay forgotten"),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("late summary failure")).not.toBeInTheDocument();
    expect(screen.queryByText(previous.text)).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "ship-search" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Stale" }));
    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
  });

  it("selects retained visible work after an authoritative root removal", async () => {
    const { root, secondRoot, secondIndex, mockApi } = twoRootWorkspace();
    vi.mocked(mockApi.removeRoot).mockResolvedValue({
      roots: [secondRoot],
      indexes: [secondIndex],
    });
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    expect(await screen.findByRole("heading", { name: "ship-search" })).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    const firstRootRow = screen.getByLabelText(`Approved root ${root.path}`).closest("li")!;
    await userEvent.click(within(firstRootRow).getByRole("button", { name: "Remove" }));
    await userEvent.click(within(firstRootRow).getByRole("button", { name: "Remove approval" }));

    await waitFor(() =>
      expect(mockApi.getArtifactDetail).toHaveBeenCalledWith(secondRoot.id, "artifact_2"),
    );
    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(await screen.findByRole("heading", { name: "retained-change" })).toBeVisible();
  });

  it("selects retained visible work after a pattern mutation removes the selection", async () => {
    const { bundle, detail, index, mockApi } = artifactWorkspace();
    const retainedMember = {
      id: "artifact_retained",
      relativePath: "openspec/changes/retained-pattern-change/tasks.md",
      evidence: "OpenSpec",
    };
    const retainedBundle: IndexedBundle = {
      ...bundle,
      bundle: {
        ...bundle.bundle,
        id: "bundle_retained",
        name: "retained-pattern-change",
        members: [retainedMember],
      },
      sourceModifiedUnixNanos: 2,
    };
    const initialIndex = {
      ...index,
      projects: [{ ...index.projects[0]!, bundles: [bundle, retainedBundle] }],
    };
    const retainedIndex = {
      ...index,
      configurationRevision: 1,
      projects: [{ ...index.projects[0]!, bundles: [retainedBundle] }],
    };
    const pattern = {
      id: "pattern_plan",
      expression: "(?:^|/)PLAN\\.md$",
      ordinal: 0,
      provenance: "custom" as const,
    };
    vi.mocked(mockApi.getIndex).mockResolvedValue(initialIndex);
    vi.mocked(mockApi.listPatterns).mockResolvedValue({ revision: 0, patterns: [pattern] });
    vi.mocked(mockApi.removePattern).mockResolvedValue({
      patterns: [],
      configurationRevision: 1,
      indexes: [retainedIndex],
      failedRootIds: [],
    });
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(async (_rootId, artifactId) =>
      artifactId === retainedMember.id
        ? {
            ...detail,
            artifactId,
            bundleId: retainedBundle.bundle.id,
            bundleName: retainedBundle.bundle.name,
            members: [retainedMember],
            relativePath: retainedMember.relativePath,
          }
        : detail,
    );
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    const patternRow = screen.getByText(pattern.expression).closest("li")!;
    await userEvent.click(within(patternRow).getByRole("button", { name: "Remove" }));

    await waitFor(() =>
      expect(mockApi.getArtifactDetail).toHaveBeenCalledWith("root_1", retainedMember.id),
    );
    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(await screen.findByRole("heading", { name: retainedBundle.bundle.name })).toBeVisible();
  });

  it("settles an invalidated active scan from the authoritative pattern result", async () => {
    const { index, mockApi, project } = artifactWorkspace();
    const refresh = deferred<Awaited<ReturnType<BackstageApi["scanRoot"]>>>();
    const pattern = {
      id: "pattern_plan",
      expression: "(?:^|/)PLAN\\.md$",
      ordinal: 0,
      provenance: "custom" as const,
    };
    vi.mocked(mockApi.scanRoot)
      .mockResolvedValueOnce({
        projects: [project],
        warnings: [],
        cancelled: false,
        entriesInspected: 10,
      })
      .mockReturnValueOnce(refresh.promise);
    vi.mocked(mockApi.listPatterns).mockResolvedValue({ revision: 0, patterns: [pattern] });
    vi.mocked(mockApi.removePattern).mockResolvedValue({
      patterns: [],
      configurationRevision: 1,
      indexes: [{ ...index, configurationRevision: 1 }],
      failedRootIds: [],
    });
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });

    await userEvent.click(screen.getByRole("button", { name: "Refresh approved roots" }));
    expect(screen.getAllByText("Scanning read-only").length).toBeGreaterThan(0);
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    const patternRow = await screen.findByText(pattern.expression);
    await userEvent.click(
      within(patternRow.closest("li")!).getByRole("button", { name: "Remove" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Done" }));

    expect(screen.getAllByText("Ready").length).toBeGreaterThan(0);
    expect(screen.queryByText("Scanning read-only")).not.toBeInTheDocument();
    await act(async () =>
      refresh.resolve({
        projects: [],
        warnings: [{ code: "late", path: "/removed", message: "must be ignored" }],
        cancelled: false,
        entriesInspected: 0,
      }),
    );
    expect(screen.getAllByText("Ready").length).toBeGreaterThan(0);
  });

  it("ignores a delayed stale pattern load after a newer mutation revision", async () => {
    const { index, mockApi } = artifactWorkspace();
    const staleLoad = deferred<{ revision: number; patterns: [] }>();
    const customPattern = {
      id: "pattern_docs",
      expression: "^docs/plans/.*\\.md$",
      ordinal: 0,
      provenance: "custom" as const,
    };
    vi.mocked(mockApi.listPatterns)
      .mockResolvedValueOnce({ revision: 0, patterns: [] })
      .mockReturnValueOnce(staleLoad.promise);
    vi.mocked(mockApi.addPattern).mockResolvedValue({
      patterns: [customPattern],
      configurationRevision: 1,
      indexes: [{ ...index, configurationRevision: 1 }],
      failedRootIds: [],
    });
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await waitFor(() => expect(mockApi.listPatterns).toHaveBeenCalledTimes(1));

    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    await waitFor(() => expect(mockApi.listPatterns).toHaveBeenCalledTimes(2));
    await userEvent.type(
      screen.getByRole("textbox", { name: "Regular expression" }),
      customPattern.expression,
    );
    await userEvent.click(screen.getByRole("button", { name: "Add pattern" }));
    expect(await screen.findByText(customPattern.expression)).toBeVisible();
    expect(screen.getByText(/Configuration revision 1/)).toBeVisible();

    await act(async () => staleLoad.resolve({ revision: 0, patterns: [] }));

    expect(screen.getByText(customPattern.expression)).toBeVisible();
    expect(screen.getByText(/Configuration revision 1/)).toBeVisible();
  });

  it("admits add-root before chooser work and blocks removal until it finishes", async () => {
    const { root, secondRoot, secondIndex, mockApi } = twoRootWorkspace();
    const choice = deferred<string | null>();
    const addedRoot = { id: "root_3", path: "/Users/dev/Added" };
    vi.mocked(mockApi.chooseRoot).mockReturnValue(choice.promise);
    vi.mocked(mockApi.approveRoot).mockResolvedValue(addedRoot);
    vi.mocked(mockApi.removeRoot).mockResolvedValue({
      roots: [secondRoot, addedRoot],
      indexes: [secondIndex],
    });
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));

    await userEvent.click(screen.getByRole("button", { name: "Add root" }));

    const firstRootRow = screen.getByLabelText(`Approved root ${root.path}`).closest("li")!;
    expect(within(firstRootRow).getByRole("button", { name: "Remove" })).toBeDisabled();
    fireEvent.click(within(firstRootRow).getByRole("button", { name: "Remove" }));
    expect(mockApi.removeRoot).not.toHaveBeenCalled();

    await act(async () => choice.resolve(addedRoot.path));
    await waitFor(() => expect(mockApi.approveRoot).toHaveBeenCalledWith(addedRoot.path));
    await waitFor(() => expect(screen.getByText(addedRoot.path)).toBeVisible());

    await userEvent.click(within(firstRootRow).getByRole("button", { name: "Remove" }));
    await userEvent.click(
      within(screen.getByRole("alertdialog", { name: "Remove approved root?" })).getByRole(
        "button",
        { name: "Remove approval" },
      ),
    );

    await waitFor(() => expect(screen.queryByText(root.path)).not.toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retained-change/i })).toBeVisible();
  });

  it("blocks pattern invocation during root removal so removed inventory cannot return", async () => {
    const { root, index, secondRoot, secondIndex, mockApi } = twoRootWorkspace();
    const removal = deferred<{ roots: (typeof secondRoot)[]; indexes: IndexSnapshot[] }>();
    const patternMutation = deferred<Awaited<ReturnType<BackstageApi["removePattern"]>>>();
    const pattern = {
      id: "pattern_plan",
      expression: "(?:^|/)PLAN\\.md$",
      ordinal: 0,
      provenance: "custom" as const,
    };
    vi.mocked(mockApi.listPatterns).mockResolvedValue({ revision: 0, patterns: [pattern] });
    vi.mocked(mockApi.removeRoot).mockReturnValue(removal.promise);
    vi.mocked(mockApi.removePattern).mockReturnValue(patternMutation.promise);
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Regular expression" }), "^plans/");
    const firstRootRow = screen.getByLabelText(`Approved root ${root.path}`).closest("li")!;
    await userEvent.click(within(firstRootRow).getByRole("button", { name: "Remove" }));
    await userEvent.click(within(firstRootRow).getByRole("button", { name: "Remove approval" }));

    const patternRow = screen.getByText(pattern.expression).closest("li")!;
    const removePattern = within(patternRow).getByRole("button", { name: "Remove" });
    expect(screen.getByRole("textbox", { name: "Regular expression" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Settings busy…" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Restore defaults" })).toBeDisabled();
    expect(removePattern).toBeDisabled();
    fireEvent.click(removePattern);
    expect(mockApi.removePattern).not.toHaveBeenCalled();

    await act(async () => removal.resolve({ roots: [secondRoot], indexes: [secondIndex] }));
    await act(async () =>
      patternMutation.resolve({
        patterns: [],
        configurationRevision: 1,
        indexes: [index, secondIndex],
        failedRootIds: [],
      }),
    );
    await waitFor(() => expect(screen.queryByText(root.path)).not.toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retained-change/i })).toBeVisible();
  });

  it("serializes root removals by disabling every root mutation control", async () => {
    const { root, mockApi } = twoRootWorkspace();
    const removal = deferred<{ roots: []; indexes: [] }>();
    vi.mocked(mockApi.removeRoot).mockReturnValue(removal.promise);
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    const firstRootRow = screen.getByLabelText(`Approved root ${root.path}`).closest("li")!;
    await userEvent.click(within(firstRootRow).getByRole("button", { name: "Remove" }));
    await userEvent.click(within(firstRootRow).getByRole("button", { name: "Remove approval" }));

    expect(screen.getByRole("button", { name: "Add root" })).toBeDisabled();
    for (const button of screen.getAllByRole("button", { name: /Remove|Removing/ })) {
      expect(button).toBeDisabled();
    }
  });

  it("identifies failed rescans on their root rows and clears them after retry", async () => {
    const { index, mockApi, root, secondRoot } = twoRootWorkspace();
    const pattern = {
      id: "pattern_docs",
      expression: "^docs/plans/.*\\.md$",
      ordinal: 0,
      provenance: "custom" as const,
    };
    vi.mocked(mockApi.addPattern).mockResolvedValue({
      patterns: [pattern],
      configurationRevision: 1,
      indexes: [{ ...index, configurationRevision: 1 }],
      failedRootIds: [root.id],
    });
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    await userEvent.type(
      screen.getByRole("textbox", { name: "Regular expression" }),
      pattern.expression,
    );
    await userEvent.click(screen.getByRole("button", { name: "Add pattern" }));

    const failedRow = screen.getByLabelText(`Approved root ${root.path}`).closest("li")!;
    const healthyRow = screen.getByLabelText(`Approved root ${secondRoot.path}`).closest("li")!;
    expect(failedRow).toHaveTextContent("Rescan failed · last successful index retained");
    expect(within(failedRow).getByRole("button", { name: "Retry" })).toBeVisible();
    expect(healthyRow).not.toHaveTextContent("Rescan failed");

    await userEvent.click(within(failedRow).getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(failedRow).not.toHaveTextContent("Rescan failed"));
  });

  it("manages accessible root-removal confirmation focus and pending state", async () => {
    const { mockApi } = artifactWorkspace();
    const removal = deferred<{ roots: []; indexes: [] }>();
    vi.mocked(mockApi.removeRoot).mockReturnValue(removal.promise);
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));
    const trigger = screen.getByRole("button", { name: "Remove" });

    await userEvent.click(trigger);
    let confirmation = screen.getByRole("alertdialog", { name: "Remove approved root?" });
    expect(within(confirmation).getByRole("button", { name: "Cancel" })).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();

    await userEvent.click(trigger);
    confirmation = screen.getByRole("alertdialog", { name: "Remove approved root?" });
    await userEvent.click(within(confirmation).getByRole("button", { name: "Remove approval" }));
    expect(within(confirmation).getByRole("button", { name: "Cancel" })).toBeDisabled();
    expect(within(confirmation).getByRole("button", { name: "Remove approval" })).toBeDisabled();

    await act(async () => removal.resolve({ roots: [], indexes: [] }));
    await screen.findByText("No folders are being scanned.");
    await waitFor(() => expect(screen.getByRole("heading", { name: "Settings" })).toHaveFocus());
  });

  it("adds and removes planning patterns, surfaces backend validation, and warns about retries", async () => {
    const { index, mockApi } = artifactWorkspace();
    const defaultPattern = {
      id: "pattern_plan",
      expression: "(?:^|/)(?:PLAN|plan)\\.md$",
      ordinal: 0,
      provenance: "default" as const,
    };
    const customPattern = {
      id: "pattern_docs",
      expression: "^docs/plans/.*\\.md$",
      ordinal: 1,
      provenance: "custom" as const,
    };
    const listPatterns = vi.fn().mockResolvedValue({ revision: 2, patterns: [defaultPattern] });
    const addPattern = vi.fn().mockResolvedValue({
      patterns: [defaultPattern, customPattern],
      configurationRevision: 3,
      indexes: [{ ...index, configurationRevision: 3 }],
      failedRootIds: ["root_1"],
    });
    const restoreDefaultPatterns = vi.fn().mockResolvedValue({
      patterns: [defaultPattern, customPattern],
      configurationRevision: 4,
      indexes: [{ ...index, configurationRevision: 4 }],
      failedRootIds: [],
    });
    const removePattern = vi.fn().mockResolvedValue({
      patterns: [defaultPattern],
      configurationRevision: 5,
      indexes: [{ ...index, configurationRevision: 5 }],
      failedRootIds: [],
    });
    Object.assign(mockApi, {
      listPatterns,
      addPattern,
      removePattern,
      restoreDefaultPatterns,
    });
    render(<App api={mockApi} />);
    await screen.findByRole("button", { name: /ship-search/i });
    await userEvent.click(screen.getByRole("button", { name: "Settings" }));

    const input = screen.getByRole("textbox", { name: "Regular expression" });
    await userEvent.type(input, customPattern.expression);
    await userEvent.click(screen.getByRole("button", { name: "Add pattern" }));

    expect(addPattern).toHaveBeenCalledWith(customPattern.expression);
    expect(await screen.findByText(customPattern.expression)).toBeVisible();
    expect(screen.getByText("Custom")).toBeVisible();
    expect(screen.getByRole("alert")).toHaveTextContent(/could not be rescanned.*retry/i);
    expect(screen.getByRole("button", { name: "Add pattern" })).toHaveFocus();
    const restoreButton = screen.getByRole("button", { name: "Restore defaults" });
    await userEvent.click(restoreButton);
    expect(restoreDefaultPatterns).toHaveBeenCalledOnce();
    expect(restoreButton).toHaveFocus();
    const customRow = screen.getByText(customPattern.expression).closest("li")!;
    await userEvent.click(within(customRow).getByRole("button", { name: "Remove" }));
    expect(removePattern).toHaveBeenCalledWith(customPattern.id);
    await waitFor(() => expect(input).toHaveFocus());

    vi.mocked(addPattern).mockRejectedValueOnce({
      code: "planning_pattern_invalid",
      message: "Regular expression could not compile: unclosed group",
    });
    await userEvent.clear(input);
    await userEvent.type(input, "(");
    await userEvent.click(screen.getByRole("button", { name: "Add pattern" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("unclosed group");
    expect(screen.getByText(/project-relative Markdown paths/i)).toBeVisible();
    expect(screen.getByText(/broad patterns are allowed/i)).toBeVisible();
  });

  it("defaults to Current and keeps OpenSpec lifecycle separate from progress", async () => {
    const { bundle: activeBundle, detail, index, mockApi } = structuredOpenSpecWorkspace();
    const doneBundle: IndexedBundle = {
      ...activeBundle,
      bundle: { ...activeBundle.bundle, id: "bundle_done", name: "done-change" },
      primaryStatus: "done",
      progress: {
        status: "available",
        progress: {
          total: 2,
          completed: 2,
          remainingCount: 0,
          tasks: [],
          remaining: [],
          parser: { name: "openspec-task-markers", version: "1" },
          warnings: [],
        },
      },
      sourceModifiedUnixNanos: 4,
    };
    const archivedMember = {
      id: "artifact_archived_tasks",
      relativePath: "openspec/changes/archive/2026-08-12-archived-change/tasks.md",
      evidence: "OpenSpec",
    };
    const archivedBundle: IndexedBundle = {
      ...activeBundle,
      bundle: {
        ...activeBundle.bundle,
        id: "bundle_archived",
        name: "archived-change",
        members: [archivedMember],
        custody: { status: "archived", archivedOn: "2026-08-12" },
      },
      primaryStatus: "archived",
      progress: {
        status: "available",
        progress: {
          total: 2,
          completed: 1,
          remainingCount: 1,
          tasks: [],
          remaining: [],
          parser: { name: "openspec-task-markers", version: "1" },
          warnings: [],
        },
      },
      sourceModifiedUnixNanos: 5,
    };
    const currentBundle: IndexedBundle = {
      ...activeBundle,
      bundle: { ...activeBundle.bundle, custody: { status: "current" } },
      primaryStatus: "active",
      progress: {
        status: "available",
        progress: {
          total: 2,
          completed: 1,
          remainingCount: 1,
          tasks: [],
          remaining: [],
          parser: { name: "openspec-task-markers", version: "1" },
          warnings: [],
        },
      },
      sourceModifiedUnixNanos: 3,
    };
    vi.mocked(mockApi.getIndex).mockResolvedValue({
      ...index,
      configurationRevision: 1,
      projects: [{ ...index.projects[0]!, bundles: [currentBundle, doneBundle, archivedBundle] }],
    });
    vi.mocked(mockApi.getArtifactDetail).mockImplementation(async (_rootId, artifactId) => {
      if (artifactId === archivedBundle.bundle.members[0]!.id) {
        return {
          ...detail,
          artifactId: archivedMember.id,
          bundleId: archivedBundle.bundle.id,
          bundleName: archivedBundle.bundle.name,
          members: [archivedMember],
          relativePath: archivedMember.relativePath,
          custody: archivedBundle.bundle.custody,
          primaryStatus: "archived",
        };
      }
      return detail;
    });
    Object.assign(mockApi, {
      listPatterns: vi.fn().mockResolvedValue({ revision: 1, patterns: [] }),
      addPattern: vi.fn(),
      removePattern: vi.fn(),
      restoreDefaultPatterns: vi.fn(),
    });
    render(<App api={mockApi} />);

    expect(await screen.findByRole("button", { name: "Current" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: /ship-search/i })).toHaveTextContent(
      "Active1 open · 1 done",
    );
    expect(screen.getByRole("button", { name: /done-change/i })).toHaveTextContent(
      "Done0 open · 2 done",
    );
    expect(screen.queryByRole("button", { name: /archived-change/i })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Archived" }));
    expect(screen.queryByRole("button", { name: /ship-search/i })).not.toBeInTheDocument();
    const archivedRow = screen.getByRole("button", { name: /archived-change/i });
    expect(archivedRow).toHaveTextContent("Archived1 open · 1 done");
    await userEvent.click(archivedRow);
    expect(await screen.findByRole("heading", { name: "archived-change" })).toBeVisible();
    expect(screen.getByText("Archived", { selector: ".artifact-lifecycle" })).toBeVisible();
    expect(screen.getByRole("tab", { name: "Overview" })).toBeVisible();
  });

  it("sorts all rows globally into local-date groups and navigates across headings", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.setSystemTime(new Date(2026, 7, 14, 12, 0, 0));
    const { bundle, index, mockApi } = artifactWorkspace();
    const makeBundle = (id: string, name: string, date: Date | null): IndexedBundle => ({
      ...bundle,
      bundle: { ...bundle.bundle, id, name },
      primaryStatus: "active",
      sourceModifiedUnixNanos: date ? date.getTime() * 1_000_000 : null,
    });
    const today = makeBundle("today", "today-change", new Date(2026, 7, 14, 9));
    const week = makeBundle("week", "week-change", new Date(2026, 7, 8, 18));
    const older = makeBundle("older", "older-change", new Date(2026, 7, 6, 18));
    const unknown = makeBundle("unknown", "unknown-change", null);
    vi.mocked(mockApi.getIndex).mockResolvedValue({
      ...index,
      configurationRevision: 0,
      projects: [{ ...index.projects[0]!, bundles: [older, unknown, week, today] }],
    });
    Object.assign(mockApi, {
      listPatterns: vi.fn().mockResolvedValue({ revision: 0, patterns: [] }),
      addPattern: vi.fn(),
      removePattern: vi.fn(),
      restoreDefaultPatterns: vi.fn(),
    });
    render(<App api={mockApi} />);
    const ledger = screen.getByLabelText("Bundle ledger");
    await within(ledger).findByRole("button", { name: /today-change/i });

    expect(
      within(ledger)
        .getAllByRole("heading")
        .map((heading) => heading.textContent),
    ).toEqual(["Today", "Past 7 days", "Older", "Date unavailable"]);
    expect(
      within(ledger)
        .getAllByRole("button", { name: /-change/i })
        .map((row) => row.textContent),
    ).toEqual([
      expect.stringContaining("today-change"),
      expect.stringContaining("week-change"),
      expect.stringContaining("older-change"),
      expect.stringContaining("unknown-change"),
    ]);

    const todayRow = within(ledger).getByRole("button", { name: /today-change/i });
    todayRow.focus();
    fireEvent.keyDown(todayRow, { key: "ArrowDown" });
    expect(within(ledger).getByRole("button", { name: /week-change/i })).toHaveFocus();
    vi.useRealTimers();
  });

  it("shows a failed regeneration while preserving the previous Summary", async () => {
    const root = { id: "root_1", path: "/Users/dev/Programming" };
    const project = {
      id: "project_1",
      name: "workbench",
      rootPath: "/Users/dev/Programming/workbench",
      git: { branch: "main" },
    };
    const bundle = {
      bundle: {
        id: "bundle_1",
        projectId: project.id,
        projectName: project.name,
        name: "ship-search",
        kind: "open_spec_change" as const,
        recognition: { status: "recognized" as const, detector: "openspec-v1" },
        members: [
          {
            id: "artifact_1",
            relativePath: "openspec/changes/ship-search/tasks.md",
            evidence: "OpenSpec",
          },
        ],
      },
      progress: {
        status: "available" as const,
        progress: {
          total: 1,
          completed: 0,
          remainingCount: 1,
          tasks: [],
          remaining: [],
          parser: { name: "openspec-task-markers", version: "1" },
          warnings: [],
        },
      },
      fingerprint: "sha256:new",
      sourceModifiedUnixNanos: 1,
      warnings: [],
    };
    const member = bundle.bundle.members[0]!;
    const previous = {
      text: "Prior summary",
      mode: "summary" as const,
      sourceFingerprint: "sha256:old",
      includedPaths: ["tasks.md"],
      generatedAt: "yesterday",
      model: "model",
      promptVersion: "summary-v1",
    };
    const mockApi = api({
      listRoots: vi.fn().mockResolvedValue([root]),
      scanRoot: vi.fn().mockResolvedValue({
        projects: [project],
        warnings: [],
        cancelled: false,
        entriesInspected: 10,
      }),
      getIndex: vi.fn().mockResolvedValue({
        rootId: root.id,
        generation: 1,
        indexedAt: "today",
        warnings: [],
        projects: [{ project, bundles: [bundle], markdownDocuments: [] }],
      }),
      getArtifactDetail: vi.fn().mockResolvedValue({
        rootId: root.id,
        artifactId: "artifact_1",
        bundleId: "bundle_1",
        projectId: project.id,
        projectName: project.name,
        projectRoot: project.rootPath,
        git: project.git,
        bundleName: "ship-search",
        bundleKind: "open_spec_change",
        recognition: bundle.bundle.recognition,
        members: bundle.bundle.members,
        relativePath: member.relativePath,
        absolutePath: `${project.rootPath}/${member.relativePath}`,
        sourceModifiedUnixNanos: bundle.sourceModifiedUnixNanos,
        markdown: "# Tasks",
        progress: bundle.progress,
        fingerprint: bundle.fingerprint,
        warnings: [],
      }),
      getGeneratedView: vi
        .fn()
        .mockResolvedValue({ status: "stale", result: previous, changedInputs: ["tasks.md"] }),
      requestSummary: vi.fn().mockRejectedValue(new Error("Pi timed out")),
    });
    render(<App api={mockApi} />);

    await userEvent.click(await screen.findByRole("button", { name: /ship-search/i }));
    expect(await screen.findByText("Prior summary")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "Regenerate Summary" }));

    expect(await screen.findByText(/Generation failed: Pi timed out/)).toBeVisible();
    expect(screen.getByText("Prior summary")).toBeVisible();
  });
});
