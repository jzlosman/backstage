import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open } from "@tauri-apps/plugin-dialog";

export interface ApprovedRoot {
  id: string;
  path: string;
}

export interface GitContext {
  branch: string;
}

export interface Project {
  id: string;
  name: string;
  rootPath: string;
  git: GitContext | null;
}

export interface ScanWarning {
  code: string;
  path: string;
  message: string;
}

export interface DiscoveryResult {
  projects: Project[];
  warnings: ScanWarning[];
  cancelled: boolean;
  entriesInspected: number;
}

export interface ArtifactMember {
  id: string;
  relativePath: string;
  evidence: string;
}

export interface Recognition {
  status: "recognized" | "possible";
  detector?: string;
  reason?: string;
}

export type OpenSpecCustody =
  { status: "current" } | { status: "archived"; archivedOn: string | null };

export type OpenSpecPrimaryStatus = "active" | "done" | "archived";

export interface Bundle {
  id: string;
  projectId: string;
  projectName: string;
  name: string;
  kind: "open_spec_change" | "possible_artifact";
  recognition: Recognition;
  members: ArtifactMember[];
  custody?: OpenSpecCustody;
}

export interface SourceLocation {
  line: number;
  column: number;
}

export interface TaskFact {
  text: string;
  completed: boolean;
  location: SourceLocation;
}

export type OpenSpecProgress =
  | {
      status: "available";
      progress: {
        total: number;
        completed: number;
        remainingCount: number;
        tasks: TaskFact[];
        remaining: TaskFact[];
        parser: { name: string; version: string };
        warnings: Array<{ line: number; message: string }>;
      };
    }
  | {
      status: "unavailable";
      progress: {
        parser: { name: string; version: string };
        warnings: Array<{ line: number; message: string }>;
      };
    };

export type SourceTimestamp = string | number | null;

export interface IndexedBundle {
  bundle: Bundle;
  progress: OpenSpecProgress;
  primaryStatus?: OpenSpecPrimaryStatus;
  fingerprint: string | null;
  sourceModifiedUnixNanos: SourceTimestamp;
  warnings: string[];
}

export interface MarkdownDocument {
  id: string;
  projectId: string;
  projectName: string;
  relativePath: string;
  sourceModifiedUnixNanos: SourceTimestamp;
}

export interface IndexedProject {
  project: Project;
  bundles: IndexedBundle[];
  markdownDocuments: MarkdownDocument[];
}

export interface IndexSnapshot {
  rootId: string;
  generation: number;
  indexedAt: string;
  configurationRevision: number;
  projects: IndexedProject[];
  warnings: ScanWarning[];
}

export interface PlanningPattern {
  id: string;
  expression: string;
  ordinal: number;
  provenance: "default" | "custom";
}

export interface PlanningPatternConfiguration {
  revision: number;
  patterns: PlanningPattern[];
}

export interface PatternMutation {
  patterns: PlanningPattern[];
  configurationRevision: number;
  indexes: IndexSnapshot[];
  failedRootIds: string[];
}

export interface RootRemovalInventory {
  roots: ApprovedRoot[];
  indexes: IndexSnapshot[];
}

export type GeneratedView =
  | { status: "never_generated"; capabilityReason?: string }
  | { status: "generating"; previous?: GeneratedResult }
  | { status: "current"; result: GeneratedResult }
  | { status: "stale"; result: GeneratedResult; changedInputs: string[] }
  | { status: "failed"; previous?: GeneratedResult; failure: string };

export interface GeneratedResult {
  text: string;
  mode: "summary";
  sourceFingerprint: string;
  includedPaths: string[];
  generatedAt: string;
  model: string | null;
  promptVersion: string;
}

export type OpenSpecOverviewKind =
  "why" | "what_changes" | "goals_and_non_goals" | "decisions" | "risks_and_trade_offs";

export interface OpenSpecOverviewSection {
  kind: OpenSpecOverviewKind;
  sourcePath: string;
  markdown: string;
}

export interface OpenSpecTaskGroup {
  title: string;
  sourcePath: string;
  tasks: TaskFact[];
}

export interface OpenSpecView {
  overview: OpenSpecOverviewSection[];
  taskGroups: OpenSpecTaskGroup[];
}

export interface MarkdownDetail {
  rootId: string;
  documentId: string;
  projectId: string;
  projectName: string;
  projectRoot: string;
  git: GitContext | null;
  relativePath: string;
  absolutePath: string;
  sourceModifiedUnixNanos: SourceTimestamp;
  markdown: string;
}

export interface ArtifactDetail {
  rootId: string;
  artifactId: string;
  bundleId: string;
  projectId: string;
  projectName: string;
  projectRoot: string;
  git: GitContext | null;
  bundleName: string;
  members: ArtifactMember[];
  bundleKind: Bundle["kind"];
  recognition: Recognition;
  custody?: OpenSpecCustody;
  primaryStatus?: OpenSpecPrimaryStatus;
  relativePath: string;
  absolutePath: string;
  sourceModifiedUnixNanos: SourceTimestamp;
  markdown: string;
  progress: OpenSpecProgress;
  fingerprint: string | null;
  warnings: string[];
  openSpecView?: OpenSpecView | null;
}

export interface BackstageApi {
  listRoots(): Promise<ApprovedRoot[]>;
  chooseRoot(): Promise<string | null>;
  approveRoot(path: string): Promise<ApprovedRoot>;
  removeRoot(rootId: string): Promise<RootRemovalInventory>;
  listPatterns(): Promise<PlanningPatternConfiguration>;
  addPattern(expression: string): Promise<PatternMutation>;
  removePattern(id: string): Promise<PatternMutation>;
  restoreDefaultPatterns(): Promise<PatternMutation>;
  scanRoot(rootId: string): Promise<DiscoveryResult>;
  cancelScan(rootId: string): Promise<boolean>;
  getIndex(rootId: string): Promise<IndexSnapshot | null>;
  getArtifactDetail(rootId: string, artifactId: string): Promise<ArtifactDetail>;
  getMarkdownDetail(rootId: string, documentId: string): Promise<MarkdownDetail>;
  getGeneratedView(rootId: string, bundleId: string): Promise<GeneratedView>;
  requestSummary(rootId: string, bundleId: string): Promise<GeneratedView>;
  cancelSummary(requestId: string): Promise<boolean>;
  copyArtifactPath(rootId: string, artifactId: string): Promise<string>;
  copyMarkdownPath(rootId: string, documentId: string): Promise<string>;
  copyContinuationPrompt(rootId: string, artifactId: string): Promise<string>;
  openTerminal(rootId: string, projectId: string): Promise<void>;
}

const tauriApi: BackstageApi = {
  listRoots: () => invoke<ApprovedRoot[]>("list_roots"),
  chooseRoot: async () => {
    const selected = await open({ directory: true, multiple: false, title: "Approve a scan root" });
    return typeof selected === "string" ? selected : null;
  },
  approveRoot: (path) => invoke<ApprovedRoot>("approve_root", { path }),
  removeRoot: (rootId) => invoke<RootRemovalInventory>("remove_root", { rootId }),
  listPatterns: () => invoke<PlanningPatternConfiguration>("list_patterns"),
  addPattern: (expression) => invoke<PatternMutation>("add_pattern", { expression }),
  removePattern: (id) => invoke<PatternMutation>("remove_pattern", { id }),
  restoreDefaultPatterns: () => invoke<PatternMutation>("restore_default_patterns"),
  scanRoot: (rootId) => invoke<DiscoveryResult>("scan_root", { rootId }),
  cancelScan: (rootId) => invoke<boolean>("cancel_scan", { rootId }),
  getIndex: (rootId) => invoke<IndexSnapshot | null>("get_index", { rootId }),
  getArtifactDetail: (rootId, artifactId) =>
    invoke<ArtifactDetail>("get_artifact_detail", { rootId, artifactId }),
  getMarkdownDetail: (rootId, documentId) =>
    invoke<MarkdownDetail>("get_markdown_detail", { rootId, documentId }),
  getGeneratedView: (rootId, bundleId) =>
    invoke<GeneratedView>("get_generated_view", { rootId, bundleId }),
  requestSummary: (rootId, bundleId) =>
    invoke<GeneratedView>("request_summary", { rootId, bundleId }),
  cancelSummary: (requestId) => invoke<boolean>("cancel_summary", { requestId }),
  copyArtifactPath: async (rootId, artifactId) => {
    const path = await invoke<string>("copy_artifact_path", { rootId, artifactId });
    await writeText(path);
    return path;
  },
  copyMarkdownPath: async (rootId, documentId) => {
    const path = await invoke<string>("copy_markdown_path", { rootId, documentId });
    await writeText(path);
    return path;
  },
  copyContinuationPrompt: async (rootId, artifactId) => {
    const prompt = await invoke<string>("copy_continuation_prompt", { rootId, artifactId });
    await writeText(prompt);
    return prompt;
  },
  openTerminal: (rootId, projectId) => invoke<void>("open_terminal", { rootId, projectId }),
};

const previewRoot: ApprovedRoot = { id: "preview-root", path: "/Users/developer/Programming" };
const previewProject: Project = {
  id: "preview-project",
  name: "atlas-workbench",
  rootPath: "/Users/developer/Programming/atlas-workbench",
  git: { branch: "feature/artifact-index" },
};
const previewMembers: ArtifactMember[] = [
  {
    id: "preview-proposal",
    relativePath: "openspec/changes/build-artifact-control-tower/proposal.md",
    evidence: "Path is supported OpenSpec change material",
  },
  {
    id: "preview-design",
    relativePath: "openspec/changes/build-artifact-control-tower/design.md",
    evidence: "Path is supported OpenSpec change material",
  },
  {
    id: "preview-artifact",
    relativePath: "openspec/changes/build-artifact-control-tower/tasks.md",
    evidence: "Path is supported OpenSpec change material",
  },
  {
    id: "preview-spec",
    relativePath:
      "openspec/changes/build-artifact-control-tower/specs/artifact-control-tower/spec.md",
    evidence: "Path is supported OpenSpec change material",
  },
];
const previewTasks: TaskFact[] = [
  {
    text: "Discover Git working trees beneath approved roots",
    completed: true,
    location: { line: 5, column: 3 },
  },
  {
    text: "Group recognized OpenSpec files into changes",
    completed: true,
    location: { line: 6, column: 3 },
  },
  {
    text: "Keep repository access strictly read-only",
    completed: true,
    location: { line: 10, column: 3 },
  },
  {
    text: "Render deterministic task progress",
    completed: true,
    location: { line: 11, column: 3 },
  },
  {
    text: "Verify the packaged macOS smoke flow",
    completed: false,
    location: { line: 15, column: 3 },
  },
  {
    text: "Record supported syntax and data locations",
    completed: false,
    location: { line: 16, column: 3 },
  },
];
const previewReadme: MarkdownDocument = {
  id: "preview-readme",
  projectId: previewProject.id,
  projectName: previewProject.name,
  relativePath: "README.md",
  sourceModifiedUnixNanos: "1786631000000000000",
};

const previewBundle: IndexedBundle = {
  bundle: {
    id: "preview-bundle",
    projectId: previewProject.id,
    projectName: previewProject.name,
    name: "build-artifact-control-tower",
    kind: "open_spec_change",
    recognition: { status: "recognized", detector: "openspec-v1" },
    members: previewMembers,
    custody: { status: "current" },
  },
  primaryStatus: "active",
  progress: {
    status: "available",
    progress: {
      total: previewTasks.length,
      completed: previewTasks.filter((task) => task.completed).length,
      remainingCount: previewTasks.filter((task) => !task.completed).length,
      tasks: previewTasks,
      remaining: previewTasks.filter((task) => !task.completed),
      parser: { name: "openspec-task-markers", version: "1" },
      warnings: [],
    },
  },
  fingerprint: "sha256:c01d7e317566d94f7c5eae5910de5687",
  sourceModifiedUnixNanos: "1786632000000000000",
  warnings: [],
};

const previewDefaultPatterns: PlanningPattern[] = [
  {
    id: "preview-plan-pattern",
    expression: "(?:^|/)(?:PLAN|plan)\\.md$",
    ordinal: 0,
    provenance: "default",
  },
  {
    id: "preview-tdd-pattern",
    expression: "(?:^|/)(?:TDD|tdd)\\.md$",
    ordinal: 1,
    provenance: "default",
  },
  {
    id: "preview-roadmap-pattern",
    expression: "(?:^|/)(?:ROADMAP|roadmap)\\.md$",
    ordinal: 2,
    provenance: "default",
  },
];
let previewPatterns = [...previewDefaultPatterns];
let previewPatternRevision = 0;

function previewIndex(): IndexSnapshot {
  return {
    rootId: previewRoot.id,
    generation: 4 + previewPatternRevision,
    indexedAt: "2026-08-13T12:04:00Z",
    configurationRevision: previewPatternRevision,
    warnings: [],
    projects: [
      {
        project: previewProject,
        bundles: [previewBundle],
        markdownDocuments: [
          previewReadme,
          ...previewMembers.map((member) => ({
            id: member.id,
            projectId: previewProject.id,
            projectName: previewProject.name,
            relativePath: member.relativePath,
            sourceModifiedUnixNanos: previewBundle.sourceModifiedUnixNanos,
          })),
        ],
      },
    ],
  };
}

function previewPatternMutation(): PatternMutation {
  return {
    patterns: [...previewPatterns],
    configurationRevision: previewPatternRevision,
    indexes: [previewIndex()],
    failedRootIds: [],
  };
}

const previewApi: BackstageApi = {
  listRoots: async () => [previewRoot],
  chooseRoot: async () => null,
  approveRoot: async () => previewRoot,
  removeRoot: async () => ({ roots: [], indexes: [] }),
  listPatterns: async () => ({
    revision: previewPatternRevision,
    patterns: [...previewPatterns],
  }),
  addPattern: async (expression) => {
    previewPatternRevision += 1;
    previewPatterns = [
      ...previewPatterns,
      {
        id: `preview-custom-${previewPatternRevision}`,
        expression,
        ordinal: previewPatterns.length,
        provenance: "custom",
      },
    ];
    return previewPatternMutation();
  },
  removePattern: async (id) => {
    previewPatternRevision += 1;
    previewPatterns = previewPatterns.filter((pattern) => pattern.id !== id);
    return previewPatternMutation();
  },
  restoreDefaultPatterns: async () => {
    previewPatternRevision += 1;
    const ids = new Set(previewPatterns.map((pattern) => pattern.id));
    previewPatterns = [
      ...previewPatterns,
      ...previewDefaultPatterns.filter((pattern) => !ids.has(pattern.id)),
    ];
    return previewPatternMutation();
  },
  scanRoot: async () => ({
    projects: [previewProject],
    warnings: [],
    cancelled: false,
    entriesInspected: 184,
  }),
  cancelScan: async () => false,
  getIndex: async () => previewIndex(),
  getArtifactDetail: async (_rootId, artifactId) => {
    const selectedMember =
      previewMembers.find((member) => member.id === artifactId) ?? previewMembers[2]!;
    const markdownById: Record<string, string> = {
      "preview-proposal":
        "# Artifact control tower\n\n## Why\n\nDevelopers running many coding agents lose the durable plans that explain what each session was trying to accomplish.\n\n## What Changes\n\n- Discover planning artifacts across approved repositories\n- Group OpenSpec files into coherent changes\n- Preserve deterministic progress and safe handoffs",
      "preview-design":
        "# Design\n\n## Goals / Non-Goals\n\n**Goals:** Make durable work easy to understand and resume.\n\n**Non-Goals:** Recreate agent sessions or modify repositories.\n\n## Decisions\n\n### Keep facts local\n\nScan and parse locally; invoke Pi only after an explicit request.\n\n### Organize artifacts, not source trees\n\nShow planning work without becoming another IDE.\n\n## Risks / Trade-offs\n\n- Detector coverage begins narrowly and grows from real evidence.",
      "preview-artifact":
        "# Tasks\n\n## 1. Local foundation\n\n- [x] Discover Git working trees beneath approved roots\n- [x] Group recognized OpenSpec files into changes\n\n## 2. Safe reading\n\n- [x] Keep repository access strictly read-only\n- [x] Render deterministic task progress\n\n## 3. Delivery\n\n- [ ] Verify the packaged macOS smoke flow\n- [ ] Record supported syntax and data locations",
      "preview-spec":
        "# Artifact control tower specification\n\nThe system SHALL keep repository access read-only and expose deterministic planning facts.",
    };
    return {
      rootId: previewRoot.id,
      artifactId: selectedMember.id,
      bundleId: previewBundle.bundle.id,
      projectId: previewProject.id,
      projectName: previewProject.name,
      projectRoot: previewProject.rootPath,
      git: previewProject.git,
      bundleName: previewBundle.bundle.name,
      bundleKind: previewBundle.bundle.kind,
      recognition: previewBundle.bundle.recognition,
      custody: previewBundle.bundle.custody,
      primaryStatus: previewBundle.primaryStatus,
      members: previewBundle.bundle.members,
      relativePath: selectedMember.relativePath,
      absolutePath: `${previewProject.rootPath}/${selectedMember.relativePath}`,
      sourceModifiedUnixNanos: previewBundle.sourceModifiedUnixNanos,
      markdown: markdownById[selectedMember.id] ?? "# Source unavailable",
      progress: previewBundle.progress,
      fingerprint: previewBundle.fingerprint,
      warnings: [],
      openSpecView: {
        overview: [
          {
            kind: "why",
            sourcePath: previewMembers[0]!.relativePath,
            markdown:
              "Developers running many coding agents lose the durable plans that explain what each session was trying to accomplish.",
          },
          {
            kind: "what_changes",
            sourcePath: previewMembers[0]!.relativePath,
            markdown:
              "- Discover planning artifacts across approved repositories\n- Group OpenSpec files into coherent changes\n- Preserve deterministic progress and safe handoffs",
          },
          {
            kind: "goals_and_non_goals",
            sourcePath: previewMembers[1]!.relativePath,
            markdown:
              "**Goals:** Make durable work easy to understand and resume.\n\n**Non-Goals:** Recreate agent sessions or modify repositories.",
          },
          {
            kind: "decisions",
            sourcePath: previewMembers[1]!.relativePath,
            markdown:
              "### Keep facts local\n\nScan and parse locally; invoke Pi only after an explicit request.\n\n### Organize artifacts, not source trees\n\nShow planning work without becoming another IDE.",
          },
          {
            kind: "risks_and_trade_offs",
            sourcePath: previewMembers[1]!.relativePath,
            markdown: "- Detector coverage begins narrowly and grows from real evidence.",
          },
        ],
        taskGroups: [
          {
            title: "1. Local foundation",
            sourcePath: previewMembers[2]!.relativePath,
            tasks: previewTasks.slice(0, 2),
          },
          {
            title: "2. Safe reading",
            sourcePath: previewMembers[2]!.relativePath,
            tasks: previewTasks.slice(2, 4),
          },
          {
            title: "3. Delivery",
            sourcePath: previewMembers[2]!.relativePath,
            tasks: previewTasks.slice(4),
          },
        ],
      },
    };
  },
  getMarkdownDetail: async () => ({
    rootId: previewRoot.id,
    documentId: previewReadme.id,
    projectId: previewProject.id,
    projectName: previewProject.name,
    projectRoot: previewProject.rootPath,
    git: previewProject.git,
    relativePath: previewReadme.relativePath,
    absolutePath: `${previewProject.rootPath}/${previewReadme.relativePath}`,
    sourceModifiedUnixNanos: previewReadme.sourceModifiedUnixNanos,
    markdown:
      "# Atlas Workbench\n\nLocal tooling and architecture notes for the artifact control tower.",
  }),
  getGeneratedView: async () => ({
    status: "stale",
    result: {
      text: "Backstage now completes the core discovery and reading loop. The remaining work is release verification and durable documentation.",
      mode: "summary",
      sourceFingerprint: "sha256:previous",
      includedPaths: [previewBundle.bundle.members[0]!.relativePath],
      generatedAt: "2026-08-13T11:52:00Z",
      model: "openai-codex/gpt-5.6-sol",
      promptVersion: "summary-v1",
    },
    changedInputs: [previewBundle.bundle.members[0]!.relativePath],
  }),
  requestSummary: async () => ({ status: "never_generated" }),
  cancelSummary: async () => false,
  copyArtifactPath: async () => "preview path",
  copyMarkdownPath: async () => "preview Markdown path",
  copyContinuationPrompt: async () => "preview continuation prompt",
  openTerminal: async () => undefined,
};

export const runtimeApi: BackstageApi =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window ? tauriApi : previewApi;
