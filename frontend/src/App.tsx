import { ArrowClockwiseIcon } from "@phosphor-icons/react/dist/csr/ArrowClockwise";
import { ArchiveIcon } from "@phosphor-icons/react/dist/csr/Archive";
import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { CircleIcon } from "@phosphor-icons/react/dist/csr/Circle";
import { CommandIcon } from "@phosphor-icons/react/dist/csr/Command";
import { FilesIcon } from "@phosphor-icons/react/dist/csr/Files";
import { FolderSimpleIcon } from "@phosphor-icons/react/dist/csr/FolderSimple";
import { MagnifyingGlassIcon } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { SidebarSimpleIcon } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent, RefObject } from "react";

import backstageMark from "./assets/backstage-mark.svg";
import { runtimeApi } from "./api";
import {
  LEDGER_WIDTH_MAX,
  LEDGER_WIDTH_MIN,
  PROJECT_WIDTH_MAX,
  PROJECT_WIDTH_MIN,
  loadPaneLayout,
  normalizePaneLayout,
  savePaneLayout,
} from "./layout";
import { renderMarkdown } from "./markdown";
import type {
  ApprovedRoot,
  ArtifactDetail,
  BackstageApi,
  GeneratedView,
  IndexSnapshot,
  IndexedBundle,
  MarkdownDetail,
  MarkdownDocument,
  OpenSpecOverviewSection,
  OpenSpecTaskGroup,
  Project,
  ScanWarning,
} from "./api";

type WorkspaceStatus =
  "loading" | "no-root" | "scanning" | "ready" | "ready-with-warnings" | "unavailable";
type RegistryScope = "planning" | "markdown";
type IndexedMarkdownDocument = MarkdownDocument & { rootId: string };

const LEDGER_BATCH_SIZE = 200;

interface AppProps {
  api?: BackstageApi;
}

export function App({ api = runtimeApi }: AppProps) {
  const [roots, setRoots] = useState<ApprovedRoot[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [indexes, setIndexes] = useState<IndexSnapshot[]>([]);
  const [warnings, setWarnings] = useState<ScanWarning[]>([]);
  const [status, setStatus] = useState<WorkspaceStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState("all");
  const [registryScope, setRegistryScope] = useState<RegistryScope>("planning");
  const [bundleFilter, setBundleFilter] = useState<
    "all" | "unfinished" | "warning" | "stale" | "recent"
  >("all");
  const [ledgerLimit, setLedgerLimit] = useState(LEDGER_BATCH_SIZE);
  const [selectedArtifact, setSelectedArtifact] = useState<ArtifactDetail | null>(null);
  const [selectedMarkdown, setSelectedMarkdown] = useState<MarkdownDetail | null>(null);
  const selectedArtifactRef = useRef<ArtifactDetail | null>(null);
  const detailRequestRef = useRef(0);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [handoffNotice, setHandoffNotice] = useState<string | null>(null);
  const [generatedView, setGeneratedView] = useState<GeneratedView>({ status: "never_generated" });
  const [generatedInventory, setGeneratedInventory] = useState<Record<string, GeneratedView>>({});
  const [paneLayout, setPaneLayout] = useState(loadPaneLayout);
  const [searchQuery, setSearchQuery] = useState("");
  const [paletteQuery, setPaletteQuery] = useState("");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const paletteTriggerRef = useRef<HTMLButtonElement>(null);
  const paletteInputRef = useRef<HTMLInputElement>(null);
  const restorePaletteFocusRef = useRef(false);
  const resizeCleanupRef = useRef<(() => void) | null>(null);
  const readingDeskRef = useRef<HTMLElement>(null);

  const scan = useCallback(
    async (nextRoots: ApprovedRoot[]) => {
      if (nextRoots.length === 0) {
        setProjects([]);
        setWarnings([]);
        setStatus("no-root");
        return;
      }

      setStatus("scanning");
      setError(null);
      try {
        const cachedIndexes = (
          await Promise.all(nextRoots.map((root) => api.getIndex(root.id)))
        ).filter((index): index is IndexSnapshot => index !== null);
        if (cachedIndexes.length > 0) {
          setIndexes(cachedIndexes);
          setProjects(cachedIndexes.flatMap((index) => index.projects.map((item) => item.project)));
        }
        const results = await Promise.all(nextRoots.map((root) => api.scanRoot(root.id)));
        const nextProjects = results.flatMap((result) => result.projects);
        const nextWarnings = results.flatMap((result) => result.warnings);
        const nextIndexes = (
          await Promise.all(nextRoots.map((root) => api.getIndex(root.id)))
        ).filter((index): index is IndexSnapshot => index !== null);
        const bundleOwners = new Map<string, { rootId: string; bundle: IndexedBundle }>();
        for (const index of [...nextIndexes].sort((left, right) =>
          left.rootId.localeCompare(right.rootId),
        )) {
          for (const project of index.projects) {
            for (const bundle of project.bundles) {
              if (!bundleOwners.has(bundle.bundle.id)) {
                bundleOwners.set(bundle.bundle.id, { rootId: index.rootId, bundle });
              }
            }
          }
        }
        const generatedEntries = await Promise.all(
          [...bundleOwners.values()].map(
            async ({ rootId, bundle }) =>
              [bundle.bundle.id, await api.getGeneratedView(rootId, bundle.bundle.id)] as const,
          ),
        );
        setProjects(nextProjects);
        setIndexes(nextIndexes);
        setGeneratedInventory(Object.fromEntries(generatedEntries));
        setWarnings(nextWarnings);
        const projectIdsWithWork = new Set(
          nextIndexes.flatMap((index) =>
            index.projects
              .filter((project) =>
                project.bundles.some((bundle) => bundle.bundle.members.length > 0),
              )
              .map((project) => project.project.id),
          ),
        );
        setSelectedProjectId((current) =>
          current === "all" || projectIdsWithWork.has(current) ? current : "all",
        );
        setStatus(nextWarnings.length > 0 ? "ready-with-warnings" : "ready");
      } catch (cause) {
        setError(errorMessage(cause));
        setStatus("unavailable");
      }
    },
    [api],
  );

  useEffect(() => {
    let active = true;
    void api
      .listRoots()
      .then(async (nextRoots) => {
        if (!active) return;
        setRoots(nextRoots);
        await scan(nextRoots);
      })
      .catch((cause: unknown) => {
        if (!active) return;
        setError(errorMessage(cause));
        setStatus("unavailable");
      });
    return () => {
      active = false;
    };
  }, [api, scan]);

  const approveRoot = async () => {
    try {
      const path = await api.chooseRoot();
      if (!path) return;
      setStatus("scanning");
      setError(null);
      const root = await api.approveRoot(path);
      const nextRoots = roots.some((candidate) => candidate.id === root.id)
        ? roots
        : [...roots, root];
      setRoots(nextRoots);
      await scan(nextRoots);
    } catch (cause) {
      setError(errorMessage(cause));
      setStatus(roots.length > 0 ? "unavailable" : "no-root");
    }
  };

  const projectFileCounts = useMemo(() => {
    const memberIds = new Map<string, Set<string>>();
    for (const index of indexes) {
      for (const indexedProject of index.projects) {
        const projectMembers = memberIds.get(indexedProject.project.id) ?? new Set<string>();
        for (const bundle of indexedProject.bundles) {
          for (const member of bundle.bundle.members) projectMembers.add(member.id);
        }
        if (registryScope === "markdown") {
          for (const document of indexedProject.markdownDocuments) projectMembers.add(document.id);
        }
        memberIds.set(indexedProject.project.id, projectMembers);
      }
    }
    return new Map([...memberIds].map(([projectId, members]) => [projectId, members.size]));
  }, [indexes, registryScope]);

  const workProjects = useMemo(() => {
    const unique = new Map<string, Project>();
    for (const project of projects) {
      if ((projectFileCounts.get(project.id) ?? 0) > 0 && !unique.has(project.id)) {
        unique.set(project.id, project);
      }
    }
    return [...unique.values()];
  }, [projectFileCounts, projects]);

  const filteredProjects = useMemo(
    () =>
      selectedProjectId === "all"
        ? workProjects
        : workProjects.filter((project) => project.id === selectedProjectId),
    [selectedProjectId, workProjects],
  );

  useEffect(() => {
    if (
      selectedProjectId !== "all" &&
      !workProjects.some((project) => project.id === selectedProjectId)
    ) {
      setSelectedProjectId("all");
    }
  }, [selectedProjectId, workProjects]);

  const visibleBundles = useMemo(() => {
    const unique = new Map<string, IndexedBundle>();
    for (const bundle of indexes.flatMap((index) =>
      index.projects.flatMap((project) =>
        project.bundles.filter(
          (bundle) => selectedProjectId === "all" || bundle.bundle.projectId === selectedProjectId,
        ),
      ),
    )) {
      if (!unique.has(bundle.bundle.id)) unique.set(bundle.bundle.id, bundle);
    }
    const bundles = [...unique.values()];
    const filtered =
      bundleFilter === "unfinished"
        ? bundles.filter(
            (bundle) =>
              bundle.progress.status === "available" && bundle.progress.progress.remainingCount > 0,
          )
        : bundleFilter === "warning"
          ? bundles.filter(
              (bundle) =>
                bundle.warnings.length > 0 || bundle.progress.progress.warnings.length > 0,
            )
          : bundleFilter === "stale"
            ? bundles.filter((bundle) => generatedInventory[bundle.bundle.id]?.status === "stale")
            : bundleFilter === "recent"
              ? recentBundles(bundles)
              : bundles;
    const query = searchQuery.trim().toLowerCase();
    return query
      ? filtered.filter((bundle) =>
          [
            bundle.bundle.name,
            bundle.bundle.projectName,
            ...bundle.bundle.members.map((member) => member.relativePath),
          ]
            .join(" ")
            .toLowerCase()
            .includes(query),
        )
      : filtered;
  }, [bundleFilter, generatedInventory, indexes, searchQuery, selectedProjectId]);

  const visibleDocuments = useMemo(() => {
    if (registryScope !== "markdown" || bundleFilter !== "all") return [];
    const query = searchQuery.trim().toLowerCase();
    const documents = indexes
      .flatMap((index) =>
        index.projects.flatMap((project) => {
          if (selectedProjectId !== "all" && project.project.id !== selectedProjectId) return [];
          const represented = new Set(
            project.bundles.flatMap((bundle) => bundle.bundle.members.map((member) => member.id)),
          );
          return project.markdownDocuments
            .filter((document) => !represented.has(document.id))
            .map((document) => ({ ...document, rootId: index.rootId }));
        }),
      )
      .sort(
        (left, right) => left.id.localeCompare(right.id) || left.rootId.localeCompare(right.rootId),
      );
    const unique = new Map<string, IndexedMarkdownDocument>();
    for (const document of documents) {
      if (!unique.has(document.id)) unique.set(document.id, document);
    }
    return [...unique.values()]
      .filter(
        (document) =>
          !query ||
          [document.relativePath, document.projectName].join(" ").toLowerCase().includes(query),
      )
      .sort(
        (left, right) =>
          left.relativePath.toLowerCase().localeCompare(right.relativePath.toLowerCase()) ||
          left.id.localeCompare(right.id),
      );
  }, [bundleFilter, indexes, registryScope, searchQuery, selectedProjectId]);

  const visibleRecordCount = visibleBundles.length + visibleDocuments.length;
  const visibleFileCount = new Set([
    ...visibleBundles.flatMap((bundle) => bundle.bundle.members.map((member) => member.id)),
    ...visibleDocuments.map((document) => document.id),
  ]).size;
  const displayedBundles = visibleBundles.slice(0, ledgerLimit);
  const displayedDocuments = visibleDocuments.slice(
    0,
    Math.max(0, ledgerLimit - displayedBundles.length),
  );
  const remainingRecordCount = Math.max(
    0,
    visibleRecordCount - displayedBundles.length - displayedDocuments.length,
  );

  useEffect(
    () => setLedgerLimit(LEDGER_BATCH_SIZE),
    [bundleFilter, indexes, registryScope, searchQuery, selectedProjectId],
  );

  const commitSelectedArtifact = (detail: ArtifactDetail) => {
    selectedArtifactRef.current = detail;
    setSelectedMarkdown(null);
    setSelectedArtifact(detail);
  };

  const commitSelectedMarkdown = (detail: MarkdownDetail) => {
    selectedArtifactRef.current = null;
    setSelectedArtifact(null);
    setSelectedMarkdown(detail);
    setGeneratedView({ status: "never_generated" });
  };

  const selectBundle = async (bundle: IndexedBundle) => {
    const root = [...indexes]
      .sort((left, right) => left.rootId.localeCompare(right.rootId))
      .find((index) =>
        index.projects.some((project) =>
          project.bundles.some((candidate) => candidate.bundle.id === bundle.bundle.id),
        ),
      );
    const member =
      bundle.bundle.members.find((candidate) => candidate.relativePath.endsWith("tasks.md")) ??
      bundle.bundle.members[0];
    if (!root || !member) return;
    const requestId = ++detailRequestRef.current;
    try {
      setDetailError(null);
      const [detail, generated] = await Promise.all([
        api.getArtifactDetail(root.rootId, member.id),
        api.getGeneratedView(root.rootId, bundle.bundle.id),
      ]);
      if (requestId !== detailRequestRef.current) return;
      commitSelectedArtifact(detail);
      setGeneratedView(generated);
      setGeneratedInventory((inventory) => ({ ...inventory, [bundle.bundle.id]: generated }));
      if (window.innerWidth <= 960) {
        setPaneLayout((layout) => ({ ...layout, ledgerCollapsed: true }));
      }
    } catch (cause) {
      if (requestId === detailRequestRef.current) setDetailError(errorMessage(cause));
    }
  };

  const selectDocument = async (document: IndexedMarkdownDocument) => {
    const requestId = ++detailRequestRef.current;
    try {
      setDetailError(null);
      const detail = await api.getMarkdownDetail(document.rootId, document.id);
      if (requestId !== detailRequestRef.current) return;
      commitSelectedMarkdown(detail);
      if (window.innerWidth <= 960) {
        setPaneLayout((layout) => ({ ...layout, ledgerCollapsed: true }));
      }
    } catch (cause) {
      if (requestId === detailRequestRef.current) setDetailError(errorMessage(cause));
    }
  };

  const changeRegistryScope = (nextScope: RegistryScope) => {
    if (nextScope === registryScope) return;
    setRegistryScope(nextScope);
    setBundleFilter("all");
    if (nextScope !== "planning") return;

    ++detailRequestRef.current;
    setSelectedMarkdown(null);
    if (selectedArtifact) return;
    const projectBundle = indexes
      .flatMap((index) => index.projects)
      .filter((project) => selectedProjectId === "all" || project.project.id === selectedProjectId)
      .flatMap((project) => project.bundles)[0];
    const fallbackBundle =
      projectBundle ??
      indexes.flatMap((index) => index.projects.flatMap((project) => project.bundles))[0];
    if (fallbackBundle) void selectBundle(fallbackBundle);
  };

  const selectedRoot = selectedArtifact
    ? indexes.find((index) => index.rootId === selectedArtifact.rootId)
    : undefined;

  const runHandoff = async (action: "path" | "prompt" | "terminal") => {
    if (!selectedArtifact || !selectedRoot) return;
    try {
      setDetailError(null);
      if (action === "path") {
        await api.copyArtifactPath(selectedRoot.rootId, selectedArtifact.artifactId);
        setHandoffNotice("Artifact path copied");
      } else if (action === "prompt") {
        await api.copyContinuationPrompt(selectedRoot.rootId, selectedArtifact.artifactId);
        setHandoffNotice("Continuation prompt copied");
      } else {
        await api.openTerminal(selectedRoot.rootId, selectedArtifact.projectId);
        setHandoffNotice("Terminal opened at the project root");
      }
    } catch (cause) {
      setDetailError(errorMessage(cause));
    }
  };

  const copyMarkdownPath = async () => {
    if (!selectedMarkdown) return;
    try {
      setDetailError(null);
      setHandoffNotice(null);
      await api.copyMarkdownPath(selectedMarkdown.rootId, selectedMarkdown.documentId);
      setHandoffNotice("Markdown path copied");
    } catch (cause) {
      setDetailError(errorMessage(cause));
    }
  };

  const requestSummary = async () => {
    if (!selectedArtifact) return;
    const requestedBundleId = selectedArtifact.bundleId;
    const root = indexes.find((index) => index.rootId === selectedArtifact.rootId);
    if (!root) return;
    const previous = generatedResult(generatedView);
    setGeneratedView({ status: "generating", ...(previous ? { previous } : {}) });
    try {
      const nextView = await api.requestSummary(root.rootId, requestedBundleId);
      if (selectedArtifactRef.current?.bundleId === requestedBundleId) setGeneratedView(nextView);
      setGeneratedInventory((inventory) => ({
        ...inventory,
        [requestedBundleId]: nextView,
      }));
    } catch (cause) {
      const failed: GeneratedView = {
        status: "failed",
        ...(previous ? { previous } : {}),
        failure: errorMessage(cause),
      };
      if (selectedArtifactRef.current?.bundleId === requestedBundleId) setGeneratedView(failed);
      setGeneratedInventory((inventory) => ({
        ...inventory,
        [requestedBundleId]: failed,
      }));
    }
  };

  useEffect(() => savePaneLayout(paneLayout), [paneLayout]);

  const selectedReadingId = selectedMarkdown?.documentId ?? selectedArtifact?.bundleId;
  useEffect(() => {
    if (selectedReadingId) readingDeskRef.current?.focus();
  }, [selectedReadingId]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const modified = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();
      if (modified && key === "k") {
        event.preventDefault();
        setPaletteOpen(true);
        return;
      }
      if (modified && key === "r" && !isEditableTarget(event.target)) {
        event.preventDefault();
        if (roots.length > 0 && status !== "scanning") void scan(roots);
        return;
      }
      if (paletteOpen) {
        if (event.altKey && ["1", "2", "3"].includes(event.key)) event.preventDefault();
        return;
      }
      if (modified && key === "f") {
        event.preventDefault();
        document.getElementById("global-search")?.focus();
        return;
      }
      if (event.altKey && ["1", "2", "3"].includes(event.key)) {
        event.preventDefault();
        document.querySelector<HTMLElement>(`[data-pane="${event.key}"]`)?.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [paletteOpen, roots, scan, status]);

  useEffect(() => {
    if (paletteOpen) {
      setPaletteQuery("");
      paletteInputRef.current?.focus();
    }
  }, [paletteOpen]);

  useEffect(() => {
    if (!paletteOpen && restorePaletteFocusRef.current) {
      restorePaletteFocusRef.current = false;
      paletteTriggerRef.current?.focus();
    }
  }, [paletteOpen]);

  const closePalette = (restoreFocus = true) => {
    restorePaletteFocusRef.current = restoreFocus;
    setPaletteOpen(false);
  };

  const selectArtifactMember = async (artifactId: string) => {
    if (!selectedArtifact) return;
    const requestId = ++detailRequestRef.current;
    try {
      setDetailError(null);
      const detail = await api.getArtifactDetail(selectedArtifact.rootId, artifactId);
      if (requestId !== detailRequestRef.current) return;
      commitSelectedArtifact(detail);
    } catch (cause) {
      if (requestId === detailRequestRef.current) setDetailError(errorMessage(cause));
    }
  };

  const resizePaneFromKeyboard = (
    pane: "project" | "ledger",
    event: ReactKeyboardEvent<HTMLDivElement>,
  ) => {
    const current = pane === "project" ? paneLayout.projectWidth : paneLayout.ledgerWidth;
    const minimum = pane === "project" ? PROJECT_WIDTH_MIN : LEDGER_WIDTH_MIN;
    const maximum = pane === "project" ? PROJECT_WIDTH_MAX : LEDGER_WIDTH_MAX;
    const step = event.shiftKey ? 24 : 8;
    const next =
      event.key === "Home"
        ? minimum
        : event.key === "End"
          ? maximum
          : event.key === "ArrowLeft"
            ? current - step
            : event.key === "ArrowRight"
              ? current + step
              : null;
    if (next === null) return;
    event.preventDefault();
    setPaneLayout((layout) =>
      normalizePaneLayout({
        ...layout,
        ...(pane === "project" ? { projectWidth: next } : { ledgerWidth: next }),
      }),
    );
  };

  const startResize = (pane: "project" | "ledger", startX: number) => {
    resizeCleanupRef.current?.();
    const initial = paneLayout;
    let latestX = startX;
    let frame: number | null = null;
    const apply = () => {
      frame = null;
      const delta = latestX - startX;
      setPaneLayout(
        normalizePaneLayout({
          ...initial,
          ...(pane === "project"
            ? { projectWidth: initial.projectWidth + delta }
            : { ledgerWidth: initial.ledgerWidth + delta }),
        }),
      );
    };
    const move = (event: PointerEvent) => {
      latestX = event.clientX;
      if (frame === null) frame = window.requestAnimationFrame(apply);
    };
    const cleanup = () => {
      if (frame !== null) {
        window.cancelAnimationFrame(frame);
        frame = null;
      }
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", cleanup);
      if (resizeCleanupRef.current === cleanup) resizeCleanupRef.current = null;
    };
    const stop = (event: PointerEvent) => {
      latestX = event.clientX;
      if (frame !== null) {
        window.cancelAnimationFrame(frame);
        frame = null;
      }
      apply();
      cleanup();
    };
    resizeCleanupRef.current = cleanup;
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", cleanup);
  };

  useEffect(() => () => resizeCleanupRef.current?.(), []);

  const statusLabel = workspaceStatusLabel(status, warnings.length);
  const hasSelection = selectedArtifact !== null || selectedMarkdown !== null;
  const ledgerCollapsed = hasSelection && paneLayout.ledgerCollapsed;
  const ledgerToggleLabel = !hasSelection
    ? "Bundle ledger remains open until work is selected"
    : ledgerCollapsed
      ? "Show bundle ledger"
      : "Hide bundle ledger";

  return (
    <main className="app-frame">
      <header className="titlebar" inert={paletteOpen}>
        <div className="wordmark" aria-label="Backstage artifact control tower">
          <img className="brand-mark" src={backstageMark} alt="" />
          <strong>BACKSTAGE</strong>
          <span>Artifact Control Tower</span>
        </div>
        <div className="titlebar-actions">
          <label className="global-search">
            <MagnifyingGlassIcon className="app-icon" aria-hidden="true" weight="regular" />
            <input
              id="global-search"
              type="search"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="Search work"
              aria-label="Search all indexed work"
            />
            <kbd>⌘F</kbd>
          </label>
          <span className={`system-state system-state--${status}`}>{statusLabel}</span>
          <button
            ref={paletteTriggerRef}
            className="icon-button"
            type="button"
            aria-label="Open command palette"
            title="Open command palette"
            onClick={() => setPaletteOpen(true)}
          >
            <CommandIcon className="app-icon" aria-hidden="true" weight="regular" />
          </button>
          <button
            className="icon-button"
            type="button"
            onClick={() => void scan(roots)}
            disabled={roots.length === 0 || status === "scanning"}
            aria-label="Refresh approved roots"
            title="Refresh approved roots"
          >
            <ArrowClockwiseIcon className="app-icon" aria-hidden="true" weight="regular" />
          </button>
          <button
            className="icon-button ledger-toggle"
            type="button"
            aria-pressed={ledgerCollapsed}
            aria-label={ledgerToggleLabel}
            title={ledgerToggleLabel}
            disabled={!hasSelection}
            onClick={() =>
              setPaneLayout((layout) => ({ ...layout, ledgerCollapsed: !ledgerCollapsed }))
            }
          >
            <SidebarSimpleIcon className="app-icon" aria-hidden="true" weight="regular" />
          </button>
          <button
            className="button button--primary button--compact"
            type="button"
            onClick={() => void approveRoot()}
          >
            Add root
          </button>
        </div>
      </header>

      <section
        className={`workspace ${ledgerCollapsed ? "ledger-is-collapsed" : ""}`}
        aria-label="Artifact workspace"
        inert={paletteOpen}
        style={
          {
            "--project-width": `${paneLayout.projectWidth}px`,
            "--ledger-width": `${paneLayout.ledgerWidth}px`,
          } as CSSProperties
        }
      >
        <aside
          id="project-registry"
          className="project-rail"
          aria-label="Project registry"
          data-pane="1"
          tabIndex={0}
        >
          <div className="pane-heading">
            <span>Project registry</span>
            <span className="registry-count">{workProjects.length}</span>
          </div>
          <div className="registry-scope" aria-label="Registry scope">
            <button
              type="button"
              aria-pressed={registryScope === "planning"}
              className={registryScope === "planning" ? "is-selected" : ""}
              onClick={() => changeRegistryScope("planning")}
              aria-label="Plan files"
            >
              <span className="registry-scope-full">Plan files</span>
              <span className="registry-scope-compact" aria-hidden="true">
                Plan
              </span>
            </button>
            <button
              type="button"
              aria-pressed={registryScope === "markdown"}
              className={registryScope === "markdown" ? "is-selected" : ""}
              onClick={() => changeRegistryScope("markdown")}
              aria-label="All Markdown"
            >
              <span className="registry-scope-full">All Markdown</span>
              <span className="registry-scope-compact" aria-hidden="true">
                MD
              </span>
            </button>
          </div>
          <nav className="project-list" aria-label="Project filters">
            <button
              className={`project-row ${selectedProjectId === "all" ? "is-selected" : ""}`}
              type="button"
              aria-pressed={selectedProjectId === "all"}
              aria-label={`All Work, ${workProjects.length} ${workProjects.length === 1 ? "project" : "projects"}`}
              title="All Work"
              onClick={() => setSelectedProjectId("all")}
            >
              <FilesIcon className="app-icon" aria-hidden="true" weight="regular" />
              <span className="project-row-copy">
                <strong>All Work</strong>
                <small>
                  {workProjects.length === 0
                    ? "Awaiting accession"
                    : `${workProjects.length} ${workProjects.length === 1 ? "project" : "projects"}`}
                </small>
              </span>
              <span className="project-compact-identity" aria-hidden="true">
                <strong>ALL</strong>
                <small>{workProjects.length}</small>
              </span>
            </button>
            {workProjects.map((project) => {
              const fileCount = projectFileCounts.get(project.id) ?? 0;
              const fileCountLabel =
                registryScope === "planning"
                  ? planningFileCountLabel(fileCount)
                  : markdownFileCountLabel(fileCount);
              const projectLabel = `${project.name}, ${project.git?.branch ?? "Git unavailable"}, ${fileCountLabel}`;
              return (
                <button
                  className={`project-row ${selectedProjectId === project.id ? "is-selected" : ""}`}
                  type="button"
                  key={project.id}
                  aria-pressed={selectedProjectId === project.id}
                  aria-label={projectLabel}
                  title={projectLabel}
                  onClick={() => setSelectedProjectId(project.id)}
                >
                  <FolderSimpleIcon className="app-icon" aria-hidden="true" weight="regular" />
                  <span className="project-row-copy">
                    <strong>{project.name}</strong>
                    <span className="project-row-meta">
                      <small>{project.git?.branch ?? "Git unavailable"}</small>
                      <small className="project-file-count">
                        {fileCount} {fileCount === 1 ? "file" : "files"}
                      </small>
                    </span>
                  </span>
                  <span className="project-compact-identity" aria-hidden="true">
                    <strong>{compactProjectLabel(project.name)}</strong>
                    <small>{fileCount}</small>
                  </span>
                </button>
              );
            })}
          </nav>
          <div className="root-registry">
            <span>Approved roots</span>
            {roots.length === 0 ? (
              <p>No approved roots</p>
            ) : (
              roots.map((root) => <code key={root.id}>{shortPath(root.path)}</code>)
            )}
          </div>
        </aside>
        <div
          className="pane-resizer pane-resizer--project"
          role="separator"
          aria-label="Resize project registry"
          aria-orientation="vertical"
          aria-controls="project-registry"
          aria-valuemin={PROJECT_WIDTH_MIN}
          aria-valuemax={PROJECT_WIDTH_MAX}
          aria-valuenow={paneLayout.projectWidth}
          aria-valuetext={`${paneLayout.projectWidth} pixels`}
          tabIndex={0}
          onKeyDown={(event) => resizePaneFromKeyboard("project", event)}
          onPointerDown={(event) => startResize("project", event.clientX)}
        />

        <section
          id="bundle-ledger"
          className="bundle-ledger"
          aria-label="Bundle ledger"
          data-pane="2"
          tabIndex={0}
        >
          <div className="ledger-toolbar">
            <div>
              <span className="pane-heading-label">
                {registryScope === "planning" ? "Bundle ledger" : "Markdown ledger"}
              </span>
              <strong>
                {selectedProjectId === "all" ? "All Work" : filteredProjects[0]?.name}
              </strong>
            </div>
            <span className="ledger-count">
              <span>
                {visibleRecordCount} {visibleRecordCount === 1 ? "record" : "records"}
              </span>
              <span aria-hidden="true">·</span>
              <span>
                {visibleFileCount} {visibleFileCount === 1 ? "file" : "files"}
              </span>
            </span>
          </div>
          <div className="ledger-filters" aria-label="Deterministic state filters">
            {registryScope === "markdown" && (
              <span className="ledger-filter-label">Planning filters</span>
            )}
            <button
              type="button"
              className={bundleFilter === "all" ? "is-selected" : ""}
              aria-pressed={bundleFilter === "all"}
              onClick={() => setBundleFilter("all")}
            >
              All
            </button>
            <button
              type="button"
              className={bundleFilter === "unfinished" ? "is-selected" : ""}
              aria-pressed={bundleFilter === "unfinished"}
              onClick={() => setBundleFilter("unfinished")}
            >
              Unfinished
            </button>
            <button
              type="button"
              className={bundleFilter === "warning" ? "is-selected" : ""}
              aria-pressed={bundleFilter === "warning"}
              onClick={() => setBundleFilter("warning")}
            >
              Warning-bearing
            </button>
            <button
              type="button"
              aria-pressed={bundleFilter === "stale"}
              className={bundleFilter === "stale" ? "is-selected" : ""}
              onClick={() => setBundleFilter("stale")}
            >
              Stale
            </button>
            <button
              type="button"
              aria-pressed={bundleFilter === "recent"}
              className={bundleFilter === "recent" ? "is-selected" : ""}
              onClick={() => setBundleFilter("recent")}
            >
              Recently changed
            </button>
          </div>
          {status === "loading" && indexes.length === 0 ? (
            <ScanSkeleton />
          ) : visibleRecordCount > 0 ? (
            <div className="bundle-list">
              {status === "scanning" && (
                <p className="refresh-indicator" role="status">
                  Refreshing in the background · prior index remains usable
                </p>
              )}
              {displayedBundles.map((bundle) => (
                <BundleRow
                  key={bundle.bundle.id}
                  bundle={bundle}
                  selected={selectedArtifact?.bundleId === bundle.bundle.id}
                  onSelect={() => void selectBundle(bundle)}
                />
              ))}
              {displayedDocuments.map((document) => (
                <DocumentRow
                  key={document.id}
                  document={document}
                  selected={selectedMarkdown?.documentId === document.id}
                  onSelect={() => void selectDocument(document)}
                />
              ))}
              {remainingRecordCount > 0 && (
                <button
                  className="ledger-load-more"
                  type="button"
                  onClick={() => setLedgerLimit((limit) => limit + LEDGER_BATCH_SIZE)}
                >
                  Show {Math.min(LEDGER_BATCH_SIZE, remainingRecordCount)} more records
                </button>
              )}
            </div>
          ) : (
            <div className="ledger-empty">
              <ArchiveIcon className="app-icon" aria-hidden="true" weight="regular" />
              <strong>{workProjects.length === 0 ? "No entries yet" : "No records match"}</strong>
              <p>
                {workProjects.length === 0
                  ? "Approve or refresh a root to populate this ledger."
                  : "Change the deterministic filter or refresh the approved root."}
              </p>
            </div>
          )}
        </section>
        <div
          className="pane-resizer pane-resizer--ledger"
          role="separator"
          aria-label="Resize bundle ledger"
          aria-orientation="vertical"
          aria-controls="bundle-ledger"
          aria-valuemin={LEDGER_WIDTH_MIN}
          aria-valuemax={LEDGER_WIDTH_MAX}
          aria-valuenow={paneLayout.ledgerWidth}
          aria-valuetext={`${paneLayout.ledgerWidth} pixels`}
          tabIndex={0}
          onKeyDown={(event) => resizePaneFromKeyboard("ledger", event)}
          onPointerDown={(event) => startResize("ledger", event.clientX)}
        />

        <section
          ref={readingDeskRef}
          className="reading-desk"
          aria-label="Reading desk"
          data-pane="3"
          tabIndex={0}
        >
          {selectedArtifact ? (
            <ArtifactReadingDesk
              detail={selectedArtifact}
              generatedView={generatedView}
              onSelectMember={selectArtifactMember}
              onRequestSummary={requestSummary}
              onCopyPath={() => runHandoff("path")}
              onCopyPrompt={() => runHandoff("prompt")}
              onOpenTerminal={() => runHandoff("terminal")}
            />
          ) : selectedMarkdown ? (
            <MarkdownReadingDesk detail={selectedMarkdown} onCopyPath={copyMarkdownPath} />
          ) : (
            <WorkspaceContent
              status={status}
              roots={roots}
              projects={filteredProjects}
              scope={registryScope}
              warnings={warnings}
              error={error}
              onApprove={approveRoot}
              onRefresh={() => scan(roots)}
            />
          )}
          {handoffNotice && (
            <p className="handoff-notice" role="status">
              {handoffNotice}
            </p>
          )}
          {detailError && (
            <p className="detail-error" role="alert">
              {detailError}
            </p>
          )}
        </section>
      </section>
      {paletteOpen && (
        <CommandPalette
          inputRef={paletteInputRef}
          query={paletteQuery}
          onQueryChange={setPaletteQuery}
          onClose={closePalette}
          onRefresh={() => void scan(roots)}
          onApprove={() => void approveRoot()}
          onToggleLedger={() =>
            setPaneLayout((layout) => ({ ...layout, ledgerCollapsed: !ledgerCollapsed }))
          }
          canToggleLedger={hasSelection}
          canRefresh={roots.length > 0 && status !== "scanning"}
        />
      )}
    </main>
  );
}

interface WorkspaceContentProps {
  status: WorkspaceStatus;
  roots: ApprovedRoot[];
  projects: Project[];
  scope: RegistryScope;
  warnings: ScanWarning[];
  error: string | null;
  onApprove: () => Promise<void>;
  onRefresh: () => Promise<void>;
}

function WorkspaceContent({
  status,
  roots,
  projects,
  scope,
  warnings,
  error,
  onApprove,
  onRefresh,
}: WorkspaceContentProps) {
  if (status === "loading" || status === "scanning") {
    return (
      <div className="desk-state" aria-live="polite">
        <RegistryStamp label="READ ONLY" />
        <h1>
          {status === "loading" ? "Opening the local registry" : "Cataloguing approved roots"}
        </h1>
        <p>Repository bytes stay untouched while Backstage inspects project boundaries.</p>
        <div className="scan-progress" aria-label="Scan in progress">
          <span />
        </div>
      </div>
    );
  }

  if (roots.length === 0) {
    return (
      <div className="first-run">
        <div className="first-run-copy">
          <RegistryStamp label="LOCAL · READ ONLY" />
          <h1>Choose where Backstage can look</h1>
          <p>
            Approve a parent folder. Backstage will find Git projects and durable planning work
            beneath it without writing inside any repository.
          </p>
          <button className="button button--primary" type="button" onClick={() => void onApprove()}>
            Approve a root
          </button>
          {error && (
            <p className="error-message" role="alert">
              {error}
            </p>
          )}
        </div>
        <div className="custody-note" aria-label="Read-only custody policy">
          <span className="custody-rule" />
          <strong>Custody policy</strong>
          <dl>
            <div>
              <dt>Repository access</dt>
              <dd>Read-only</dd>
            </div>
            <div>
              <dt>Index location</dt>
              <dd>App-owned storage</dd>
            </div>
            <div>
              <dt>Pi invocation</dt>
              <dd>Explicit only</dd>
            </div>
          </dl>
        </div>
      </div>
    );
  }

  if (status === "unavailable") {
    return (
      <div className="desk-state">
        <RegistryStamp label="UNAVAILABLE" tone="warning" />
        <h1>The approved root could not be refreshed</h1>
        <p>
          {error ??
            "The root may have moved or become unreadable. Prior indexed work remains available when present."}
        </p>
        <button className="button" type="button" onClick={() => void onRefresh()}>
          Try refresh again
        </button>
      </div>
    );
  }

  if (projects.length === 0) {
    return (
      <div className="desk-state">
        <RegistryStamp label="SCAN COMPLETE" />
        <h1>{scope === "planning" ? "No planning work found" : "No Markdown files found"}</h1>
        <p>
          Backstage inspected {roots.length} approved {roots.length === 1 ? "root" : "roots"}.{" "}
          {scope === "planning"
            ? "Switch to All Markdown, add another root, or refresh after planning work is added."
            : "Add another root or refresh after Markdown files are added."}
        </p>
        <div className="button-row">
          <button className="button" type="button" onClick={() => void onRefresh()}>
            Refresh
          </button>
          <button className="button button--primary" type="button" onClick={() => void onApprove()}>
            Approve another root
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="desk-state">
      <RegistryStamp label="PROJECTS INDEXED" />
      <h1>
        {scope === "planning" ? "Select a bundle from the ledger" : "Select a file or bundle"}
      </h1>
      <p>
        {projects.length} {projects.length === 1 ? "project" : "projects"} with{" "}
        {scope === "planning" ? "planning work" : "Markdown"}
        {projects.length === 1 ? " is" : " are"} in scope.
      </p>
      {warnings.length > 0 && (
        <section className="warning-sheet" aria-label="Scan warnings">
          <strong>
            Ready with {warnings.length} {warnings.length === 1 ? "warning" : "warnings"}
          </strong>
          {warnings.slice(0, 3).map((warning) => (
            <p key={`${warning.code}:${warning.path}`}>{warning.message}</p>
          ))}
        </section>
      )}
    </div>
  );
}

function CommandPalette({
  inputRef,
  query,
  onQueryChange,
  onClose,
  onRefresh,
  onApprove,
  onToggleLedger,
  canToggleLedger,
  canRefresh,
}: {
  inputRef: RefObject<HTMLInputElement | null>;
  query: string;
  onQueryChange: (query: string) => void;
  onClose: (restoreFocus?: boolean) => void;
  onRefresh: () => void;
  onApprove: () => void;
  onToggleLedger: () => void;
  canToggleLedger: boolean;
  canRefresh: boolean;
}) {
  const commands = [
    {
      label: "Search indexed work",
      hint: "⌘F",
      disabled: false,
      run: () => document.getElementById("global-search")?.focus(),
    },
    { label: "Refresh approved roots", hint: "⌘R", disabled: !canRefresh, run: onRefresh },
    { label: "Approve another root", hint: "", disabled: false, run: onApprove },
    {
      label: "Toggle bundle ledger",
      hint: "",
      disabled: !canToggleLedger,
      run: onToggleLedger,
    },
  ].filter((command) => command.label.toLowerCase().includes(query.trim().toLowerCase()));
  return (
    <div
      className="palette-backdrop"
      role="presentation"
      onMouseDown={(event) => event.target === event.currentTarget && onClose()}
    >
      <section
        className="command-palette"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            onClose();
            return;
          }
          if (event.key !== "Tab") return;
          const focusable = Array.from(
            event.currentTarget.querySelectorAll<HTMLElement>(
              'input, button:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
            ),
          );
          const first = focusable[0];
          const last = focusable.at(-1);
          if (!first || !last) return;
          if (event.shiftKey && document.activeElement === first) {
            event.preventDefault();
            last.focus();
          } else if (!event.shiftKey && document.activeElement === last) {
            event.preventDefault();
            first.focus();
          }
        }}
      >
        <label>
          <MagnifyingGlassIcon className="app-icon" aria-hidden="true" weight="regular" />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="Type a command"
            aria-label="Search commands"
          />
          <kbd>Esc</kbd>
        </label>
        <div className="command-list">
          {commands.map((command) => (
            <button
              key={command.label}
              type="button"
              disabled={command.disabled}
              onClick={() => {
                onClose(command.label !== "Search indexed work");
                if (command.label === "Search indexed work") {
                  requestAnimationFrame(command.run);
                } else {
                  command.run();
                }
              }}
            >
              <span>{command.label}</span>
              <kbd>{command.hint}</kbd>
            </button>
          ))}
          {commands.length === 0 && <p>No valid commands match.</p>}
        </div>
      </section>
    </div>
  );
}

type OpenSpecReaderMode = "overview" | "tasks" | "source";

function MarkdownReadingDesk({
  detail,
  onCopyPath,
}: {
  detail: MarkdownDetail;
  onCopyPath: () => Promise<void>;
}) {
  const rendered = useMemo(() => renderMarkdown(detail.markdown), [detail.markdown]);
  return (
    <article className="artifact-reading markdown-reading">
      <header className="artifact-header">
        <RegistryStamp label="MARKDOWN DOCUMENT" />
        <h1>{fileLabel(detail.relativePath)}</h1>
      </header>
      <aside className="provenance-spine markdown-provenance" aria-label="Markdown provenance">
        <dl>
          <div>
            <dt>Project</dt>
            <dd>{detail.projectName}</dd>
          </div>
          <div>
            <dt>Path</dt>
            <dd>
              <code>{detail.relativePath}</code>
            </dd>
          </div>
          <div>
            <dt>Git</dt>
            <dd>{detail.git ? `Branch ${detail.git.branch}` : "Git metadata unavailable"}</dd>
          </div>
          <div>
            <dt>Modified</dt>
            <dd>{formatSourceDate(detail.sourceModifiedUnixNanos)}</dd>
          </div>
        </dl>
      </aside>
      <div className="artifact-actions markdown-actions" aria-label="Read-only handoffs">
        <button className="button" type="button" onClick={() => void onCopyPath()}>
          Copy path
        </button>
      </div>
      <section
        className="markdown-document standalone-markdown-document"
        aria-label="Rendered Markdown document"
        dangerouslySetInnerHTML={{ __html: rendered }}
      />
    </article>
  );
}

function ArtifactReadingDesk({
  detail,
  generatedView,
  onRequestSummary,
  onSelectMember,
  onCopyPath,
  onCopyPrompt,
  onOpenTerminal,
}: {
  detail: ArtifactDetail;
  generatedView: GeneratedView;
  onRequestSummary: () => Promise<void>;
  onSelectMember: (artifactId: string) => Promise<void>;
  onCopyPath: () => Promise<void>;
  onCopyPrompt: () => Promise<void>;
  onOpenTerminal: () => Promise<void>;
}) {
  const [readerMode, setReaderMode] = useState<OpenSpecReaderMode>("overview");
  const progress = detail.progress.status === "available" ? detail.progress.progress : null;
  const isStructuredOpenSpec =
    detail.bundleKind === "open_spec_change" && detail.openSpecView !== null && detail.openSpecView;
  const rendered = useMemo(() => renderMarkdown(detail.markdown), [detail.markdown]);
  const renderedOverview = useMemo(
    () =>
      detail.openSpecView?.overview.map((section) => ({
        ...section,
        rendered: renderMarkdown(section.markdown),
      })) ?? [],
    [detail.openSpecView],
  );

  useEffect(() => setReaderMode("overview"), [detail.bundleId]);

  return (
    <article className={`artifact-reading ${isStructuredOpenSpec ? "open-spec-reading" : ""}`}>
      <header className="artifact-header">
        <RegistryStamp
          label={isStructuredOpenSpec ? "OPEN SPEC CHANGE" : "CANDIDATE PLANNING FILE"}
        />
        <h1>{detail.bundleName}</h1>
        {isStructuredOpenSpec ? (
          <p className="artifact-context">
            <span>{detail.projectName}</span>
            <span>{detail.git ? detail.git.branch : "Git unavailable"}</span>
            <span>
              {progress
                ? `${progress.completed}/${progress.total} tasks complete`
                : "Task facts unavailable"}
            </span>
          </p>
        ) : (
          <p>{detail.relativePath}</p>
        )}
      </header>

      {isStructuredOpenSpec ? (
        <>
          <nav className="openspec-view-tabs" aria-label="OpenSpec viewer" role="tablist">
            {(["overview", "tasks", "source"] as const).map((mode) => (
              <button
                id={`openspec-${mode}-tab`}
                key={mode}
                type="button"
                role="tab"
                aria-selected={readerMode === mode}
                aria-controls={`openspec-${mode}-panel`}
                tabIndex={readerMode === mode ? 0 : -1}
                onClick={() => setReaderMode(mode)}
                onKeyDown={(event) => moveOpenSpecTab(event, mode, setReaderMode)}
              >
                {mode === "overview"
                  ? "Overview"
                  : mode === "tasks"
                    ? `Tasks ${progress?.total ?? 0}`
                    : "Source"}
              </button>
            ))}
          </nav>
          <ArtifactActions
            onCopyPath={onCopyPath}
            onCopyPrompt={onCopyPrompt}
            onOpenTerminal={onOpenTerminal}
          />
          <ArtifactWarnings detail={detail} />

          {readerMode === "overview" && (
            <section
              id="openspec-overview-panel"
              className="openspec-panel openspec-overview"
              role="tabpanel"
              aria-labelledby="openspec-overview-tab"
            >
              {renderedOverview.length > 0 ? (
                renderedOverview.map((section) => (
                  <OpenSpecOverviewExcerpt
                    key={`${section.sourcePath}:${section.kind}`}
                    section={section}
                  />
                ))
              ) : (
                <div className="openspec-empty">
                  <h2>No overview sections found</h2>
                  <p>This change does not contain the canonical proposal or design sections.</p>
                  <button className="button" type="button" onClick={() => setReaderMode("source")}>
                    View source files
                  </button>
                </div>
              )}
              <GeneratedSummary view={generatedView} onRequest={onRequestSummary} />
            </section>
          )}

          {readerMode === "tasks" && (
            <OpenSpecTasksPanel
              groups={detail.openSpecView?.taskGroups ?? []}
              progress={progress}
              onViewSource={() => setReaderMode("source")}
            />
          )}

          {readerMode === "source" && (
            <section
              id="openspec-source-panel"
              className="openspec-panel openspec-source"
              role="tabpanel"
              aria-labelledby="openspec-source-tab"
            >
              <ArtifactMembers detail={detail} onSelectMember={onSelectMember} />
              <details className="source-details">
                <summary>Source details</summary>
                <ArtifactProvenance detail={detail} />
              </details>
              <section
                className="markdown-document"
                aria-label="Rendered artifact Markdown"
                dangerouslySetInnerHTML={{ __html: rendered }}
              />
            </section>
          )}
        </>
      ) : (
        <>
          <ArtifactMembers detail={detail} onSelectMember={onSelectMember} />
          <ArtifactProvenance detail={detail} />
          <ArtifactWarnings detail={detail} />
          <ArtifactActions
            onCopyPath={onCopyPath}
            onCopyPrompt={onCopyPrompt}
            onOpenTerminal={onOpenTerminal}
          />
          <GeneratedSummary view={generatedView} onRequest={onRequestSummary} />
          {progress && progress.remaining.length > 0 && (
            <section className="remaining-tasks" aria-label="Remaining deterministic tasks">
              <strong>Remaining work · observed task markers</strong>
              <ol>
                {progress.remaining.map((task) => (
                  <li key={`${task.location.line}:${task.text}`}>
                    <span>{task.text}</span>
                    <small>line {task.location.line}</small>
                  </li>
                ))}
              </ol>
            </section>
          )}
          <section
            className="markdown-document"
            aria-label="Rendered artifact Markdown"
            dangerouslySetInnerHTML={{ __html: rendered }}
          />
        </>
      )}
    </article>
  );
}

function OpenSpecOverviewExcerpt({
  section,
}: {
  section: OpenSpecOverviewSection & { rendered: string };
}) {
  return (
    <section className={`overview-excerpt overview-excerpt--${section.kind}`}>
      <div className="overview-excerpt-heading">
        <h2>{overviewSectionLabel(section.kind)}</h2>
        <span>{fileLabel(section.sourcePath)}</span>
      </div>
      <div
        className="markdown-document overview-markdown"
        dangerouslySetInnerHTML={{ __html: section.rendered }}
      />
    </section>
  );
}

function OpenSpecTasksPanel({
  groups,
  progress,
  onViewSource,
}: {
  groups: OpenSpecTaskGroup[];
  progress: Extract<ArtifactDetail["progress"], { status: "available" }>["progress"] | null;
  onViewSource: () => void;
}) {
  return (
    <section
      id="openspec-tasks-panel"
      className="openspec-panel openspec-tasks"
      role="tabpanel"
      aria-labelledby="openspec-tasks-tab"
    >
      <header className="work-plan-heading">
        <div>
          <h2>Work plan</h2>
          <p>Observed task markers from tasks.md. Backstage never changes them.</p>
        </div>
        <strong>
          {progress
            ? `${progress.completed} complete · ${progress.remainingCount} remaining`
            : "Task facts unavailable"}
        </strong>
      </header>
      {groups.length > 0 ? (
        <div className="task-groups">
          {groups.map((group) => {
            const completed = group.tasks.filter((task) => task.completed).length;
            return (
              <section className="task-group" key={`${group.sourcePath}:${group.title}`}>
                <header>
                  <h3>{group.title}</h3>
                  <span>
                    {completed}/{group.tasks.length} complete
                  </span>
                </header>
                <ul>
                  {group.tasks.map((task) => (
                    <li
                      className={task.completed ? "is-complete" : "is-remaining"}
                      key={`${task.location.line}:${task.text}`}
                    >
                      {task.completed ? (
                        <CheckCircleIcon aria-hidden="true" weight="fill" />
                      ) : (
                        <CircleIcon aria-hidden="true" weight="regular" />
                      )}
                      <span>{task.text}</span>
                      <small>line {task.location.line}</small>
                    </li>
                  ))}
                </ul>
              </section>
            );
          })}
        </div>
      ) : (
        <div className="openspec-empty">
          <h2>Structured tasks unavailable</h2>
          <p>No supported task markers were found. The original tasks file remains available.</p>
          <button className="button" type="button" onClick={onViewSource}>
            View tasks source
          </button>
        </div>
      )}
    </section>
  );
}

function ArtifactMembers({
  detail,
  onSelectMember,
}: {
  detail: ArtifactDetail;
  onSelectMember: (artifactId: string) => Promise<void>;
}) {
  return (
    <nav className="artifact-members" aria-label="Bundle files">
      {detail.members.map((member) => (
        <button
          key={member.id}
          type="button"
          aria-pressed={member.id === detail.artifactId}
          className={member.id === detail.artifactId ? "is-selected" : ""}
          onClick={() => void onSelectMember(member.id)}
        >
          {fileLabel(member.relativePath)}
        </button>
      ))}
    </nav>
  );
}

function ArtifactProvenance({ detail }: { detail: ArtifactDetail }) {
  const progress = detail.progress.status === "available" ? detail.progress.progress : null;
  const recognition =
    detail.recognition.status === "recognized"
      ? `Recognized by deterministic rule: ${detail.recognition.detector ?? "detector unavailable"}`
      : `Planning candidate: ${candidateEvidenceLabel(detail.recognition.reason)}`;
  return (
    <aside className="provenance-spine" aria-label="Artifact provenance">
      <dl>
        <div>
          <dt>Project</dt>
          <dd>{detail.projectName}</dd>
        </div>
        <div>
          <dt>Bundle</dt>
          <dd>{detail.bundleName}</dd>
        </div>
        <div>
          <dt>Path</dt>
          <dd>
            <code>{detail.relativePath}</code>
          </dd>
        </div>
        <div>
          <dt>Recognition</dt>
          <dd>{recognition}</dd>
        </div>
        <div>
          <dt>Git</dt>
          <dd>{detail.git ? `Branch ${detail.git.branch}` : "Git metadata unavailable"}</dd>
        </div>
        <div>
          <dt>Modified</dt>
          <dd>{formatSourceDate(detail.sourceModifiedUnixNanos)}</dd>
        </div>
        <div>
          <dt>Parser</dt>
          <dd>
            {detail.progress.progress.parser.name} v{detail.progress.progress.parser.version}
          </dd>
        </div>
        <div>
          <dt>Task facts</dt>
          <dd>
            {progress
              ? `${progress.completed} complete · ${progress.remainingCount} remaining`
              : "Unavailable"}
          </dd>
        </div>
        <div>
          <dt>Fingerprint</dt>
          <dd>
            <code>{detail.fingerprint ? shortFingerprint(detail.fingerprint) : "Unavailable"}</code>
          </dd>
        </div>
      </dl>
    </aside>
  );
}

function ArtifactWarnings({ detail }: { detail: ArtifactDetail }) {
  const warningCount = detail.warnings.length + detail.progress.progress.warnings.length;
  if (warningCount === 0) return null;
  return (
    <section className="warning-sheet artifact-warning-sheet">
      <strong>
        {warningCount} source {warningCount === 1 ? "warning" : "warnings"}
      </strong>
      {detail.warnings.map((warning) => (
        <p key={warning}>{warning}</p>
      ))}
      {detail.progress.progress.warnings.map((warning) => (
        <p key={`${warning.line}:${warning.message}`}>
          Line {warning.line}: {warning.message}
        </p>
      ))}
    </section>
  );
}

function ArtifactActions({
  onCopyPath,
  onCopyPrompt,
  onOpenTerminal,
}: {
  onCopyPath: () => Promise<void>;
  onCopyPrompt: () => Promise<void>;
  onOpenTerminal: () => Promise<void>;
}) {
  return (
    <div className="artifact-actions" aria-label="Read-only handoffs">
      <button className="button" type="button" onClick={() => void onCopyPath()}>
        Copy path
      </button>
      <button className="button" type="button" onClick={() => void onCopyPrompt()}>
        Copy continuation prompt
      </button>
      <button className="button" type="button" onClick={() => void onOpenTerminal()}>
        Open terminal
      </button>
    </div>
  );
}

function moveOpenSpecTab(
  event: ReactKeyboardEvent<HTMLButtonElement>,
  current: OpenSpecReaderMode,
  select: (mode: OpenSpecReaderMode) => void,
) {
  const modes: OpenSpecReaderMode[] = ["overview", "tasks", "source"];
  const currentIndex = modes.indexOf(current);
  const nextIndex =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? modes.length - 1
        : event.key === "ArrowRight"
          ? (currentIndex + 1) % modes.length
          : event.key === "ArrowLeft"
            ? (currentIndex - 1 + modes.length) % modes.length
            : null;
  if (nextIndex === null) return;
  event.preventDefault();
  const next = modes[nextIndex];
  if (!next) return;
  document.getElementById(`openspec-${next}-tab`)?.focus();
  select(next);
}

function overviewSectionLabel(kind: OpenSpecOverviewSection["kind"]) {
  switch (kind) {
    case "why":
      return "Why this change";
    case "what_changes":
      return "What changes";
    case "goals_and_non_goals":
      return "Goals and boundaries";
    case "decisions":
      return "Key decisions";
    case "risks_and_trade_offs":
      return "Risks and trade-offs";
  }
}

function GeneratedSummary({
  view,
  onRequest,
}: {
  view: GeneratedView;
  onRequest: () => Promise<void>;
}) {
  const result = generatedResult(view);
  const isGenerating = view.status === "generating";
  const isStale = view.status === "stale";
  return (
    <section
      className={`generated-summary generated-summary--${view.status}`}
      aria-label="Pi-generated Summary"
    >
      <div className="generated-summary-heading">
        <div>
          <span>Pi-generated</span>
          <strong>Summary</strong>
        </div>
        <span className="generated-status">{generatedStatusLabel(view)}</span>
      </div>
      {view.status === "never_generated" && (
        <p>
          No repository content has been sent to Pi. Generate only when you want a bounded snapshot
          explained.
        </p>
      )}
      {view.status === "stale" && (
        <p>
          The source fingerprint changed after this result. Changed inputs:{" "}
          {view.changedInputs.length > 0
            ? view.changedInputs.join(", ")
            : "source content or membership"}
          .
        </p>
      )}
      {view.status === "failed" && (
        <p className="generated-failure">
          Generation failed: {view.failure}.{" "}
          {result ? "The prior result remains below." : "No result was replaced."}
        </p>
      )}
      {isGenerating && result && (
        <p className="generated-prior-label">
          Prior result remains visible while regeneration runs.
        </p>
      )}
      {result && (
        <div className="generated-text">
          <p>{result.text}</p>
          <small>
            Generated {result.generatedAt} · {result.model ?? "model unavailable"} · prompt{" "}
            {result.promptVersion}
          </small>
        </div>
      )}
      <button
        className={result ? "button" : "button button--primary"}
        type="button"
        disabled={isGenerating}
        onClick={() => void onRequest()}
      >
        {isGenerating
          ? "Generating…"
          : result
            ? isStale
              ? "Regenerate Summary"
              : "Generate again"
            : "Generate Summary"}
      </button>
    </section>
  );
}

function generatedResult(view: GeneratedView) {
  switch (view.status) {
    case "current":
    case "stale":
      return view.result;
    case "generating":
    case "failed":
      return view.previous;
    case "never_generated":
      return undefined;
  }
}

function generatedStatusLabel(view: GeneratedView) {
  switch (view.status) {
    case "never_generated":
      return "Never generated";
    case "generating":
      return "Generating";
    case "current":
      return "Current";
    case "stale":
      return "Stale · regeneration available";
    case "failed":
      return "Failed · prior result preserved";
  }
}

function DocumentRow({
  document,
  selected,
  onSelect,
}: {
  document: IndexedMarkdownDocument;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      className={`bundle-row document-row ${selected ? "is-selected" : ""}`}
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
    >
      <span className="bundle-row-top">
        <span className="bundle-kind bundle-kind--markdown">Markdown document</span>
      </span>
      <strong>{fileLabel(document.relativePath)}</strong>
      <small>{document.projectName}</small>
      <span className="bundle-progress">
        {document.relativePath}
        {document.sourceModifiedUnixNanos
          ? ` · changed ${formatSourceDate(document.sourceModifiedUnixNanos)}`
          : ""}
      </span>
    </button>
  );
}

function BundleRow({
  bundle,
  selected,
  onSelect,
}: {
  bundle: IndexedBundle;
  selected: boolean;
  onSelect: () => void;
}) {
  const progress = bundle.progress.status === "available" ? bundle.progress.progress : null;
  const warningCount = bundle.warnings.length + bundle.progress.progress.warnings.length;
  const label = bundle.bundle.kind === "open_spec_change" ? "OpenSpec" : "Planning candidate";
  return (
    <button
      className={`bundle-row ${selected ? "is-selected" : ""}`}
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
    >
      <span className="bundle-row-top">
        <span className={`bundle-kind bundle-kind--${bundle.bundle.kind}`}>{label}</span>
        {warningCount > 0 && <span className="bundle-warning">{warningCount} warning</span>}
      </span>
      <strong>{bundle.bundle.name}</strong>
      <small>{bundle.bundle.projectName}</small>
      <span className="bundle-progress">
        {bundle.bundle.kind === "possible_artifact"
          ? candidateEvidenceLabel(
              bundle.bundle.recognition.status === "possible"
                ? bundle.bundle.recognition.reason
                : undefined,
            )
          : progress
            ? `${progress.completed}/${progress.total} tasks complete`
            : "Progress unavailable"}
        {bundle.sourceModifiedUnixNanos
          ? ` · changed ${formatSourceDate(bundle.sourceModifiedUnixNanos)}`
          : ""}
      </span>
    </button>
  );
}

function ScanSkeleton() {
  return (
    <div className="skeleton-list" aria-label="Scanning projects">
      {Array.from({ length: 6 }, (_, index) => (
        <span key={index} style={{ width: `${82 - index * 5}%` }} />
      ))}
    </div>
  );
}

function RegistryStamp({
  label,
  tone = "default",
}: {
  label: string;
  tone?: "default" | "warning";
}) {
  return <span className={`registry-stamp registry-stamp--${tone}`}>{label}</span>;
}

function isEditableTarget(target: EventTarget | null) {
  return (
    target instanceof HTMLElement &&
    (target.matches("input, textarea, select") || target.isContentEditable)
  );
}

function planningFileCountLabel(count: number) {
  return `${count} planning ${count === 1 ? "file" : "files"}`;
}

function markdownFileCountLabel(count: number) {
  return `${count} Markdown ${count === 1 ? "file" : "files"}`;
}

function candidateEvidenceLabel(reason?: string) {
  if (!reason || reason === "Candidate filename" || reason.includes("planning candidate")) {
    return "Matched configured planning filename";
  }
  return reason;
}

function compactProjectLabel(name: string) {
  const parts = name.split(/[\s._-]+/).filter(Boolean);
  if (parts.length > 1)
    return parts
      .slice(0, 3)
      .map((part) => part[0])
      .join("")
      .toUpperCase();
  return name.slice(0, 3).toUpperCase();
}

function recentBundles(bundles: IndexedBundle[]) {
  const latest = Math.max(...bundles.map((bundle) => bundle.sourceModifiedUnixNanos ?? 0), 0);
  if (!latest) return [];
  const sevenDaysInNanos = 7 * 24 * 60 * 60 * 1_000_000_000;
  return bundles
    .filter(
      (bundle) =>
        bundle.sourceModifiedUnixNanos !== null &&
        latest - bundle.sourceModifiedUnixNanos <= sevenDaysInNanos,
    )
    .sort(
      (left, right) => (right.sourceModifiedUnixNanos ?? 0) - (left.sourceModifiedUnixNanos ?? 0),
    );
}

function fileLabel(path: string) {
  const parts = path.split("/");
  if (path.includes("/specs/") && parts.length >= 2) {
    return `specs/${parts.at(-2)}/${parts.at(-1)}`;
  }
  return parts.at(-1) ?? path;
}

function formatSourceDate(unixNanos: number | null) {
  if (!unixNanos) return "Source date unavailable";
  const date = new Date(unixNanos / 1_000_000);
  return Number.isNaN(date.valueOf()) ? "Source date unavailable" : date.toLocaleString();
}

function shortFingerprint(fingerprint: string) {
  return fingerprint.length > 22
    ? `${fingerprint.slice(0, 16)}…${fingerprint.slice(-6)}`
    : fingerprint;
}

function shortPath(path: string) {
  const parts = path.split("/").filter(Boolean);
  return parts.length > 3 ? `…/${parts.slice(-3).join("/")}` : path;
}

function workspaceStatusLabel(status: WorkspaceStatus, warningCount: number) {
  switch (status) {
    case "loading":
      return "Opening registry";
    case "no-root":
      return "Approval needed";
    case "scanning":
      return "Scanning read-only";
    case "ready":
      return "Ready";
    case "ready-with-warnings":
      return `Ready with ${warningCount} ${warningCount === 1 ? "warning" : "warnings"}`;
    case "unavailable":
      return "Root unavailable";
  }
}

function errorMessage(cause: unknown) {
  if (typeof cause === "object" && cause && "message" in cause && typeof cause.message === "string")
    return cause.message;
  return cause instanceof Error ? cause.message : String(cause);
}
