import { ArrowClockwiseIcon } from "@phosphor-icons/react/dist/csr/ArrowClockwise";
import { ArchiveIcon } from "@phosphor-icons/react/dist/csr/Archive";
import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { CircleIcon } from "@phosphor-icons/react/dist/csr/Circle";
import { CommandIcon } from "@phosphor-icons/react/dist/csr/Command";
import { FilesIcon } from "@phosphor-icons/react/dist/csr/Files";
import { FolderSimpleIcon } from "@phosphor-icons/react/dist/csr/FolderSimple";
import { GearSixIcon } from "@phosphor-icons/react/dist/csr/GearSix";
import { MagnifyingGlassIcon } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { SidebarSimpleIcon } from "@phosphor-icons/react/dist/csr/SidebarSimple";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent, RefObject } from "react";

import { compareDatedRecords, groupDatedRecords, validSourceMilliseconds } from "./activity";
import { WorkRecordReadingDesk } from "./CapabilityRenderer";
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
import { settleBatches } from "./settleBatches";
import type {
  AnnotationCommand,
  AnnotationTarget,
  ApprovedRoot,
  ArtifactDetail,
  BackstageApi,
  GeneratedView,
  IndexSnapshot,
  IndexedBundle,
  MarkdownDetail,
  MarkdownDocument,
  OpenSpecOverviewSection,
  OpenSpecPrimaryStatus,
  PatternMutation,
  PlanningPattern,
  OpenSpecTaskGroup,
  Project,
  ScanWarning,
  SourceTimestamp,
  WorkRecord,
  WorkRecordAnnotation,
  WorkRecordDetail,
} from "./api";

type WorkspaceStatus =
  "loading" | "no-root" | "scanning" | "ready" | "ready-with-warnings" | "unavailable";
type RegistryScope = "planning" | "markdown";
type BundleFilter = "current" | "active" | "done" | "archived" | "warning" | "stale";
type AnnotationFilter =
  | "all"
  | "undecided"
  | "approved"
  | "rejected"
  | "applicable"
  | "obsolete"
  | "superseded"
  | "favorite"
  | "todo"
  | "priority_low"
  | "priority_medium"
  | "priority_high";
type IndexedMarkdownDocument = MarkdownDocument & { rootId: string };
type IndexedWorkRecord = {
  rootId: string;
  indexGeneration: number;
  project: Project;
  record: WorkRecord;
  generationSupported: boolean;
};
type LedgerRecord =
  | {
      kind: "record";
      id: string;
      sourceModifiedUnixNanos: SourceTimestamp;
      record: IndexedWorkRecord;
    }
  | { kind: "bundle"; id: string; sourceModifiedUnixNanos: SourceTimestamp; bundle: IndexedBundle }
  | {
      kind: "document";
      id: string;
      sourceModifiedUnixNanos: SourceTimestamp;
      document: IndexedMarkdownDocument;
    };
const LEDGER_BATCH_SIZE = 200;
const GENERATED_INVENTORY_CONCURRENCY = 4;
const systemClock = () => new Date();

interface AppProps {
  api?: BackstageApi;
  clock?: () => Date;
}

export function App({ api = runtimeApi, clock = systemClock }: AppProps) {
  const [roots, setRoots] = useState<ApprovedRoot[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [indexes, setIndexes] = useState<IndexSnapshot[]>([]);
  const [warnings, setWarnings] = useState<ScanWarning[]>([]);
  const [status, setStatus] = useState<WorkspaceStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState("all");
  const [registryScope, setRegistryScope] = useState<RegistryScope>("planning");
  const [bundleFilter, setBundleFilter] = useState<BundleFilter>("current");
  const [annotationFilter, setAnnotationFilter] = useState<AnnotationFilter>("all");
  const [currentTime, setCurrentTime] = useState(clock);
  const [ledgerLimit, setLedgerLimit] = useState(LEDGER_BATCH_SIZE);
  const [selectedWorkRecord, setSelectedWorkRecord] = useState<WorkRecordDetail | null>(null);
  const [storedAnnotationTargets, setStoredAnnotationTargets] = useState<AnnotationTarget[]>([]);
  const [selectedArtifact, setSelectedArtifact] = useState<ArtifactDetail | null>(null);
  const [selectedMarkdown, setSelectedMarkdown] = useState<MarkdownDetail | null>(null);
  const selectedWorkRecordRef = useRef<WorkRecordDetail | null>(null);
  const selectedArtifactRef = useRef<ArtifactDetail | null>(null);
  const detailRequestRef = useRef(0);
  const scanRequestRef = useRef(0);
  const inventoryEpochRef = useRef(0);
  const patternRequestRef = useRef(0);
  const patternMutationRef = useRef(0);
  const patternRevisionRef = useRef(0);
  const settingsMutationRef = useRef(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [handoffNotice, setHandoffNotice] = useState<string | null>(null);
  const [generatedView, setGeneratedView] = useState<GeneratedView>({ status: "never_generated" });
  const [generatedInventory, setGeneratedInventory] = useState<Record<string, GeneratedView>>({});
  const [paneLayout, setPaneLayout] = useState(loadPaneLayout);
  const [searchQuery, setSearchQuery] = useState("");
  const [paletteQuery, setPaletteQuery] = useState("");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [appMode, setAppMode] = useState<"work" | "settings">("work");
  const [patterns, setPatterns] = useState<PlanningPattern[]>([]);
  const [patternRevision, setPatternRevision] = useState(0);
  const [patternsLoading, setPatternsLoading] = useState(true);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [settingsNotice, setSettingsNotice] = useState<string | null>(null);
  const [failedPatternRootIds, setFailedPatternRootIds] = useState<string[]>([]);
  const [confirmingRootId, setConfirmingRootId] = useState<string | null>(null);
  const [removingRootId, setRemovingRootId] = useState<string | null>(null);
  const paletteTriggerRef = useRef<HTMLButtonElement>(null);
  const settingsTriggerRef = useRef<HTMLButtonElement>(null);
  const settingsHeadingRef = useRef<HTMLHeadingElement>(null);
  const planningPatternInputRef = useRef<HTMLInputElement>(null);
  const settingsOpenerRef = useRef<HTMLElement | null>(null);
  const paletteInputRef = useRef<HTMLInputElement>(null);
  const restorePaletteFocusRef = useRef(false);
  const resizeCleanupRef = useRef<(() => void) | null>(null);
  const readingDeskRef = useRef<HTMLElement>(null);

  useEffect(() => {
    let timer = 0;
    const scheduleRegroup = () => {
      const now = clock();
      setCurrentTime(now);
      const nextMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
      timer = window.setTimeout(
        scheduleRegroup,
        Math.max(1, nextMidnight.getTime() - now.getTime() + 25),
      );
    };
    scheduleRegroup();
    return () => window.clearTimeout(timer);
  }, [clock]);

  const reconcileSelectedWorkRecord = useCallback(
    async (nextIndexes: IndexSnapshot[], inventoryEpoch: number, detailRequest: number) => {
      const selected = selectedWorkRecordRef.current;
      if (!selected || !api.getWorkRecordDetail) return;
      let retained: IndexedWorkRecord | undefined;
      for (const index of [...nextIndexes].sort((left, right) =>
        left.rootId.localeCompare(right.rootId),
      )) {
        for (const project of index.projects) {
          const record = project.records?.find(
            (candidate) => candidate.subjectId === selected.subjectId,
          );
          if (!record) continue;
          retained = {
            rootId: index.rootId,
            indexGeneration: index.generation,
            project: project.project,
            record,
            generationSupported: project.bundles.some((bundle) =>
              samePaths(
                record.sources.map((source) => source.relativePath),
                bundle.bundle.members.map((member) => member.relativePath),
              ),
            ),
          };
          break;
        }
        if (retained) break;
      }
      if (!retained) {
        if (selectedWorkRecordRef.current?.subjectId === selected.subjectId) {
          selectedWorkRecordRef.current = null;
          setSelectedWorkRecord(null);
        }
        return;
      }
      try {
        const detailPromise = api.getWorkRecordDetail(
          retained.rootId,
          retained.record.subjectId,
          retained.indexGeneration,
        );
        const generatedPromise = retained.generationSupported
          ? api.getGeneratedView(retained.rootId, retained.record.subjectId).catch(() => undefined)
          : Promise.resolve(undefined);
        const [detail, generated] = await Promise.all([detailPromise, generatedPromise]);
        if (
          inventoryEpoch !== inventoryEpochRef.current ||
          detailRequest !== detailRequestRef.current ||
          selectedWorkRecordRef.current?.subjectId !== selected.subjectId
        ) {
          return;
        }
        selectedWorkRecordRef.current = detail;
        setSelectedWorkRecord(detail);
        setGeneratedView(generated ?? { status: "never_generated" });
        setGeneratedInventory((inventory) => {
          if (generated) return { ...inventory, [retained.record.subjectId]: generated };
          const next = { ...inventory };
          delete next[retained.record.subjectId];
          return next;
        });
      } catch (cause) {
        if (
          inventoryEpoch === inventoryEpochRef.current &&
          detailRequest === detailRequestRef.current &&
          selectedWorkRecordRef.current?.subjectId === selected.subjectId
        ) {
          selectedWorkRecordRef.current = null;
          setSelectedWorkRecord(null);
          setDetailError(errorMessage(cause));
        }
      }
    },
    [api],
  );

  const scan = useCallback(
    async (nextRoots: ApprovedRoot[]) => {
      const requestId = ++scanRequestRef.current;
      if (nextRoots.length === 0) {
        ++inventoryEpochRef.current;
        ++detailRequestRef.current;
        selectedWorkRecordRef.current = null;
        setSelectedWorkRecord(null);
        setProjects([]);
        setIndexes([]);
        setWarnings([]);
        setStatus("no-root");
        return true;
      }

      setStatus("scanning");
      setError(null);
      try {
        const cachedIndexes = (
          await Promise.all(nextRoots.map((root) => api.getIndex(root.id)))
        ).filter((index): index is IndexSnapshot => index !== null);
        if (requestId !== scanRequestRef.current) return;
        if (cachedIndexes.length > 0) {
          setIndexes(cachedIndexes);
          setProjects(cachedIndexes.flatMap((index) => index.projects.map((item) => item.project)));
        }
        const results = await Promise.all(nextRoots.map((root) => api.scanRoot(root.id)));
        if (requestId !== scanRequestRef.current) return;
        const nextProjects = results.flatMap((result) => result.projects);
        const nextWarnings = results.flatMap((result) => result.warnings);
        const nextIndexes = (
          await Promise.all(nextRoots.map((root) => api.getIndex(root.id)))
        ).filter((index): index is IndexSnapshot => index !== null);
        if (requestId !== scanRequestRef.current) return;
        const bundleOwners = new Map<string, { rootId: string; bundle: IndexedBundle }>();
        const generatedOwners = new Map<string, { rootId: string; ownerId: string }>();
        for (const index of [...nextIndexes].sort((left, right) =>
          left.rootId.localeCompare(right.rootId),
        )) {
          for (const project of index.projects) {
            for (const bundle of project.bundles) {
              if (!bundleOwners.has(bundle.bundle.id)) {
                bundleOwners.set(bundle.bundle.id, { rootId: index.rootId, bundle });
              }
            }
            for (const record of project.records ?? []) {
              if (
                !generatedOwners.has(record.subjectId) &&
                project.bundles.some((bundle) =>
                  samePaths(
                    record.sources.map((source) => source.relativePath),
                    bundle.bundle.members.map((member) => member.relativePath),
                  ),
                )
              ) {
                generatedOwners.set(record.subjectId, {
                  rootId: index.rootId,
                  ownerId: record.subjectId,
                });
              }
            }
            if (project.records === undefined) {
              for (const bundle of project.bundles) {
                if (!generatedOwners.has(bundle.bundle.id)) {
                  generatedOwners.set(bundle.bundle.id, {
                    rootId: index.rootId,
                    ownerId: bundle.bundle.id,
                  });
                }
              }
            }
          }
        }
        const inventoryEpoch = ++inventoryEpochRef.current;
        const detailRequest = ++detailRequestRef.current;
        const retainedBundleIds = new Set(bundleOwners.keys());
        const retainedSubjectIds = new Set(
          nextIndexes.flatMap((index) =>
            index.projects.flatMap((project) =>
              (project.records ?? []).map((record) => record.subjectId),
            ),
          ),
        );
        setProjects(nextProjects);
        setIndexes(nextIndexes);
        setGeneratedInventory((inventory) =>
          Object.fromEntries(
            Object.entries(inventory).filter(
              ([ownerId]) => retainedBundleIds.has(ownerId) || retainedSubjectIds.has(ownerId),
            ),
          ),
        );
        void reconcileSelectedWorkRecord(nextIndexes, inventoryEpoch, detailRequest);
        setWarnings(nextWarnings);
        const projectIdsWithWork = new Set(
          nextIndexes.flatMap((index) =>
            index.projects
              .filter(
                (project) =>
                  (project.records?.length ?? 0) > 0 ||
                  project.bundles.some((bundle) => bundle.bundle.members.length > 0),
              )
              .map((project) => project.project.id),
          ),
        );
        setSelectedProjectId((current) =>
          current === "all" || projectIdsWithWork.has(current) ? current : "all",
        );
        setStatus(nextWarnings.length > 0 ? "ready-with-warnings" : "ready");

        const isCurrentInventory = () =>
          requestId === scanRequestRef.current && inventoryEpoch === inventoryEpochRef.current;
        void settleBatches(
          [...generatedOwners.values()],
          GENERATED_INVENTORY_CONCURRENCY,
          ({ rootId, ownerId }) => api.getGeneratedView(rootId, ownerId),
          (batch) => {
            if (!isCurrentInventory()) return;
            const generatedEntries = batch.flatMap(({ item, result }) =>
              result.status === "fulfilled" ? ([[item.ownerId, result.value]] as const) : [],
            );
            if (generatedEntries.length === 0) return;
            setGeneratedInventory((inventory) =>
              isCurrentInventory()
                ? { ...inventory, ...Object.fromEntries(generatedEntries) }
                : inventory,
            );
            const selectedSubjectId = selectedWorkRecordRef.current?.subjectId;
            const selectedGenerated = generatedEntries.find(
              ([ownerId]) => ownerId === selectedSubjectId,
            );
            if (selectedGenerated && isCurrentInventory()) {
              setGeneratedView(selectedGenerated[1]);
            }
          },
          isCurrentInventory,
        );
        return true;
      } catch (cause) {
        if (requestId !== scanRequestRef.current) return;
        setError(errorMessage(cause));
        setStatus("unavailable");
        return false;
      }
    },
    [api, reconcileSelectedWorkRecord],
  );

  const loadPatterns = useCallback(async () => {
    const requestId = ++patternRequestRef.current;
    setPatternsLoading(true);
    try {
      const configuration = await api.listPatterns();
      if (
        requestId !== patternRequestRef.current ||
        configuration.revision < patternRevisionRef.current
      )
        return;
      patternRevisionRef.current = configuration.revision;
      setPatterns(configuration.patterns);
      setPatternRevision(configuration.revision);
      setSettingsError(null);
    } catch (cause) {
      if (requestId === patternRequestRef.current) setSettingsError(errorMessage(cause));
    } finally {
      if (requestId === patternRequestRef.current) setPatternsLoading(false);
    }
  }, [api]);

  useEffect(() => {
    void loadPatterns();
  }, [loadPatterns]);

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
    if (settingsMutationRef.current) return;
    settingsMutationRef.current = true;
    setSettingsBusy(true);
    try {
      const path = await api.chooseRoot();
      if (!path) return;
      setStatus("scanning");
      setError(null);
      setSettingsError(null);
      const root = await api.approveRoot(path);
      const alreadyApproved = roots.some((candidate) => candidate.id === root.id);
      const nextRoots = alreadyApproved ? roots : [...roots, root];
      setRoots(nextRoots);
      setSettingsNotice(
        alreadyApproved
          ? "That folder was already approved; one approval remains."
          : "Root approved. Scanning read-only.",
      );
      await scan(nextRoots);
    } catch (cause) {
      const message = errorMessage(cause);
      setError(message);
      setSettingsError(message);
      setStatus(roots.length > 0 ? "unavailable" : "no-root");
    } finally {
      settingsMutationRef.current = false;
      setSettingsBusy(false);
    }
  };

  const replaceIndexInventory = (nextIndexes: IndexSnapshot[]) => {
    ++scanRequestRef.current;
    const detailRequest = ++detailRequestRef.current;
    const inventoryEpoch = ++inventoryEpochRef.current;
    setIndexes(nextIndexes);
    setProjects(nextIndexes.flatMap((index) => index.projects.map((item) => item.project)));
    setWarnings(nextIndexes.flatMap((index) => index.warnings));
    const retainedBundleIds = new Set(
      nextIndexes.flatMap((index) =>
        index.projects.flatMap((project) => project.bundles.map((bundle) => bundle.bundle.id)),
      ),
    );
    const retainedSubjectIds = new Set(
      nextIndexes.flatMap((index) =>
        index.projects.flatMap((project) =>
          (project.records ?? []).map((record) => record.subjectId),
        ),
      ),
    );
    const retainedDocumentIds = new Set(
      nextIndexes.flatMap((index) =>
        index.projects.flatMap((project) =>
          project.markdownDocuments.map((document) => document.id),
        ),
      ),
    );
    setGeneratedInventory((inventory) =>
      Object.fromEntries(
        Object.entries(inventory).filter(
          ([id]) => retainedBundleIds.has(id) || retainedSubjectIds.has(id),
        ),
      ),
    );
    void reconcileSelectedWorkRecord(nextIndexes, inventoryEpoch, detailRequest);
    if (selectedArtifact && retainedBundleIds.has(selectedArtifact.bundleId)) {
      const retainedOwner = [...nextIndexes]
        .sort((left, right) => left.rootId.localeCompare(right.rootId))
        .find((index) =>
          index.projects.some((project) =>
            project.bundles.some((bundle) => bundle.bundle.id === selectedArtifact.bundleId),
          ),
        );
      if (retainedOwner && retainedOwner.rootId !== selectedArtifact.rootId) {
        const retainedDetail = { ...selectedArtifact, rootId: retainedOwner.rootId };
        selectedArtifactRef.current = retainedDetail;
        setSelectedArtifact(retainedDetail);
      }
    }
    if (selectedMarkdown && retainedDocumentIds.has(selectedMarkdown.documentId)) {
      const retainedOwner = [...nextIndexes]
        .sort((left, right) => left.rootId.localeCompare(right.rootId))
        .find((index) =>
          index.projects.some((project) =>
            project.markdownDocuments.some(
              (document) => document.id === selectedMarkdown.documentId,
            ),
          ),
        );
      if (retainedOwner && retainedOwner.rootId !== selectedMarkdown.rootId) {
        setSelectedMarkdown({ ...selectedMarkdown, rootId: retainedOwner.rootId });
      }
    }
  };

  const openSettings = (opener: HTMLElement | null) => {
    settingsOpenerRef.current = opener;
    setAppMode("settings");
    setSettingsError(null);
    setSettingsNotice(null);
    void loadPatterns();
  };

  const closeSettings = () => {
    settingsOpenerRef.current?.focus();
    setAppMode("work");
    setConfirmingRootId(null);
  };

  useEffect(() => {
    if (appMode === "settings") settingsHeadingRef.current?.focus();
  }, [appMode]);

  const removeApprovedRoot = async (rootId: string) => {
    if (settingsMutationRef.current) return;
    settingsMutationRef.current = true;
    setSettingsBusy(true);
    setRemovingRootId(rootId);
    setSettingsError(null);
    setSettingsNotice(null);
    try {
      const inventory = await api.removeRoot(rootId);
      const retainedRootIds = new Set(inventory.roots.map((root) => root.id));
      setRoots(inventory.roots);
      setFailedPatternRootIds((ids) => ids.filter((id) => retainedRootIds.has(id)));
      replaceIndexInventory(inventory.indexes);
      setConfirmingRootId(null);
      setStatus(inventory.roots.length === 0 ? "no-root" : "ready");
      setSettingsNotice("Approval removed. Repository files remain untouched.");
      requestAnimationFrame(() => settingsHeadingRef.current?.focus());
    } catch (cause) {
      setSettingsError(errorMessage(cause));
    } finally {
      settingsMutationRef.current = false;
      setSettingsBusy(false);
      setRemovingRootId(null);
    }
  };

  const applyPatternMutation = async (
    mutation: () => Promise<PatternMutation>,
    focusAfterRemoval = false,
  ) => {
    if (settingsMutationRef.current) return;
    settingsMutationRef.current = true;
    setSettingsBusy(true);
    const mutationId = ++patternMutationRef.current;
    setSettingsError(null);
    setSettingsNotice(null);
    try {
      const result = await mutation();
      if (
        mutationId !== patternMutationRef.current ||
        result.configurationRevision < patternRevisionRef.current
      )
        return;
      ++patternRequestRef.current;
      patternRevisionRef.current = result.configurationRevision;
      setPatterns(result.patterns);
      setPatternRevision(result.configurationRevision);
      setPatternsLoading(false);
      setFailedPatternRootIds(result.failedRootIds);
      replaceIndexInventory(result.indexes);
      const indexWarnings = result.indexes.flatMap((index) => index.warnings);
      setStatus(
        roots.length === 0
          ? "no-root"
          : result.failedRootIds.length > 0 || indexWarnings.length > 0
            ? "ready-with-warnings"
            : "ready",
      );
      setSettingsNotice("Planning patterns saved in app-owned configuration.");
      if (focusAfterRemoval) requestAnimationFrame(() => planningPatternInputRef.current?.focus());
    } catch (cause) {
      if (mutationId === patternMutationRef.current) setSettingsError(errorMessage(cause));
    } finally {
      settingsMutationRef.current = false;
      if (mutationId === patternMutationRef.current) setSettingsBusy(false);
    }
  };

  const retryFailedPatternRoots = async () => {
    if (await scan(roots)) setFailedPatternRootIds([]);
  };

  const projectFileCounts = useMemo(() => {
    const memberIds = new Map<string, Set<string>>();
    const query = searchQuery.trim().toLowerCase();
    for (const index of indexes) {
      for (const indexedProject of index.projects) {
        const projectMembers = memberIds.get(indexedProject.project.id) ?? new Set<string>();
        if (indexedProject.records !== undefined) {
          for (const record of indexedProject.records) {
            if (!recordInScope(record, registryScope)) continue;
            if (!recordMatchesFilter(record, bundleFilter, generatedInventory)) continue;
            if (!recordMatchesAnnotation(record, annotationFilter)) continue;
            if (query && !recordSearchText(record, indexedProject.project).includes(query))
              continue;
            for (const source of record.sources) projectMembers.add(source.relativePath);
          }
          memberIds.set(indexedProject.project.id, projectMembers);
          continue;
        }
        const matchingBundles = indexedProject.bundles.filter(
          (bundle) =>
            bundleMatchesFilter(bundle, bundleFilter, generatedInventory) &&
            (!query ||
              [
                bundle.bundle.name,
                bundle.bundle.projectName,
                ...bundle.bundle.members.map((member) => member.relativePath),
              ]
                .join(" ")
                .toLowerCase()
                .includes(query)),
        );
        for (const bundle of matchingBundles) {
          for (const member of bundle.bundle.members) projectMembers.add(member.id);
        }
        if (registryScope === "markdown" && bundleFilter === "current") {
          const represented = new Set(
            indexedProject.bundles.flatMap((bundle) =>
              bundle.bundle.members.map((member) => member.id),
            ),
          );
          for (const document of indexedProject.markdownDocuments) {
            if (
              !represented.has(document.id) &&
              (!query ||
                [document.relativePath, document.projectName]
                  .join(" ")
                  .toLowerCase()
                  .includes(query))
            ) {
              projectMembers.add(document.id);
            }
          }
        }
        memberIds.set(indexedProject.project.id, projectMembers);
      }
    }
    return new Map([...memberIds].map(([projectId, members]) => [projectId, members.size]));
  }, [annotationFilter, bundleFilter, generatedInventory, indexes, registryScope, searchQuery]);

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

  const visibleRecords = useMemo(() => {
    const uniqueWorkRecords = new Map<string, IndexedWorkRecord>();
    const uniqueBundles = new Map<string, IndexedBundle>();
    const uniqueDocuments = new Map<string, IndexedMarkdownDocument>();
    for (const index of [...indexes].sort((left, right) =>
      left.rootId.localeCompare(right.rootId),
    )) {
      for (const project of index.projects) {
        if (selectedProjectId !== "all" && project.project.id !== selectedProjectId) continue;
        if (project.records !== undefined) {
          for (const record of project.records) {
            if (!uniqueWorkRecords.has(record.subjectId)) {
              uniqueWorkRecords.set(record.subjectId, {
                rootId: index.rootId,
                indexGeneration: index.generation,
                project: project.project,
                record,
                generationSupported: project.bundles.some((bundle) =>
                  samePaths(
                    record.sources.map((source) => source.relativePath),
                    bundle.bundle.members.map((member) => member.relativePath),
                  ),
                ),
              });
            }
          }
          continue;
        }
        for (const bundle of project.bundles) {
          if (!uniqueBundles.has(bundle.bundle.id)) uniqueBundles.set(bundle.bundle.id, bundle);
        }
        if (registryScope !== "markdown") continue;
        const represented = new Set(
          project.bundles.flatMap((bundle) => bundle.bundle.members.map((member) => member.id)),
        );
        for (const document of project.markdownDocuments) {
          if (!represented.has(document.id) && !uniqueDocuments.has(document.id)) {
            uniqueDocuments.set(document.id, { ...document, rootId: index.rootId });
          }
        }
      }
    }

    const query = searchQuery.trim().toLowerCase();
    const records = [...uniqueWorkRecords.values()]
      .filter(({ record }) => recordInScope(record, registryScope))
      .filter(({ record }) => recordMatchesFilter(record, bundleFilter, generatedInventory))
      .filter(({ record }) => recordMatchesAnnotation(record, annotationFilter))
      .filter(({ record, project }) => !query || recordSearchText(record, project).includes(query))
      .map((record): LedgerRecord => ({
        kind: "record",
        id: record.record.subjectId,
        sourceModifiedUnixNanos: record.record.sourceModifiedUnixNanos,
        record,
      }));
    const bundles = [...uniqueBundles.values()]
      .filter((bundle) => bundleMatchesFilter(bundle, bundleFilter, generatedInventory))
      .filter(
        (bundle) =>
          !query ||
          [
            bundle.bundle.name,
            bundle.bundle.projectName,
            ...bundle.bundle.members.map((member) => member.relativePath),
          ]
            .join(" ")
            .toLowerCase()
            .includes(query),
      )
      .map((bundle): LedgerRecord => ({
        kind: "bundle",
        id: bundle.bundle.id,
        sourceModifiedUnixNanos: bundle.sourceModifiedUnixNanos,
        bundle,
      }));
    const documents =
      bundleFilter === "current"
        ? [...uniqueDocuments.values()]
            .filter(
              (document) =>
                !query ||
                [document.relativePath, document.projectName]
                  .join(" ")
                  .toLowerCase()
                  .includes(query),
            )
            .map((document): LedgerRecord => ({
              kind: "document",
              id: document.id,
              sourceModifiedUnixNanos: document.sourceModifiedUnixNanos,
              document,
            }))
        : [];
    return [...records, ...bundles, ...documents].sort(compareDatedRecords);
  }, [
    annotationFilter,
    bundleFilter,
    generatedInventory,
    indexes,
    registryScope,
    searchQuery,
    selectedProjectId,
  ]);

  const visibleRecordCount = visibleRecords.length;
  const visibleFileCount = new Set(
    visibleRecords.flatMap((record) =>
      record.kind === "record"
        ? record.record.record.sources.map(
            (source) => `${record.record.project.id}:${source.relativePath}`,
          )
        : record.kind === "bundle"
          ? record.bundle.bundle.members.map((member) => member.id)
          : [record.document.id],
    ),
  ).size;
  const displayedRecords = visibleRecords.slice(0, ledgerLimit);
  const displayedGroups = groupDatedRecords(displayedRecords, currentTime);
  const remainingRecordCount = Math.max(0, visibleRecordCount - displayedRecords.length);

  useEffect(
    () => setLedgerLimit(LEDGER_BATCH_SIZE),
    [annotationFilter, bundleFilter, indexes, registryScope, searchQuery, selectedProjectId],
  );

  const commitSelectedWorkRecord = useCallback((detail: WorkRecordDetail) => {
    selectedWorkRecordRef.current = detail;
    selectedArtifactRef.current = null;
    setSelectedArtifact(null);
    setSelectedMarkdown(null);
    setSelectedWorkRecord(detail);
  }, []);

  const commitSelectedArtifact = useCallback((detail: ArtifactDetail) => {
    selectedWorkRecordRef.current = null;
    selectedArtifactRef.current = detail;
    setSelectedWorkRecord(null);
    setSelectedMarkdown(null);
    setSelectedArtifact(detail);
  }, []);

  const commitSelectedMarkdown = useCallback((detail: MarkdownDetail) => {
    selectedWorkRecordRef.current = null;
    selectedArtifactRef.current = null;
    setSelectedWorkRecord(null);
    setSelectedArtifact(null);
    setSelectedMarkdown(detail);
    setGeneratedView({ status: "never_generated" });
  }, []);

  const selectWorkRecord = useCallback(
    async (indexed: IndexedWorkRecord) => {
      const requestId = ++detailRequestRef.current;
      const inventoryEpoch = inventoryEpochRef.current;
      if (!api.getWorkRecordDetail) {
        setDetailError("Neutral Work Record detail is unavailable in this build");
        return;
      }
      const shouldLoadGenerated = indexed.generationSupported;
      const generatedRequest = shouldLoadGenerated
        ? Promise.resolve()
            .then(() => api.getGeneratedView(indexed.rootId, indexed.record.subjectId))
            .then(
              (view) => ({ status: "fulfilled" as const, view }),
              (cause: unknown) => ({ status: "rejected" as const, cause }),
            )
        : null;
      try {
        setDetailError(null);
        const detail = await api.getWorkRecordDetail(
          indexed.rootId,
          indexed.record.subjectId,
          indexed.indexGeneration,
        );
        if (requestId !== detailRequestRef.current || inventoryEpoch !== inventoryEpochRef.current)
          return;
        commitSelectedWorkRecord(detail);
        setGeneratedView(
          generatedInventory[indexed.record.subjectId] ?? { status: "never_generated" },
        );
        if (window.innerWidth <= 960) {
          setPaneLayout((layout) => ({ ...layout, ledgerCollapsed: true }));
        }
      } catch (cause) {
        if (requestId === detailRequestRef.current) setDetailError(errorMessage(cause));
        return;
      }
      if (!generatedRequest) return;
      const generated = await generatedRequest;
      if (requestId !== detailRequestRef.current || inventoryEpoch !== inventoryEpochRef.current)
        return;
      if (generated.status === "rejected") {
        setGeneratedView({
          status: "never_generated",
          capabilityReason: `Generated summary unavailable: ${errorMessage(generated.cause)}`,
        });
        return;
      }
      setGeneratedView(generated.view);
      setGeneratedInventory((inventory) => ({
        ...inventory,
        [indexed.record.subjectId]: generated.view,
      }));
    },
    [api, commitSelectedWorkRecord, generatedInventory],
  );

  const selectBundle = useCallback(
    async (bundle: IndexedBundle) => {
      const root = [...indexes]
        .sort((left, right) => left.rootId.localeCompare(right.rootId))
        .find((index) =>
          index.projects.some((project) =>
            project.bundles.some((candidate) => candidate.bundle.id === bundle.bundle.id),
          ),
        );
      const taskMember = bundle.bundle.members.find(
        (candidate) => fileLabel(candidate.relativePath) === "tasks.md",
      );
      const member =
        (bundle.progress.status === "available" ? taskMember : undefined) ??
        bundle.bundle.members.find(
          (candidate) => fileLabel(candidate.relativePath) === "proposal.md",
        ) ??
        bundle.bundle.members.find(
          (candidate) => fileLabel(candidate.relativePath) === "design.md",
        ) ??
        bundle.bundle.members.find(
          (candidate) => fileLabel(candidate.relativePath) !== "tasks.md",
        ) ??
        taskMember ??
        bundle.bundle.members[0];
      if (!root || !member) return;
      const requestId = ++detailRequestRef.current;
      const inventoryEpoch = inventoryEpochRef.current;
      const generatedRequest = Promise.resolve()
        .then(() => api.getGeneratedView(root.rootId, bundle.bundle.id))
        .then(
          (view) => ({ status: "fulfilled" as const, view }),
          (cause: unknown) => ({ status: "rejected" as const, cause }),
        );
      try {
        setDetailError(null);
        const detail = await api.getArtifactDetail(root.rootId, member.id);
        if (requestId !== detailRequestRef.current) return;
        commitSelectedArtifact(detail);
        setGeneratedView(generatedInventory[bundle.bundle.id] ?? { status: "never_generated" });
        if (window.innerWidth <= 960) {
          setPaneLayout((layout) => ({ ...layout, ledgerCollapsed: true }));
        }
      } catch (cause) {
        if (requestId === detailRequestRef.current) setDetailError(errorMessage(cause));
        return;
      }

      const generated = await generatedRequest;
      if (requestId !== detailRequestRef.current || inventoryEpoch !== inventoryEpochRef.current)
        return;
      if (generated.status === "rejected") {
        setGeneratedView({
          status: "never_generated",
          capabilityReason: `Generated summary unavailable: ${errorMessage(generated.cause)}`,
        });
        return;
      }
      setGeneratedView(generated.view);
      setGeneratedInventory((inventory) => ({
        ...inventory,
        [bundle.bundle.id]: generated.view,
      }));
    },
    [api, commitSelectedArtifact, generatedInventory, indexes],
  );

  const selectDocument = useCallback(
    async (document: IndexedMarkdownDocument) => {
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
    },
    [api, commitSelectedMarkdown],
  );

  useEffect(() => {
    const selectedId =
      selectedWorkRecord?.subjectId ?? selectedArtifact?.bundleId ?? selectedMarkdown?.documentId;
    if (!selectedId || visibleRecords.some((record) => record.id === selectedId)) return;
    ++detailRequestRef.current;
    selectedWorkRecordRef.current = null;
    selectedArtifactRef.current = null;
    setSelectedWorkRecord(null);
    setSelectedArtifact(null);
    setSelectedMarkdown(null);
    setGeneratedView({ status: "never_generated" });
    const fallback = visibleRecords[0];
    if (fallback?.kind === "record") void selectWorkRecord(fallback.record);
    if (fallback?.kind === "bundle") void selectBundle(fallback.bundle);
    if (fallback?.kind === "document") void selectDocument(fallback.document);
  }, [
    selectBundle,
    selectDocument,
    selectWorkRecord,
    selectedWorkRecord?.subjectId,
    selectedArtifact?.bundleId,
    selectedMarkdown?.documentId,
    visibleRecords,
  ]);

  const changeRegistryScope = (nextScope: RegistryScope) => {
    if (nextScope === registryScope) return;
    setRegistryScope(nextScope);
    setBundleFilter("current");
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

  useEffect(() => {
    let active = true;
    const subjectId = selectedWorkRecord?.subjectId;
    if (!subjectId || !api.getWorkRecordAnnotationTargets) {
      setStoredAnnotationTargets([]);
      return () => {
        active = false;
      };
    }
    api
      .getWorkRecordAnnotationTargets(subjectId)
      .then((targets) => {
        if (active) setStoredAnnotationTargets(targets);
      })
      .catch(() => {
        if (active) setStoredAnnotationTargets([]);
      });
    return () => {
      active = false;
    };
  }, [api, selectedWorkRecord?.subjectId]);

  const annotationTargets = useMemo(() => {
    if (!selectedWorkRecord) return [];
    const targets = new Map<string, AnnotationTarget>(
      storedAnnotationTargets.map((target) => [target.subjectId, target]),
    );
    for (const index of indexes) {
      for (const project of index.projects) {
        for (const record of project.records ?? []) {
          if (record.subjectId === selectedWorkRecord.subjectId) continue;
          targets.set(record.subjectId, {
            subjectId: record.subjectId,
            label: `${record.displayName} · ${project.project.name}`,
            exactLocatorKey: [
              record.locator.formatId,
              record.locator.projectId,
              record.locator.adapterRecordKey,
            ].join(":"),
            available: true,
          });
        }
      }
    }
    const disposition = selectedWorkRecord.record.annotation?.disposition;
    if (disposition?.status === "superseded" && !targets.has(disposition.replacement)) {
      targets.set(disposition.replacement, {
        subjectId: disposition.replacement,
        label: disposition.replacement,
        exactLocatorKey: disposition.replacement,
        available: false,
      });
    }
    return [...targets.values()].sort((left, right) => left.label.localeCompare(right.label));
  }, [indexes, selectedWorkRecord, storedAnnotationTargets]);

  const updateSelectedAnnotation = async (command: AnnotationCommand) => {
    const subjectId = selectedWorkRecord?.subjectId;
    if (!subjectId || !api.updateWorkRecordAnnotation) {
      setDetailError("Private annotation updates are unavailable in this build");
      return;
    }
    try {
      ++detailRequestRef.current;
      setDetailError(null);
      const annotation = await api.updateWorkRecordAnnotation(subjectId, command);
      if (selectedWorkRecordRef.current?.subjectId !== subjectId) return;
      const nextDetail = {
        ...selectedWorkRecordRef.current,
        record: { ...selectedWorkRecordRef.current.record, annotation },
      };
      selectedWorkRecordRef.current = nextDetail;
      setSelectedWorkRecord(nextDetail);
      setIndexes((current) =>
        current.map((index) => ({
          ...index,
          projects: index.projects.map((project) => ({
            ...project,
            records: project.records?.map((record) =>
              record.subjectId === subjectId ? { ...record, annotation } : record,
            ),
          })),
        })),
      );
    } catch (cause) {
      setDetailError(errorMessage(cause));
    }
  };

  const selectedWorkRoot = selectedWorkRecord
    ? indexes.find((index) => index.rootId === selectedWorkRecord.rootId)
    : undefined;
  const selectedRoot = selectedArtifact
    ? indexes.find((index) => index.rootId === selectedArtifact.rootId)
    : undefined;

  const runWorkRecordHandoff = async (action: "path" | "prompt" | "terminal") => {
    if (!selectedWorkRecord || !selectedWorkRoot) return;
    try {
      setDetailError(null);
      setHandoffNotice(null);
      if (action === "path") {
        if (!api.copyWorkRecordPath) throw new Error("Work Record path handoff is unavailable");
        await api.copyWorkRecordPath(
          selectedWorkRoot.rootId,
          selectedWorkRecord.subjectId,
          selectedWorkRecord.indexGeneration,
        );
        setHandoffNotice("Source path copied");
      } else if (action === "prompt") {
        if (!api.copyWorkRecordPrompt) throw new Error("Work Record prompt handoff is unavailable");
        await api.copyWorkRecordPrompt(
          selectedWorkRoot.rootId,
          selectedWorkRecord.subjectId,
          selectedWorkRecord.indexGeneration,
        );
        setHandoffNotice("Continuation prompt copied");
      } else {
        await api.openTerminal(selectedWorkRoot.rootId, selectedWorkRecord.projectId);
        setHandoffNotice("Terminal opened at the project root");
      }
    } catch (cause) {
      setDetailError(errorMessage(cause));
    }
  };

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
    const requestedId = selectedWorkRecord?.subjectId ?? selectedArtifact?.bundleId;
    const rootId = selectedWorkRecord?.rootId ?? selectedArtifact?.rootId;
    if (!requestedId || !rootId) return;
    const root = indexes.find((index) => index.rootId === rootId);
    if (!root) return;
    const inventoryEpoch = inventoryEpochRef.current;
    const previous = generatedResult(generatedView);
    const selectionIsCurrent = () =>
      selectedWorkRecord
        ? selectedWorkRecordRef.current?.subjectId === requestedId
        : selectedArtifactRef.current?.bundleId === requestedId;
    setGeneratedView({ status: "generating", ...(previous ? { previous } : {}) });
    try {
      const nextView = await api.requestSummary(root.rootId, requestedId);
      if (inventoryEpoch !== inventoryEpochRef.current) return;
      if (selectionIsCurrent()) setGeneratedView(nextView);
      setGeneratedInventory((inventory) => ({
        ...inventory,
        [requestedId]: nextView,
      }));
    } catch (cause) {
      if (inventoryEpoch !== inventoryEpochRef.current) return;
      const failed: GeneratedView = {
        status: "failed",
        ...(previous ? { previous } : {}),
        failure: errorMessage(cause),
      };
      if (selectionIsCurrent()) setGeneratedView(failed);
      setGeneratedInventory((inventory) => ({
        ...inventory,
        [requestedId]: failed,
      }));
    }
  };

  useEffect(() => savePaneLayout(paneLayout), [paneLayout]);

  const selectedReadingId =
    selectedWorkRecord?.subjectId ?? selectedMarkdown?.documentId ?? selectedArtifact?.bundleId;
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

  const statusLabel = workspaceStatusLabel(status, warnings.length + failedPatternRootIds.length);
  const hasSelection =
    selectedWorkRecord !== null || selectedArtifact !== null || selectedMarkdown !== null;
  const ledgerCollapsed = hasSelection && paneLayout.ledgerCollapsed;
  const ledgerToggleLabel = !hasSelection
    ? "Bundle ledger remains open until work is selected"
    : ledgerCollapsed
      ? "Show bundle ledger"
      : "Hide bundle ledger";

  return (
    <main className="app-frame">
      <header
        className={`titlebar ${appMode === "settings" ? "is-settings" : ""}`}
        inert={paletteOpen}
      >
        <div className="wordmark" aria-label="Backstage artifact control tower">
          <img className="brand-mark" src={backstageMark} alt="" />
          <strong>BACKSTAGE</strong>
          <span>Artifact Control Tower</span>
        </div>
        <div className="titlebar-actions">
          {appMode === "work" && (
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
          )}
          {appMode === "work" && (
            <span className={`system-state system-state--${status}`}>{statusLabel}</span>
          )}
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
            ref={settingsTriggerRef}
            className="button button--compact settings-control"
            type="button"
            aria-pressed={appMode === "settings"}
            onClick={() =>
              appMode === "settings" ? closeSettings() : openSettings(settingsTriggerRef.current)
            }
          >
            <GearSixIcon className="app-icon" aria-hidden="true" weight="regular" />
            <span>Settings</span>
          </button>
          {appMode === "work" && (
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
          )}
          {appMode === "work" && (
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
          )}
          {appMode === "work" && (
            <button
              className="button button--primary button--compact"
              type="button"
              disabled={settingsBusy}
              onClick={() => void approveRoot()}
            >
              Add root
            </button>
          )}
        </div>
      </header>

      {appMode === "settings" ? (
        <SettingsSurface
          headingRef={settingsHeadingRef}
          patternInputRef={planningPatternInputRef}
          roots={roots}
          indexes={indexes}
          patterns={patterns}
          patternRevision={patternRevision}
          patternsLoading={patternsLoading}
          settingsBusy={settingsBusy}
          failedPatternRootIds={failedPatternRootIds}
          confirmingRootId={confirmingRootId}
          removingRootId={removingRootId}
          error={settingsError}
          notice={settingsNotice}
          onDone={closeSettings}
          onAddRoot={approveRoot}
          onRetry={retryFailedPatternRoots}
          onConfirmRoot={setConfirmingRootId}
          onCancelRoot={() => setConfirmingRootId(null)}
          onRemoveRoot={removeApprovedRoot}
          onAddPattern={(expression) => applyPatternMutation(() => api.addPattern(expression))}
          onRemovePattern={(id) => applyPatternMutation(() => api.removePattern(id), true)}
          onRestoreDefaults={() => applyPatternMutation(() => api.restoreDefaultPatterns())}
        />
      ) : (
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
              {(
                [
                  ["current", "Current"],
                  ["active", "Active"],
                  ["done", "Done"],
                  ["archived", "Archived"],
                  ["warning", "Warning-bearing"],
                  ["stale", "Stale"],
                ] as const
              ).map(([filter, label]) => (
                <button
                  key={filter}
                  type="button"
                  className={bundleFilter === filter ? "is-selected" : ""}
                  aria-pressed={bundleFilter === filter}
                  onClick={() => {
                    ++detailRequestRef.current;
                    setBundleFilter(filter);
                  }}
                >
                  {label}
                </button>
              ))}
              <label className="annotation-filter">
                Private annotation
                <select
                  aria-label="Filter by private annotation"
                  value={annotationFilter}
                  onChange={(event) => setAnnotationFilter(event.target.value as AnnotationFilter)}
                >
                  <option value="all">All annotations</option>
                  <option value="undecided">Undecided</option>
                  <option value="approved">Approved</option>
                  <option value="rejected">Rejected</option>
                  <option value="applicable">Applicable</option>
                  <option value="obsolete">Obsolete</option>
                  <option value="superseded">Superseded</option>
                  <option value="favorite">Favorite</option>
                  <option value="todo">Todo</option>
                  <option value="priority_low">Low priority</option>
                  <option value="priority_medium">Medium priority</option>
                  <option value="priority_high">High priority</option>
                </select>
              </label>
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
                {displayedGroups.map((group) => (
                  <section
                    className="ledger-group"
                    aria-labelledby={`ledger-group-${slug(group.label)}`}
                    key={group.label}
                  >
                    <h2 id={`ledger-group-${slug(group.label)}`}>{group.label}</h2>
                    {group.records.map((record) =>
                      record.kind === "record" ? (
                        <WorkRecordRow
                          key={record.id}
                          indexed={record.record}
                          now={currentTime}
                          selected={
                            selectedWorkRecord?.subjectId === record.record.record.subjectId
                          }
                          onSelect={() => void selectWorkRecord(record.record)}
                          onNavigate={navigateLedgerRows}
                        />
                      ) : record.kind === "bundle" ? (
                        <BundleRow
                          key={record.id}
                          bundle={record.bundle}
                          now={currentTime}
                          selected={selectedArtifact?.bundleId === record.bundle.bundle.id}
                          onSelect={() => void selectBundle(record.bundle)}
                          onNavigate={navigateLedgerRows}
                        />
                      ) : (
                        <DocumentRow
                          key={record.id}
                          document={record.document}
                          now={currentTime}
                          selected={selectedMarkdown?.documentId === record.document.id}
                          onSelect={() => void selectDocument(record.document)}
                          onNavigate={navigateLedgerRows}
                        />
                      ),
                    )}
                  </section>
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
            {selectedWorkRecord ? (
              <>
                <WorkRecordReadingDesk
                  detail={selectedWorkRecord}
                  annotationTargets={annotationTargets}
                  onUpdateAnnotation={(command) => void updateSelectedAnnotation(command)}
                  onOpenAnnotationTarget={(subjectId) => {
                    for (const index of indexes) {
                      for (const project of index.projects) {
                        const target = project.records?.find(
                          (record) => record.subjectId === subjectId,
                        );
                        if (target) {
                          void selectWorkRecord({
                            rootId: index.rootId,
                            indexGeneration: index.generation,
                            project: project.project,
                            record: target,
                            generationSupported: project.bundles.some((bundle) =>
                              samePaths(
                                target.sources.map((source) => source.relativePath),
                                bundle.bundle.members.map((member) => member.relativePath),
                              ),
                            ),
                          });
                          return;
                        }
                      }
                    }
                  }}
                  onCopyPath={() => void runWorkRecordHandoff("path")}
                  onCopyPrompt={() => void runWorkRecordHandoff("prompt")}
                  onOpenTerminal={() => void runWorkRecordHandoff("terminal")}
                  onRescan={async () => {
                    await scan(roots);
                  }}
                />
                {recordSupportsGeneration(
                  indexes,
                  selectedWorkRecord.rootId,
                  selectedWorkRecord.subjectId,
                ) && <GeneratedSummary view={generatedView} onRequest={requestSummary} />}
              </>
            ) : selectedArtifact ? (
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
                settingsBusy={settingsBusy}
                onApprove={approveRoot}
                onRefresh={async () => {
                  await scan(roots);
                }}
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
      )}
      {paletteOpen && (
        <CommandPalette
          inputRef={paletteInputRef}
          query={paletteQuery}
          onQueryChange={setPaletteQuery}
          onClose={closePalette}
          onRefresh={() => void scan(roots)}
          onApprove={() => void approveRoot()}
          onOpenSettings={() => openSettings(paletteTriggerRef.current)}
          onToggleLedger={() =>
            setPaneLayout((layout) => ({ ...layout, ledgerCollapsed: !ledgerCollapsed }))
          }
          canApprove={!settingsBusy}
          canToggleLedger={hasSelection}
          canRefresh={roots.length > 0 && status !== "scanning"}
        />
      )}
    </main>
  );
}

function SettingsSurface({
  headingRef,
  patternInputRef,
  roots,
  indexes,
  patterns,
  patternRevision,
  patternsLoading,
  settingsBusy,
  failedPatternRootIds,
  confirmingRootId,
  removingRootId,
  error,
  notice,
  onDone,
  onAddRoot,
  onRetry,
  onConfirmRoot,
  onCancelRoot,
  onRemoveRoot,
  onAddPattern,
  onRemovePattern,
  onRestoreDefaults,
}: {
  headingRef: RefObject<HTMLHeadingElement | null>;
  patternInputRef: RefObject<HTMLInputElement | null>;
  roots: ApprovedRoot[];
  indexes: IndexSnapshot[];
  patterns: PlanningPattern[];
  patternRevision: number;
  patternsLoading: boolean;
  settingsBusy: boolean;
  failedPatternRootIds: string[];
  confirmingRootId: string | null;
  removingRootId: string | null;
  error: string | null;
  notice: string | null;
  onDone: () => void;
  onAddRoot: () => Promise<void>;
  onRetry: () => Promise<void>;
  onConfirmRoot: (rootId: string) => void;
  onCancelRoot: () => void;
  onRemoveRoot: (rootId: string) => Promise<void>;
  onAddPattern: (expression: string) => Promise<void>;
  onRemovePattern: (id: string) => Promise<void>;
  onRestoreDefaults: () => Promise<void>;
}) {
  const [expression, setExpression] = useState("");
  return (
    <section className="settings-surface" aria-labelledby="settings-heading">
      <header className="settings-heading">
        <div>
          <h1 id="settings-heading" ref={headingRef} tabIndex={-1}>
            Settings
          </h1>
          <p>Manage app-owned approvals and planning conventions. Repositories remain read-only.</p>
        </div>
        <button className="button button--primary" type="button" onClick={onDone}>
          Done
        </button>
      </header>

      {error && (
        <p className="settings-feedback settings-feedback--error" role="alert">
          {error} Try the action again; existing roots and indexes remain available.
        </p>
      )}
      {notice && (
        <p className="settings-feedback settings-feedback--notice" role="status">
          {notice}
        </p>
      )}

      <section className="settings-section" aria-labelledby="approved-roots-heading">
        <header>
          <div>
            <h2 id="approved-roots-heading">Approved roots</h2>
            <p>Backstage scans only these folders and keeps indexes in app-owned storage.</p>
          </div>
          <button
            className="button button--primary"
            type="button"
            disabled={settingsBusy}
            onClick={() => void onAddRoot()}
          >
            Add root
          </button>
        </header>
        {roots.length === 0 ? (
          <div className="settings-empty">
            <strong>No folders are being scanned.</strong>
            <p>Add a root to discover local planning work without changing repository files.</p>
          </div>
        ) : (
          <ul className="settings-register root-register">
            {roots.map((root) => (
              <ApprovedRootRow
                key={root.id}
                root={root}
                index={indexes.find((candidate) => candidate.rootId === root.id)}
                failed={failedPatternRootIds.includes(root.id)}
                confirming={confirmingRootId === root.id}
                rootBusy={settingsBusy}
                removing={removingRootId === root.id}
                onRetry={onRetry}
                onConfirm={() => onConfirmRoot(root.id)}
                onCancel={onCancelRoot}
                onRemove={() => onRemoveRoot(root.id)}
              />
            ))}
          </ul>
        )}
      </section>

      <section className="settings-section" aria-labelledby="planning-patterns-heading">
        <header>
          <div>
            <h2 id="planning-patterns-heading">Planning patterns</h2>
            <p>
              Rust-compatible regular expressions match normalized project-relative Markdown paths.
              Valid broad patterns are allowed and may classify every in-scope Markdown file as
              planning work.
            </p>
          </div>
          <button
            className="button"
            type="button"
            disabled={settingsBusy}
            onClick={() => void onRestoreDefaults()}
          >
            Restore defaults
          </button>
        </header>
        <form
          className="pattern-form"
          onSubmit={(event) => {
            event.preventDefault();
            void onAddPattern(expression);
          }}
        >
          <label htmlFor="planning-pattern-expression">Regular expression</label>
          <div>
            <input
              ref={patternInputRef}
              id="planning-pattern-expression"
              value={expression}
              onChange={(event) => setExpression(event.target.value)}
              placeholder="^docs/plans/.*\\.md$"
              disabled={settingsBusy}
            />
            <button
              className="button button--primary"
              type="submit"
              disabled={settingsBusy || expression.length === 0}
            >
              {settingsBusy ? "Settings busy…" : "Add pattern"}
            </button>
          </div>
          <small>
            Configuration revision {patternRevision}. This changes app-owned configuration and
            indexes only; it never writes matching repositories.
          </small>
        </form>
        {failedPatternRootIds.length > 0 && (
          <p className="settings-feedback settings-feedback--warning" role="alert">
            {failedPatternRootIds.length} approved{" "}
            {failedPatternRootIds.length === 1 ? "root" : "roots"} could not be rescanned. The last
            successful index remains available; retry Refresh.
          </p>
        )}
        {patternsLoading ? (
          <div className="settings-empty" aria-live="polite">
            <strong>Loading planning patterns…</strong>
          </div>
        ) : patterns.length === 0 ? (
          <div className="settings-empty">
            <strong>No planning patterns configured.</strong>
            <p>OpenSpec recognition and All Markdown indexing continue independently.</p>
          </div>
        ) : (
          <ul className="settings-register pattern-register">
            {[...patterns]
              .sort(
                (left, right) => left.ordinal - right.ordinal || left.id.localeCompare(right.id),
              )
              .map((pattern) => (
                <li key={pattern.id}>
                  <div className="settings-row">
                    <div className="settings-row-copy">
                      <code title={pattern.expression}>{pattern.expression}</code>
                      <span>{pattern.provenance === "default" ? "Default" : "Custom"}</span>
                    </div>
                    <button
                      className="button"
                      type="button"
                      disabled={settingsBusy}
                      onClick={() => void onRemovePattern(pattern.id)}
                    >
                      Remove
                    </button>
                  </div>
                </li>
              ))}
          </ul>
        )}
      </section>
    </section>
  );
}

function ApprovedRootRow({
  root,
  index,
  failed,
  confirming,
  rootBusy,
  removing,
  onRetry,
  onConfirm,
  onCancel,
  onRemove,
}: {
  root: ApprovedRoot;
  index: IndexSnapshot | undefined;
  failed: boolean;
  confirming: boolean;
  rootBusy: boolean;
  removing: boolean;
  onRetry: () => Promise<void>;
  onConfirm: () => void;
  onCancel: () => void;
  onRemove: () => Promise<void>;
}) {
  const removeTriggerRef = useRef<HTMLButtonElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const projectCount = index?.projects.length ?? 0;

  useEffect(() => {
    if (confirming) cancelRef.current?.focus();
  }, [confirming]);

  const cancel = () => {
    onCancel();
    removeTriggerRef.current?.focus();
  };

  return (
    <li>
      <div className="settings-row">
        <div className="settings-row-copy">
          <code title={root.path} aria-label={`Approved root ${root.path}`}>
            {root.path}
          </code>
          <span>
            {failed
              ? "Rescan failed · last successful index retained"
              : index
                ? `${projectCount} ${projectCount === 1 ? "project" : "projects"} indexed · revision ${index.configurationRevision}`
                : "Index unavailable · retry scan"}
          </span>
        </div>
        <div className="settings-row-actions">
          {(failed || !index) && (
            <button
              className="button"
              type="button"
              disabled={rootBusy}
              onClick={() => void onRetry()}
            >
              Retry
            </button>
          )}
          <button
            ref={removeTriggerRef}
            className="button"
            type="button"
            disabled={rootBusy}
            onClick={onConfirm}
          >
            {removing ? "Removing…" : "Remove"}
          </button>
        </div>
      </div>
      {confirming && (
        <section
          className="removal-confirmation"
          role="alertdialog"
          aria-labelledby={`remove-root-${root.id}`}
          onKeyDown={(event) => {
            if (event.key !== "Escape" || rootBusy) return;
            event.preventDefault();
            event.stopPropagation();
            cancel();
          }}
        >
          <div>
            <h3 id={`remove-root-${root.id}`} tabIndex={-1}>
              Remove approved root?
            </h3>
            <p>
              Backstage forgets approval, index, and unreachable generated summaries. Repository
              files remain untouched.
            </p>
          </div>
          <div className="button-row">
            <button
              ref={cancelRef}
              className="button"
              type="button"
              disabled={rootBusy}
              onClick={cancel}
            >
              Cancel
            </button>
            <button
              className="button button--primary"
              type="button"
              disabled={rootBusy}
              onClick={() => void onRemove()}
            >
              Remove approval
            </button>
          </div>
        </section>
      )}
    </li>
  );
}

interface WorkspaceContentProps {
  status: WorkspaceStatus;
  roots: ApprovedRoot[];
  projects: Project[];
  scope: RegistryScope;
  warnings: ScanWarning[];
  error: string | null;
  settingsBusy: boolean;
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
  settingsBusy,
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
          <button
            className="button button--primary"
            type="button"
            disabled={settingsBusy}
            onClick={() => void onApprove()}
          >
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
  onOpenSettings,
  onToggleLedger,
  canApprove,
  canToggleLedger,
  canRefresh,
}: {
  inputRef: RefObject<HTMLInputElement | null>;
  query: string;
  onQueryChange: (query: string) => void;
  onClose: (restoreFocus?: boolean) => void;
  onRefresh: () => void;
  onApprove: () => void;
  onOpenSettings: () => void;
  onToggleLedger: () => void;
  canApprove: boolean;
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
    { label: "Approve another root", hint: "", disabled: !canApprove, run: onApprove },
    { label: "Open Settings", hint: "", disabled: false, run: onOpenSettings },
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
                onClose(
                  command.label !== "Search indexed work" && command.label !== "Open Settings",
                );
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
  const primaryStatus =
    detail.bundleKind === "open_spec_change" ? detailPrimaryStatus(detail) : null;
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
            <span className="artifact-lifecycle">{primaryStatusLabel(primaryStatus!)}</span>
            <span>{detail.projectName}</span>
            <span>{detail.git ? detail.git.branch : "Git unavailable"}</span>
            <span>
              {progress
                ? `${progress.remainingCount} open · ${progress.completed} done`
                : "Progress unavailable"}
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
                    ? progress
                      ? `Tasks ${progress.total}`
                      : "Tasks"
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
        <>
          <p>
            No repository content has been sent to Pi. Generate only when you want a bounded
            snapshot explained.
          </p>
          {view.capabilityReason && (
            <p className="generated-failure">
              {view.capabilityReason}. Artifact detail remains available; retry generation when the
              service recovers.
            </p>
          )}
        </>
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

function WorkRecordRow({
  indexed,
  now,
  selected,
  onSelect,
  onNavigate,
}: {
  indexed: IndexedWorkRecord;
  now: Date;
  selected: boolean;
  onSelect: () => void;
  onNavigate: (event: ReactKeyboardEvent<HTMLButtonElement>) => void;
}) {
  const { record, project } = indexed;
  const sourceDate = formatLedgerDate(record.sourceModifiedUnixNanos, now);
  const status = recordFactText(record, "openspec.primary_status");
  const open = recordFactCount(record, "openspec.task.open_count");
  const done = recordFactCount(record, "openspec.task.done_count");
  const warningCount = record.warnings.length;
  const annotation = record.annotation ?? defaultRecordAnnotation();
  return (
    <button
      className={`bundle-row work-record-row ${selected ? "is-selected" : ""}`}
      type="button"
      data-ledger-row="true"
      aria-pressed={selected}
      onClick={onSelect}
      onKeyDown={onNavigate}
    >
      <span className="bundle-row-top">
        <span className={`bundle-kind bundle-kind--${record.locator.formatId}`}>
          {record.recognition.level === "plain"
            ? "Markdown document"
            : record.locator.formatId === "openspec"
              ? "OpenSpec"
              : record.locator.formatId === "wayfinder-local"
                ? "Wayfinder"
                : "Planning candidate"}
        </span>
        {warningCount > 0 && (
          <span className="bundle-warning">
            {warningCount} {warningCount === 1 ? "warning" : "warnings"}
          </span>
        )}
      </span>
      <strong>{record.displayName}</strong>
      <span className="annotation-badges" aria-label="Private annotations">
        <span>{titleCase(annotation.decision)}</span>
        <span>{titleCase(annotation.disposition.status)}</span>
        {annotation.favorite && <span>Favorite</span>}
        {annotation.todo && <span>Todo</span>}
        {annotation.priority && <span>{titleCase(annotation.priority)} priority</span>}
      </span>
      <span className="bundle-primary-meta">
        <span>
          {isOpenSpecStatus(status) && (
            <strong className="bundle-lifecycle">{primaryStatusLabel(status)}</strong>
          )}
          {open !== null && done !== null && (
            <span className="bundle-task-counts">
              {open} open · {done} done
            </span>
          )}
          {record.recognition.level === "possible" && (
            <span className="bundle-provenance">
              {record.recognition.evidence[0] ?? "Matched configured planning pattern"}
            </span>
          )}
        </span>
        <time dateTime={sourceDate.dateTime} aria-label={sourceDate.full} title={sourceDate.full}>
          {sourceDate.concise}
        </time>
      </span>
      <small>{project.name}</small>
    </button>
  );
}

function DocumentRow({
  document,
  now,
  selected,
  onSelect,
  onNavigate,
}: {
  document: IndexedMarkdownDocument;
  now: Date;
  selected: boolean;
  onSelect: () => void;
  onNavigate: (event: ReactKeyboardEvent<HTMLButtonElement>) => void;
}) {
  const sourceDate = formatLedgerDate(document.sourceModifiedUnixNanos, now);
  return (
    <button
      className={`bundle-row document-row ${selected ? "is-selected" : ""}`}
      type="button"
      data-ledger-row="true"
      aria-pressed={selected}
      onClick={onSelect}
      onKeyDown={onNavigate}
    >
      <span className="bundle-row-top">
        <span className="bundle-kind bundle-kind--markdown">Markdown document</span>
      </span>
      <strong>{fileLabel(document.relativePath)}</strong>
      <span className="bundle-primary-meta">
        <span className="bundle-provenance">{document.projectName}</span>
        <time dateTime={sourceDate.dateTime} aria-label={sourceDate.full} title={sourceDate.full}>
          {sourceDate.concise}
        </time>
      </span>
      <small className="bundle-path">{document.relativePath}</small>
    </button>
  );
}

function BundleRow({
  bundle,
  now,
  selected,
  onSelect,
  onNavigate,
}: {
  bundle: IndexedBundle;
  now: Date;
  selected: boolean;
  onSelect: () => void;
  onNavigate: (event: ReactKeyboardEvent<HTMLButtonElement>) => void;
}) {
  const progress = bundle.progress.status === "available" ? bundle.progress.progress : null;
  const warningCount = bundle.warnings.length + bundle.progress.progress.warnings.length;
  const isOpenSpec = bundle.bundle.kind === "open_spec_change";
  const label = isOpenSpec ? "OpenSpec" : "Planning candidate";
  const status = isOpenSpec ? bundlePrimaryStatus(bundle) : null;
  const sourceDate = formatLedgerDate(bundle.sourceModifiedUnixNanos, now);
  return (
    <button
      className={`bundle-row ${selected ? "is-selected" : ""}`}
      type="button"
      data-ledger-row="true"
      aria-pressed={selected}
      onClick={onSelect}
      onKeyDown={onNavigate}
    >
      <span className="bundle-row-top">
        <span className={`bundle-kind bundle-kind--${bundle.bundle.kind}`}>{label}</span>
        {warningCount > 0 && (
          <span className="bundle-warning">
            {warningCount} {warningCount === 1 ? "warning" : "warnings"}
          </span>
        )}
      </span>
      <strong>{bundle.bundle.name}</strong>
      <span className="bundle-primary-meta">
        <span>
          {status && <strong className="bundle-lifecycle">{primaryStatusLabel(status)}</strong>}
          {isOpenSpec && (
            <span className="bundle-task-counts">
              {progress
                ? `${progress.remainingCount} open · ${progress.completed} done`
                : "Progress unavailable"}
            </span>
          )}
          {!isOpenSpec && (
            <span className="bundle-provenance">
              {candidateEvidenceLabel(
                bundle.bundle.recognition.status === "possible"
                  ? bundle.bundle.recognition.reason
                  : undefined,
              )}
            </span>
          )}
        </span>
        <time dateTime={sourceDate.dateTime} aria-label={sourceDate.full} title={sourceDate.full}>
          {sourceDate.concise}
        </time>
      </span>
      <small>{bundle.bundle.projectName}</small>
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

function bundlePrimaryStatus(bundle: IndexedBundle): OpenSpecPrimaryStatus {
  if (bundle.primaryStatus) return bundle.primaryStatus;
  if (bundle.bundle.custody?.status === "archived") return "archived";
  return bundle.progress.status === "available" && bundle.progress.progress.remainingCount === 0
    ? "done"
    : "active";
}

function detailPrimaryStatus(detail: ArtifactDetail): OpenSpecPrimaryStatus {
  if (detail.primaryStatus) return detail.primaryStatus;
  if (detail.custody?.status === "archived") return "archived";
  return detail.progress.status === "available" && detail.progress.progress.remainingCount === 0
    ? "done"
    : "active";
}

function primaryStatusLabel(status: OpenSpecPrimaryStatus) {
  return status[0]!.toUpperCase() + status.slice(1);
}

function samePaths(left: string[], right: string[]) {
  if (left.length !== right.length) return false;
  const expected = new Set(left);
  return expected.size === right.length && right.every((path) => expected.has(path));
}

function recordSupportsGeneration(indexes: IndexSnapshot[], rootId: string, subjectId: string) {
  for (const index of indexes) {
    if (index.rootId !== rootId) continue;
    for (const project of index.projects) {
      const record = project.records?.find((candidate) => candidate.subjectId === subjectId);
      if (
        record &&
        project.bundles.some((bundle) =>
          samePaths(
            record.sources.map((source) => source.relativePath),
            bundle.bundle.members.map((member) => member.relativePath),
          ),
        )
      ) {
        return true;
      }
    }
  }
  return false;
}

function defaultRecordAnnotation(): WorkRecordAnnotation {
  return {
    decision: "undecided",
    disposition: { status: "applicable" },
    favorite: false,
    todo: false,
    priority: null,
  };
}

function recordMatchesAnnotation(record: WorkRecord, filter: AnnotationFilter) {
  if (filter === "all") return true;
  const annotation = record.annotation ?? defaultRecordAnnotation();
  if (filter === "undecided" || filter === "approved" || filter === "rejected") {
    return annotation.decision === filter;
  }
  if (filter === "applicable" || filter === "obsolete" || filter === "superseded") {
    return annotation.disposition.status === filter;
  }
  if (filter === "favorite") return annotation.favorite;
  if (filter === "todo") return annotation.todo;
  return annotation.priority === filter.replace("priority_", "");
}

function recordInScope(record: WorkRecord, scope: RegistryScope) {
  return scope === "markdown" || record.recognition.level !== "plain";
}

function recordSearchText(record: WorkRecord, project: Project) {
  return [
    record.displayName,
    project.name,
    record.locator.formatId,
    record.locator.adapterRecordKey,
    ...record.sources.map((source) => source.relativePath),
    ...record.facts.flatMap((fact) => [fact.key, fact.label, String(fact.value.value)]),
  ]
    .join(" ")
    .toLowerCase();
}

function recordFactText(record: WorkRecord, key: string) {
  const value = record.facts.find((fact) => fact.key === key)?.value;
  return value?.type === "text" ? value.value : null;
}

function recordFactCount(record: WorkRecord, key: string) {
  const value = record.facts.find((fact) => fact.key === key)?.value;
  return value?.type === "count" ? value.value : null;
}

function isOpenSpecStatus(value: string | null): value is OpenSpecPrimaryStatus {
  return value === "active" || value === "done" || value === "archived";
}

function recordMatchesFilter(
  record: WorkRecord,
  filter: BundleFilter,
  generatedInventory: Record<string, GeneratedView>,
) {
  const status = recordFactText(record, "openspec.primary_status");
  switch (filter) {
    case "current":
      return status !== "archived";
    case "active":
      return status === "active";
    case "done":
      return status === "done";
    case "archived":
      return status === "archived";
    case "warning":
      return record.warnings.length > 0;
    case "stale":
      return generatedInventory[record.subjectId]?.status === "stale";
  }
}

function bundleMatchesFilter(
  bundle: IndexedBundle,
  filter: BundleFilter,
  generatedInventory: Record<string, GeneratedView>,
) {
  const isOpenSpec = bundle.bundle.kind === "open_spec_change";
  const status = isOpenSpec ? bundlePrimaryStatus(bundle) : null;
  switch (filter) {
    case "current":
      return status !== "archived";
    case "active":
      return status === "active";
    case "done":
      return status === "done";
    case "archived":
      return status === "archived";
    case "warning":
      return bundle.warnings.length > 0 || bundle.progress.progress.warnings.length > 0;
    case "stale":
      return generatedInventory[bundle.bundle.id]?.status === "stale";
  }
}

function formatLedgerDate(unixNanos: SourceTimestamp, now: Date) {
  const milliseconds = validSourceMilliseconds(unixNanos);
  if (milliseconds === null) {
    return { concise: "Date unavailable", full: "Source date unavailable", dateTime: undefined };
  }
  const date = new Date(milliseconds);
  return {
    concise: date.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      year: date.getFullYear() === now.getFullYear() ? undefined : "numeric",
      hour: "numeric",
      minute: "2-digit",
    }),
    full: date.toLocaleString(undefined, { dateStyle: "full", timeStyle: "short" }),
    dateTime: date.toISOString(),
  };
}

function navigateLedgerRows(event: ReactKeyboardEvent<HTMLButtonElement>) {
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
  const rows = Array.from(document.querySelectorAll<HTMLButtonElement>('[data-ledger-row="true"]'));
  const current = rows.indexOf(event.currentTarget);
  const next =
    event.key === "Home"
      ? rows[0]
      : event.key === "End"
        ? rows.at(-1)
        : event.key === "ArrowDown"
          ? rows[current + 1]
          : rows[current - 1];
  if (!next) return;
  event.preventDefault();
  next.focus();
}

function titleCase(value: string) {
  return value[0]!.toUpperCase() + value.slice(1).replaceAll("_", " ");
}

function slug(value: string) {
  return value.toLowerCase().replaceAll(" ", "-");
}

function fileLabel(path: string) {
  const parts = path.split("/");
  if (path.includes("/specs/") && parts.length >= 2) {
    return `specs/${parts.at(-2)}/${parts.at(-1)}`;
  }
  return parts.at(-1) ?? path;
}

function formatSourceDate(unixNanos: SourceTimestamp) {
  const milliseconds = validSourceMilliseconds(unixNanos);
  return milliseconds === null
    ? "Source date unavailable"
    : new Date(milliseconds).toLocaleString();
}

function shortFingerprint(fingerprint: string) {
  return fingerprint.length > 22
    ? `${fingerprint.slice(0, 16)}…${fingerprint.slice(-6)}`
    : fingerprint;
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
