# Backstage Domain Language

## Approved Root

A local directory the user has authorized Backstage to scan. Removing approval removes Backstage's knowledge of that root; it never removes or changes the directory.

## Project

A Git working tree discovered beneath an Approved Root.

## Source Document

One safely indexed Markdown file within a Project. It remains readable even when no specialized Planning Format recognizes it or structured parsing fails.

## Planning Format

A deterministic convention that interprets one or more Source Documents as a Work Record. Plain Markdown is the fallback format; OpenSpec and Wayfinder are specialized formats.
_Avoid_: Plugin, artifact type

## Planning Pattern

A validated rule that identifies a project-relative Markdown path as possible planning work. A default Planning Pattern is seeded by Backstage but has no stronger status than a user-created pattern: either may be edited or removed.

## Work Record

One ledger entry derived from one or more Source Documents. A plain-Markdown Work Record contains one document; a specialized Planning Format may group several documents, and a record's source modification time remains an observed filesystem fact.

## OpenSpec Change

The proposal, design, tasks, and capability deltas belonging to one current or archived OpenSpec change. Current and archived changes remain the same kind of Work Record and use the same reader.

## OpenSpec Custody

The repository location of an OpenSpec Change: **Current** beneath `openspec/changes/<change>/`, or **Archived** beneath OpenSpec's archive. Custody does not imply task completion.

## Wayfinder Effort

A map and its decision tickets pursuing one named destination. A local Wayfinder Effort appears as one grouped Work Record.
_Avoid_: Wayfinder session

## Wayfinder Frontier

The open, unclaimed decision tickets in a Wayfinder Effort whose declared blockers are resolved. Frontier membership is derived from source state and does not claim a ticket.

## Task Progress

Deterministically parsed counts of done and open task markers. A current OpenSpec Change with no open tasks is **Done** but remains Current until archived. An Archived change may still contain open tasks or have unavailable progress.

## Work Status

The primary OpenSpec ledger label derived from custody and progress: **Active** for a current change with open or unavailable progress, **Done** for a current change with zero open tasks, and **Archived** for an archived change. Open and done counts remain visible separately, and private user annotations never change this source-derived label.

## Private Annotation

A local, user-owned classification attached to a Work Record. It is not repository truth, does not modify Source Documents, and is not shared through the Project.
_Avoid_: Frontmatter, source metadata

## Decision

The user's selection judgment for a Work Record: **Undecided**, **Approved**, or **Rejected**. Decision is independent of source-derived progress and disposition.
_Avoid_: Work Status

## Disposition

The user's judgment of whether a Work Record remains authoritative: **Applicable**, **Obsolete**, or **Superseded**. A Superseded record names another Work Record as its replacement.
_Avoid_: OpenSpec Custody, archive state

## Attention Markers

Independent private signals that a Work Record is a **Favorite**, is **Todo**, or has **Low**, **Medium**, or **High** priority. They do not imply approval, implementation progress, or source lifecycle.
