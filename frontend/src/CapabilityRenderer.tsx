import { CheckCircleIcon } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { CircleIcon } from "@phosphor-icons/react/dist/csr/Circle";
import { WarningIcon } from "@phosphor-icons/react/dist/csr/Warning";
import { useEffect, useMemo, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";

import { renderMarkdown } from "./markdown";
import type {
  AnnotationCommand,
  AnnotationTarget,
  CapabilityView,
  FactValue,
  StructuredBlock,
  SummaryFact,
  WorkRecordAnnotation,
  WorkRecordDetail,
  WorkRecordWarning,
} from "./api";

export function WorkRecordReadingDesk({
  detail,
  annotationTargets = [],
  onUpdateAnnotation,
  onOpenAnnotationTarget,
  onCopyPath,
  onCopyPrompt,
  onOpenTerminal,
  onRescan,
}: {
  detail: WorkRecordDetail;
  annotationTargets?: AnnotationTarget[];
  onUpdateAnnotation?: (command: AnnotationCommand) => void;
  onOpenAnnotationTarget?: (subjectId: string) => void;
  onCopyPath?: () => void;
  onCopyPrompt?: () => void;
  onOpenTerminal?: () => void;
  onRescan?: () => void | Promise<void>;
}) {
  const [selectedCapabilityId, setSelectedCapabilityId] = useState(
    detail.capabilities[0]?.capability.id ?? "",
  );

  useEffect(() => {
    setSelectedCapabilityId(detail.capabilities[0]?.capability.id ?? "");
  }, [detail.capabilities, detail.subjectId]);

  const selected =
    detail.capabilities.find((view) => view.capability.id === selectedCapabilityId) ??
    detail.capabilities[0];

  const moveTab = (event: ReactKeyboardEvent<HTMLButtonElement>, currentId: string) => {
    const index = detail.capabilities.findIndex((view) => view.capability.id === currentId);
    if (index < 0) return;
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? detail.capabilities.length - 1
          : event.key === "ArrowLeft"
            ? (index - 1 + detail.capabilities.length) % detail.capabilities.length
            : event.key === "ArrowRight"
              ? (index + 1) % detail.capabilities.length
              : null;
    if (nextIndex === null) return;
    event.preventDefault();
    const next = detail.capabilities[nextIndex];
    if (!next) return;
    setSelectedCapabilityId(next.capability.id);
    requestAnimationFrame(() =>
      document.getElementById(`record-${detail.subjectId}-${next.capability.id}-tab`)?.focus(),
    );
  };

  return (
    <article className="artifact-reading work-record-reading">
      <header className="artifact-header">
        <span className="registry-stamp">{formatLabel(detail.record.locator.formatId)}</span>
        <h1>{detail.record.displayName}</h1>
        <p className="artifact-context">
          <span>{detail.projectName}</span>
          <span>{detail.git ? detail.git.branch : "Git unavailable"}</span>
          <span>{detail.record.sources.length} source files</span>
        </p>
      </header>

      <aside className="provenance-spine" aria-label="Work Record provenance">
        <dl>
          <div>
            <dt>Format</dt>
            <dd>{detail.record.recognition.adapterId}</dd>
          </div>
          <div>
            <dt>Record</dt>
            <dd>
              <code>{detail.record.locator.adapterRecordKey}</code>
            </dd>
          </div>
          <div>
            <dt>Recognition</dt>
            <dd>{formatLabel(detail.record.recognition.level)}</dd>
          </div>
          <div>
            <dt>Fingerprint</dt>
            <dd>
              <code>{detail.fingerprint ?? "Unavailable"}</code>
            </dd>
          </div>
        </dl>
      </aside>

      <RecordLimitations
        warnings={[...detail.record.warnings, ...detail.warnings]}
        sourceReadable={detail.capabilities.some(
          (view) =>
            view.capability.id === "source" &&
            view.blocks.some(
              (block) =>
                block.kind === "markdown_section" ||
                (block.kind === "source_list" && block.sources.length > 0),
            ),
        )}
        onOpenSource={
          detail.capabilities.some((view) => view.capability.id === "source")
            ? () => setSelectedCapabilityId("source")
            : undefined
        }
        onRescan={onRescan}
      />

      <AnnotationControls
        annotation={detail.record.annotation ?? defaultAnnotation()}
        targets={annotationTargets}
        disabled={!onUpdateAnnotation}
        onUpdate={(command) => onUpdateAnnotation?.(command)}
        onOpenTarget={onOpenAnnotationTarget}
      />

      {(onCopyPath || onCopyPrompt || onOpenTerminal) && (
        <div className="artifact-actions" aria-label="Read-only handoffs">
          {onCopyPath && (
            <button className="button" type="button" onClick={onCopyPath}>
              Copy path
            </button>
          )}
          {onCopyPrompt && (
            <button className="button" type="button" onClick={onCopyPrompt}>
              Copy continuation prompt
            </button>
          )}
          {onOpenTerminal && (
            <button className="button" type="button" onClick={onOpenTerminal}>
              Open terminal
            </button>
          )}
        </div>
      )}

      {detail.capabilities.length > 1 && (
        <nav className="openspec-view-tabs" aria-label="Work Record views" role="tablist">
          {detail.capabilities.map((view) => {
            const selected = view.capability.id === selectedCapabilityId;
            return (
              <button
                id={`record-${detail.subjectId}-${view.capability.id}-tab`}
                key={view.capability.id}
                type="button"
                role="tab"
                aria-selected={selected}
                aria-controls={`record-${detail.subjectId}-${view.capability.id}-panel`}
                tabIndex={selected ? 0 : -1}
                onClick={() => setSelectedCapabilityId(view.capability.id)}
                onKeyDown={(event) => moveTab(event, view.capability.id)}
              >
                {view.capability.label}
              </button>
            );
          })}
        </nav>
      )}

      {selected ? (
        <section
          id={`record-${detail.subjectId}-${selected.capability.id}-panel`}
          className="openspec-panel capability-panel"
          role={detail.capabilities.length > 1 ? "tabpanel" : "region"}
          aria-labelledby={
            detail.capabilities.length > 1
              ? `record-${detail.subjectId}-${selected.capability.id}-tab`
              : undefined
          }
          aria-label={detail.capabilities.length === 1 ? selected.capability.label : undefined}
        >
          <CapabilityRenderer view={selected} />
        </section>
      ) : (
        <div className="openspec-empty">
          <h2>No readable view is available</h2>
          <p>The record remains indexed, but its adapter returned no capability payload.</p>
        </div>
      )}
    </article>
  );
}

function RecordLimitations({
  warnings,
  sourceReadable,
  onOpenSource,
  onRescan,
}: {
  warnings: WorkRecordWarning[];
  sourceReadable: boolean;
  onOpenSource?: () => void;
  onRescan?: () => void | Promise<void>;
}) {
  const [rescanning, setRescanning] = useState(false);
  const unique = [
    ...new Map(
      warnings.map((warning) => [
        `${warning.code}:${warning.sourcePath ?? ""}:${warning.line ?? ""}:${warning.message}`,
        warning,
      ]),
    ).values(),
  ];
  if (unique.length === 0) return null;
  const countLabel = `${unique.length} record ${unique.length === 1 ? "limitation" : "limitations"}`;
  const rescan = async () => {
    if (!onRescan || rescanning) return;
    setRescanning(true);
    try {
      await onRescan();
    } finally {
      setRescanning(false);
    }
  };
  return (
    <details className="record-limitations">
      <summary>
        <WarningIcon aria-hidden="true" weight="regular" />
        <span>
          <strong>{countLabel}</strong>
          <small>
            {sourceReadable ? "Captured source remains readable" : "Review impact and next steps"}
          </small>
        </span>
        <span className="record-limitations-disclosure">Review details</span>
      </summary>
      <div className="record-limitations-body">
        <ul>
          {unique.map((warning) => {
            const presentation = limitationPresentation(warning);
            return (
              <li
                key={`${warning.code}:${warning.sourcePath ?? ""}:${warning.line ?? ""}:${warning.message}`}
              >
                <h3>{presentation.title}</h3>
                <p>{presentation.impact}</p>
                {warning.sourcePath && (
                  <code>
                    {warning.sourcePath}
                    {warning.line ? `:${warning.line}` : ""}
                  </code>
                )}
              </li>
            );
          })}
        </ul>
        {(onOpenSource || onRescan) && (
          <div className="record-limitations-actions" aria-label="Record limitation actions">
            {onOpenSource && (
              <button className="button button--compact" type="button" onClick={onOpenSource}>
                Open source
              </button>
            )}
            {onRescan && (
              <button
                className="button button--compact"
                type="button"
                disabled={rescanning}
                onClick={() => void rescan()}
              >
                {rescanning ? "Rescanning…" : "Rescan"}
              </button>
            )}
          </div>
        )}
      </div>
    </details>
  );
}

function limitationPresentation(warning: WorkRecordWarning) {
  if (warning.code === "incomplete_source_snapshot") {
    return {
      title: "Freshness check unavailable",
      impact:
        "Backstage cannot verify the complete source set. Open Source to inspect anything captured safely, then rescan.",
    };
  }
  if (warning.code === "openspec_progress_unavailable") {
    return {
      title: "Task progress unavailable",
      impact:
        "Completion counts may be missing. Open Source to inspect any captured tasks file, then rescan.",
    };
  }
  return {
    title: warning.message,
    impact:
      "Some structured facts may be unavailable. Review the source details before continuing.",
  };
}

function AnnotationControls({
  annotation,
  targets,
  disabled,
  onUpdate,
  onOpenTarget,
}: {
  annotation: WorkRecordAnnotation;
  targets: AnnotationTarget[];
  disabled: boolean;
  onUpdate: (command: AnnotationCommand) => void;
  onOpenTarget?: (subjectId: string) => void;
}) {
  const replacement =
    annotation.disposition.status === "superseded" ? annotation.disposition.replacement : "";
  const availableTargets = targets.filter((target) => target.available);
  return (
    <section className="annotation-controls" aria-label="Private Work Record annotations">
      <header>
        <h2>Private annotations</h2>
        <span>Stored by Backstage · repository unchanged</span>
      </header>
      <div className="annotation-fields">
        <label>
          Decision
          <select
            value={annotation.decision}
            disabled={disabled}
            onChange={(event) =>
              onUpdate({
                command: "set_decision",
                value: event.target.value as WorkRecordAnnotation["decision"],
              })
            }
          >
            <option value="undecided">Undecided</option>
            <option value="approved">Approved</option>
            <option value="rejected">Rejected</option>
          </select>
        </label>
        <label>
          Disposition
          <select
            value={annotation.disposition.status}
            disabled={disabled}
            onChange={(event) => {
              const status = event.target.value;
              if (status === "superseded") {
                const target = availableTargets.find(
                  (candidate) => candidate.subjectId !== replacement,
                );
                if (target) {
                  onUpdate({
                    command: "set_disposition",
                    value: { status: "superseded", replacement: target.subjectId },
                  });
                }
              } else {
                onUpdate({
                  command: "set_disposition",
                  value: status === "obsolete" ? { status: "obsolete" } : { status: "applicable" },
                });
              }
            }}
          >
            <option value="applicable">Applicable</option>
            <option value="obsolete">Obsolete</option>
            <option value="superseded" disabled={availableTargets.length === 0}>
              Superseded
            </option>
          </select>
        </label>
        {annotation.disposition.status === "superseded" && (
          <label>
            Replacement
            <select
              value={replacement}
              disabled={disabled}
              onChange={(event) =>
                onUpdate({
                  command: "set_disposition",
                  value: { status: "superseded", replacement: event.target.value },
                })
              }
            >
              {targets.map((target) => (
                <option
                  key={target.subjectId}
                  value={target.subjectId}
                  disabled={!target.available && target.subjectId !== replacement}
                >
                  {target.available
                    ? target.label
                    : `${target.label} · ${target.exactLocatorKey} · unavailable`}
                </option>
              ))}
            </select>
          </label>
        )}
        {annotation.disposition.status === "superseded" &&
          targets.find((target) => target.subjectId === replacement)?.available &&
          onOpenTarget && (
            <button
              type="button"
              className="button button--compact annotation-target-link"
              onClick={() => onOpenTarget(replacement)}
            >
              Open replacement
            </button>
          )}
        <label>
          Priority
          <select
            value={annotation.priority ?? ""}
            disabled={disabled}
            onChange={(event) =>
              onUpdate({
                command: "set_priority",
                value: (event.target.value || null) as WorkRecordAnnotation["priority"],
              })
            }
          >
            <option value="">No priority</option>
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
          </select>
        </label>
        <label className="annotation-check">
          <input
            type="checkbox"
            checked={annotation.favorite}
            disabled={disabled}
            onChange={(event) => onUpdate({ command: "set_favorite", value: event.target.checked })}
          />
          Favorite
        </label>
        <label className="annotation-check">
          <input
            type="checkbox"
            checked={annotation.todo}
            disabled={disabled}
            onChange={(event) => onUpdate({ command: "set_todo", value: event.target.checked })}
          />
          Todo
        </label>
      </div>
    </section>
  );
}

export function CapabilityRenderer({ view }: { view: CapabilityView }) {
  return (
    <div className={`capability-blocks capability-blocks--${view.capability.id}`}>
      {view.blocks.map((block) => (
        <CapabilityBlock key={block.id} block={block} />
      ))}
    </div>
  );
}

function CapabilityBlock({ block }: { block: StructuredBlock }) {
  if (block.kind === "markdown_section") {
    return <MarkdownSection block={block} />;
  }
  if (block.kind === "fact_register") {
    return (
      <section className="capability-facts">
        <h2>{block.title}</h2>
        <dl>
          {block.facts.map((fact) => (
            <div key={fact.key}>
              <dt>{fact.label}</dt>
              <dd>{factValue(fact.value)}</dd>
            </div>
          ))}
        </dl>
      </section>
    );
  }
  if (block.kind === "progress") {
    return (
      <section className="capability-progress" aria-label={`${block.label} progress`}>
        <div>
          <h2>{block.label}</h2>
          <strong>
            {block.completed} of {block.total} complete
          </strong>
        </div>
        <progress value={block.completed} max={Math.max(1, block.total)} />
      </section>
    );
  }
  if (block.kind === "item_collection") {
    return (
      <section className="task-group capability-items">
        <header>
          <h2>{block.title}</h2>
          <span>{block.items.length} items</span>
        </header>
        <ul>
          {block.items.map((item) => {
            const completed = itemCompletion(item.facts);
            const stateClass =
              completed === true
                ? "is-complete"
                : completed === false
                  ? "is-remaining"
                  : "is-neutral";
            return (
              <li className={stateClass} key={item.id}>
                {completed === true ? (
                  <CheckCircleIcon aria-label="Resolved" weight="fill" />
                ) : completed === false ? (
                  <CircleIcon aria-label="Open" weight="regular" />
                ) : null}
                <span>
                  <strong>{item.title}</strong>
                  {item.markdown && (
                    <span
                      className="markdown-document capability-item-markdown"
                      dangerouslySetInnerHTML={{ __html: renderMarkdown(item.markdown) }}
                    />
                  )}
                  {item.facts.length > 0 && (
                    <span className="capability-item-facts" aria-label={`${item.title} facts`}>
                      {item.facts.map((fact) => (
                        <span key={fact.key}>
                          <b>{fact.label}</b> {factValue(fact.value)}
                        </span>
                      ))}
                    </span>
                  )}
                  {item.relationships.length > 0 && (
                    <span
                      className="capability-item-relationships"
                      aria-label={`${item.title} relationships`}
                    >
                      {item.relationships.map((relationship) => (
                        <span key={`${relationship.kind}:${relationship.targetSubjectId}`}>
                          {relationship.label}
                        </span>
                      ))}
                    </span>
                  )}
                </span>
                {item.source.line && <small>line {item.source.line}</small>}
              </li>
            );
          })}
        </ul>
      </section>
    );
  }
  if (block.kind === "relationship_list") {
    return (
      <section className="capability-relationships">
        <h2>{block.title}</h2>
        {block.relationships.length > 0 ? (
          <ul>
            {block.relationships.map((relationship) => (
              <li key={`${relationship.kind}:${relationship.targetSubjectId}`}>
                <strong>{relationship.label}</strong>
                <small>{formatLabel(relationship.kind)}</small>
              </li>
            ))}
          </ul>
        ) : (
          <p>No relationships are available.</p>
        )}
      </section>
    );
  }
  if (block.kind === "empty_state") {
    return <p className="openspec-empty">{block.message}</p>;
  }
  if (block.kind === "warning") {
    return (
      <aside className="warning-sheet capability-warning" aria-label="Record warning">
        <strong>{block.warning.message}</strong>
        {block.warning.sourcePath && (
          <code>
            {block.warning.sourcePath}
            {block.warning.line ? `:${block.warning.line}` : ""}
          </code>
        )}
      </aside>
    );
  }
  return (
    <section className="capability-sources">
      <h2>{block.title}</h2>
      <ul>
        {block.sources.map((source) => (
          <li key={`${source.relativePath}:${source.line ?? ""}`}>
            <code>{source.relativePath}</code>
            {source.line && <small>line {source.line}</small>}
          </li>
        ))}
      </ul>
    </section>
  );
}

function MarkdownSection({
  block,
}: {
  block: Extract<StructuredBlock, { kind: "markdown_section" }>;
}) {
  const rendered = useMemo(() => renderMarkdown(block.markdown), [block.markdown]);
  return (
    <section className="overview-excerpt capability-markdown">
      <div className="overview-excerpt-heading">
        <h2>{block.title}</h2>
        <span>{block.source.relativePath}</span>
      </div>
      <div
        className="markdown-document overview-markdown"
        dangerouslySetInnerHTML={{ __html: rendered }}
      />
    </section>
  );
}

function itemCompletion(facts: SummaryFact[]): boolean | undefined {
  const completed = facts.find(
    (candidate) => candidate.key === "completed" || candidate.key.endsWith(".completed"),
  )?.value;
  if (completed?.type === "boolean") return completed.value;
  const status = facts.find((candidate) => candidate.key === "wayfinder.ticket.status")?.value;
  if (status?.type !== "text") return undefined;
  if (status.value === "resolved") return true;
  if (status.value === "open" || status.value === "claimed") return false;
  return undefined;
}

function factValue(value: FactValue): string {
  if (value.type === "boolean") return value.value ? "Yes" : "No";
  return String(value.value);
}

function defaultAnnotation(): WorkRecordAnnotation {
  return {
    decision: "undecided",
    disposition: { status: "applicable" },
    favorite: false,
    todo: false,
    priority: null,
  };
}

function formatLabel(value: string): string {
  return value
    .replaceAll("_", " ")
    .replaceAll("-", " ")
    .replace(/\b\w/g, (character) => character.toUpperCase());
}
