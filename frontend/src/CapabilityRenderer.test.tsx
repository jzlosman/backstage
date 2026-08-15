import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { WorkRecordReadingDesk } from "./CapabilityRenderer";
import type { WorkRecordDetail } from "./api";

afterEach(cleanup);

const detail: WorkRecordDetail = {
  rootId: "root_1",
  subjectId: "subject_1",
  indexGeneration: 7,
  projectId: "project_1",
  projectName: "Workbench",
  projectRoot: "/tmp/workbench",
  git: { branch: "main" },
  record: {
    subjectId: "subject_1",
    locator: {
      projectId: "project_1",
      formatId: "openspec",
      adapterRecordKey: "openspec/changes/search",
    },
    displayName: "search",
    recognition: {
      level: "recognized",
      adapterId: "openspec-v1",
      adapterVersion: 1,
      evidence: ["OpenSpec change material"],
    },
    sources: [
      { relativePath: "proposal.md", sourceModifiedUnixNanos: "7" },
      { relativePath: "tasks.md", sourceModifiedUnixNanos: "8" },
    ],
    facts: [],
    warnings: [],
    capabilities: [
      { id: "overview", label: "Overview" },
      { id: "tasks", label: "Tasks" },
      { id: "source", label: "Source" },
    ],
    sourceModifiedUnixNanos: "8",
    fingerprint: "sha256:indexed",
  },
  capabilities: [
    {
      capability: { id: "overview", label: "Overview" },
      blocks: [
        {
          kind: "markdown_section",
          id: "why",
          title: "Why",
          markdown: "Trusted context<script>alert(1)</script>",
          source: { relativePath: "proposal.md" },
        },
        {
          kind: "fact_register",
          id: "facts",
          title: "Facts",
          facts: [
            {
              key: "openspec.primary_status",
              label: "Status",
              value: { type: "text", value: "active" },
              provenance: { adapterId: "openspec-v1", sourcePaths: ["tasks.md"] },
            },
          ],
        },
      ],
    },
    {
      capability: { id: "tasks", label: "Tasks" },
      blocks: [
        { kind: "progress", id: "progress", label: "Tasks", completed: 1, total: 2 },
        {
          kind: "item_collection",
          id: "items",
          title: "Foundation",
          items: [
            {
              id: "task-1",
              title: "Keep source readable",
              source: { relativePath: "tasks.md", line: 4 },
              facts: [
                {
                  key: "openspec.task.completed",
                  label: "Completed",
                  value: { type: "boolean", value: false },
                  provenance: { adapterId: "openspec-v1", sourcePaths: ["tasks.md"] },
                },
              ],
              relationships: [],
            },
          ],
        },
        {
          kind: "relationship_list",
          id: "relations",
          title: "Relationships",
          relationships: [],
        },
        { kind: "empty_state", id: "empty", message: "No additional relationships" },
        {
          kind: "warning",
          id: "warning",
          warning: { code: "partial", message: "One source is partial", sourcePath: "tasks.md" },
        },
      ],
    },
    {
      capability: { id: "source", label: "Source" },
      blocks: [
        {
          kind: "source_list",
          id: "sources",
          title: "Source",
          sources: [{ relativePath: "proposal.md" }, { relativePath: "tasks.md" }],
        },
      ],
    },
  ],
  fingerprint: "sha256:fresh",
  warnings: [],
};

describe("WorkRecordReadingDesk", () => {
  it("renders neutral capabilities without executing unsafe Markdown", () => {
    const { container } = render(<WorkRecordReadingDesk detail={detail} />);

    expect(screen.getByRole("heading", { name: "search" })).toBeInTheDocument();
    expect(screen.getByText("Trusted context")).toBeInTheDocument();
    expect(container.querySelector("script")).toBeNull();
    expect(screen.getByText("Status")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Tasks" }));
    expect(screen.getByText("1 of 2 complete")).toBeInTheDocument();
    expect(screen.getByText("Keep source readable")).toBeInTheDocument();
    expect(screen.getByText("line 4")).toBeInTheDocument();
    expect(screen.getByText("One source is partial")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "Source" }));
    const sourcePanel = screen.getByRole("tabpanel");
    expect(within(sourcePanel).getByText("proposal.md")).toBeInTheDocument();
    expect(within(sourcePanel).getByText("tasks.md")).toBeInTheDocument();
  });

  it("supports arrow-key capability navigation", () => {
    render(<WorkRecordReadingDesk detail={detail} />);
    const overview = screen.getByRole("tab", { name: "Overview" });

    fireEvent.keyDown(overview, { key: "ArrowRight" });

    expect(screen.getByRole("tab", { name: "Tasks" })).toHaveAttribute("aria-selected", "true");
  });

  it("renders Wayfinder frontier, ticket facts, blockers, answers, warnings, and sanitized source", () => {
    const wayfinder: WorkRecordDetail = {
      ...detail,
      record: {
        ...detail.record,
        locator: {
          projectId: "project_1",
          formatId: "wayfinder-local",
          adapterRecordKey: ".scratch/search",
        },
        displayName: "search effort",
        recognition: {
          level: "recognized",
          adapterId: "wayfinder-local-v1",
          adapterVersion: 1,
          evidence: ["Exact local map"],
        },
        capabilities: [
          { id: "overview", label: "Overview" },
          { id: "questions", label: "Questions" },
          { id: "source", label: "Source" },
        ],
      },
      warnings: [
        {
          code: "adapter_claim_overlap",
          message: "Wayfinder won an overlapping source claim",
          sourcePath: ".scratch/search/map.md",
        },
      ],
      capabilities: [
        {
          capability: { id: "overview", label: "Overview" },
          blocks: [
            {
              kind: "markdown_section",
              id: "destination",
              title: "Destination",
              markdown: "Ship safely.<script>alert(1)</script>",
              source: { relativePath: ".scratch/search/map.md", line: 3 },
            },
            {
              kind: "fact_register",
              id: "frontier",
              title: "Frontier",
              facts: [
                {
                  key: "wayfinder.frontier.next",
                  label: "Next candidate",
                  value: { type: "text", value: "#2" },
                  provenance: {
                    adapterId: "wayfinder-local-v1",
                    sourcePaths: [".scratch/search/issues/02-build.md"],
                  },
                },
              ],
            },
          ],
        },
        {
          capability: { id: "questions", label: "Questions" },
          blocks: [
            {
              kind: "item_collection",
              id: "questions",
              title: "Questions",
              items: [
                {
                  id: "ticket-2",
                  title: "#2 How should it ship?",
                  markdown: "How should it ship?\n\n### Answer\nUse bounded reads.",
                  source: { relativePath: ".scratch/search/issues/02-build.md", line: 6 },
                  facts: [
                    {
                      key: "wayfinder.ticket.status",
                      label: "Status",
                      value: { type: "text", value: "resolved" },
                      provenance: {
                        adapterId: "wayfinder-local-v1",
                        sourcePaths: [".scratch/search/issues/02-build.md"],
                      },
                    },
                  ],
                  relationships: [
                    {
                      kind: "blocked_by",
                      targetSubjectId: "ticket_1",
                      label: "Blocked by #1",
                    },
                  ],
                },
              ],
            },
            {
              kind: "warning",
              id: "warning",
              warning: {
                code: "wayfinder_blocker_unresolved",
                message: "Blocker #1 is unavailable",
                sourcePath: ".scratch/search/issues/02-build.md",
              },
            },
          ],
        },
        {
          capability: { id: "source", label: "Source" },
          blocks: [
            {
              kind: "markdown_section",
              id: "source-map",
              title: ".scratch/search/map.md",
              markdown: "[Remote](https://example.com) <img src=x onerror=alert(1)>",
              source: { relativePath: ".scratch/search/map.md" },
            },
          ],
        },
      ],
    };
    const { container } = render(<WorkRecordReadingDesk detail={wayfinder} />);

    expect(screen.getByText("Ship safely.")).toBeInTheDocument();
    expect(screen.getByText("#2")).toBeInTheDocument();
    expect(screen.getByText("Wayfinder won an overlapping source claim")).toBeInTheDocument();
    expect(container.querySelector("script")).toBeNull();
    fireEvent.keyDown(screen.getByRole("tab", { name: "Overview" }), { key: "ArrowRight" });
    expect(screen.getByText("Use bounded reads.")).toBeInTheDocument();
    expect(screen.getByLabelText("Resolved")).toBeInTheDocument();
    expect(screen.getByLabelText("#2 How should it ship? facts")).toHaveTextContent(
      "Status resolved",
    );
    expect(screen.getByText("Blocked by #1")).toBeInTheDocument();
    expect(screen.getByText("Blocker #1 is unavailable")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "Source" }));
    expect(container.querySelector("img")).toBeNull();
  });

  it("keeps record limitations compact and offers direct recovery actions", () => {
    const onCopyPath = vi.fn();
    const onRescan = vi.fn();
    const limited: WorkRecordDetail = {
      ...detail,
      fingerprint: undefined,
      warnings: [
        {
          code: "incomplete_source_snapshot",
          message: "OpenSpec fingerprint is unavailable because the captured record is incomplete",
        },
        {
          code: "openspec_progress_unavailable",
          message: "Supported deterministic OpenSpec task progress is unavailable",
          sourcePath: "tasks.md",
        },
      ],
    };
    render(<WorkRecordReadingDesk detail={limited} onCopyPath={onCopyPath} onRescan={onRescan} />);

    const limitationMessage = screen.getByText("Task progress unavailable");
    expect(screen.getByText("2 record limitations")).toBeVisible();
    expect(screen.getByText("Captured source remains readable")).toBeVisible();
    expect(limitationMessage).not.toBeVisible();

    fireEvent.click(screen.getByText("2 record limitations"));
    expect(limitationMessage).toBeVisible();
    expect(screen.getByText(/completion counts may be missing/i)).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Open source" }));
    expect(screen.getByRole("tab", { name: "Source" })).toHaveAttribute("aria-selected", "true");
    fireEvent.click(screen.getByRole("button", { name: "Copy path" }));
    fireEvent.click(screen.getByRole("button", { name: "Rescan" }));
    expect(onCopyPath).toHaveBeenCalledOnce();
    expect(onRescan).toHaveBeenCalledOnce();
  });

  it("keeps distinct warning messages that share code and source", () => {
    const repeated: WorkRecordDetail = {
      ...detail,
      warnings: [
        {
          code: "wayfinder_map_section_missing",
          message: "Destination section is missing",
          sourcePath: ".scratch/search/map.md",
        },
        {
          code: "wayfinder_map_section_missing",
          message: "Notes section is missing",
          sourcePath: ".scratch/search/map.md",
        },
      ],
    };
    render(<WorkRecordReadingDesk detail={repeated} />);

    expect(screen.getByText("2 record limitations")).toBeVisible();
    fireEvent.click(screen.getByText("2 record limitations"));
    expect(screen.getByText("Destination section is missing")).toBeVisible();
    expect(screen.getByText("Notes section is missing")).toBeVisible();
  });

  it("does not claim source is readable when no source block was captured", () => {
    const unavailable: WorkRecordDetail = {
      ...detail,
      warnings: [
        {
          code: "source_unavailable",
          message: "Source could not be captured safely",
          sourcePath: "tasks.md",
        },
      ],
      capabilities: detail.capabilities.map((view) =>
        view.capability.id === "source" ? { ...view, blocks: [] } : view,
      ),
    };
    render(<WorkRecordReadingDesk detail={unavailable} />);

    expect(screen.getByText("Review impact and next steps")).toBeVisible();
    expect(screen.queryByText("Captured source remains readable")).not.toBeInTheDocument();
  });

  it("uses a labeled region instead of an orphaned tabpanel for one capability", () => {
    const sourceOnly: WorkRecordDetail = {
      ...detail,
      record: {
        ...detail.record,
        capabilities: [{ id: "source", label: "Source" }],
      },
      capabilities: [detail.capabilities[2]!],
    };
    render(<WorkRecordReadingDesk detail={sourceOnly} />);

    expect(screen.getByRole("region", { name: "Source" })).toBeInTheDocument();
    expect(screen.queryByRole("tabpanel")).not.toBeInTheDocument();
  });

  it("exposes independent accessible annotation controls and supersession targets", () => {
    const onUpdate = vi.fn();
    render(
      <WorkRecordReadingDesk
        detail={detail}
        annotationTargets={[
          {
            subjectId: "subject_2",
            label: "Replacement plan",
            exactLocatorKey: "openspec-v1:replacement",
            available: true,
          },
          {
            subjectId: "subject_3",
            label: "Old target",
            exactLocatorKey: "openspec-v1:old",
            available: false,
          },
        ]}
        onUpdateAnnotation={onUpdate}
      />,
    );

    fireEvent.change(screen.getByLabelText("Decision"), { target: { value: "approved" } });
    fireEvent.click(screen.getByLabelText("Favorite"));
    fireEvent.change(screen.getByLabelText("Priority"), { target: { value: "high" } });
    fireEvent.change(screen.getByLabelText("Disposition"), {
      target: { value: "superseded" },
    });

    expect(onUpdate).toHaveBeenCalledWith({ command: "set_decision", value: "approved" });
    expect(onUpdate).toHaveBeenCalledWith({ command: "set_favorite", value: true });
    expect(onUpdate).toHaveBeenCalledWith({ command: "set_priority", value: "high" });
    expect(onUpdate).toHaveBeenCalledWith({
      command: "set_disposition",
      value: { status: "superseded", replacement: "subject_2" },
    });
    expect(screen.getByRole("option", { name: "Superseded" })).toBeEnabled();
  });

  it("offers navigation only for an available supersession target", () => {
    const onOpen = vi.fn();
    const superseded = {
      ...detail,
      record: {
        ...detail.record,
        annotation: {
          decision: "undecided" as const,
          disposition: { status: "superseded" as const, replacement: "subject_2" },
          favorite: false,
          todo: false,
          priority: null,
        },
      },
    };
    render(
      <WorkRecordReadingDesk
        detail={superseded}
        annotationTargets={[
          {
            subjectId: "subject_2",
            label: "Replacement plan",
            exactLocatorKey: "openspec-v1:project:replacement",
            available: true,
          },
        ]}
        onUpdateAnnotation={() => undefined}
        onOpenAnnotationTarget={onOpen}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Open replacement" }));
    expect(onOpen).toHaveBeenCalledWith("subject_2");
  });

  it("keeps unavailable supersession targets visible with last-known identity", () => {
    const superseded = {
      ...detail,
      record: {
        ...detail.record,
        annotation: {
          decision: "undecided" as const,
          disposition: { status: "superseded" as const, replacement: "subject_3" },
          favorite: false,
          todo: false,
          priority: null,
        },
      },
    };
    render(
      <WorkRecordReadingDesk
        detail={superseded}
        annotationTargets={[
          {
            subjectId: "subject_3",
            label: "Old target",
            exactLocatorKey: "openspec-v1:project:old",
            available: false,
          },
        ]}
        onUpdateAnnotation={() => undefined}
      />,
    );

    expect(
      screen.getByRole("option", {
        name: "Old target · openspec-v1:project:old · unavailable",
      }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Open replacement" })).not.toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Superseded" })).toBeDisabled();
  });
});
