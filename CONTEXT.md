# Backstage Domain Language

## Approved Root

A local directory the user has authorized Backstage to scan. Removing approval removes Backstage's knowledge of that root; it never removes or changes the directory.

## Project

A Git working tree discovered beneath an Approved Root.

## Planning Pattern

A validated rule that identifies a project-relative Markdown path as possible planning work. A default Planning Pattern is seeded by Backstage but has no stronger status than a user-created pattern: either may be edited or removed.

## Work Record

One ledger entry representing either a recognized planning bundle or one ordinary Markdown document. A record's source modification time is an observed filesystem fact.

## OpenSpec Change

The proposal, design, tasks, and capability deltas belonging to one current or archived OpenSpec change. Current and archived changes remain the same kind of Work Record and use the same reader.

## OpenSpec Custody

The repository location of an OpenSpec Change: **Current** beneath `openspec/changes/<change>/`, or **Archived** beneath OpenSpec's archive. Custody does not imply task completion.

## Task Progress

Deterministically parsed counts of done and open task markers. A current OpenSpec Change with no open tasks is **Done** but remains Current until archived. An Archived change may still contain open tasks or have unavailable progress.

## Work Status

The primary ledger label derived from custody and progress: **Active** for a current change with open or unavailable progress, **Done** for a current change with zero open tasks, and **Archived** for an archived change. Open and done counts remain visible separately from this label.
