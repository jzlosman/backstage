## ADDED Requirements

### Requirement: Newest-first work ordering
The ledger SHALL order all matching planning bundles and standalone Markdown records by observed source modification time descending, with deterministic identity as a stable tie-breaker.

#### Scenario: Records have different source times
- **WHEN** two visible records have valid observed source modification times
- **THEN** the more recently modified record appears first regardless of project or filename

#### Scenario: Records differ only below JavaScript's safe integer precision
- **WHEN** visible records have distinct observed nanosecond timestamps that would round to the same JavaScript number
- **THEN** the later exact timestamp still sorts first
- **AND** the Tauri bridge does not discard timestamp precision

#### Scenario: Records share a source time
- **WHEN** visible records have equal observed source modification times
- **THEN** their deterministic identities provide stable ordering across renders and rescans

#### Scenario: Planning and ordinary Markdown are mixed
- **WHEN** All Markdown contains OpenSpec bundles, possible planning artifacts, and standalone documents
- **THEN** every row participates in the same newest-first ordering rule

#### Scenario: Filters or search change
- **WHEN** the user changes project, scope, status, or search filters
- **THEN** the complete matching set is filtered before newest-first ordering and grouping

### Requirement: Local recency groups
The ledger SHALL partition the sorted records into Today, Past 7 days, Older, and Date unavailable groups using the user's local calendar and an explicit current-time input.

#### Scenario: Source changed today
- **WHEN** a record's source date matches the current local calendar date
- **THEN** it appears under Today

#### Scenario: Source changed during the prior seven dates
- **WHEN** a record's source date falls on one of the seven local calendar dates before Today
- **THEN** it appears under Past 7 days

#### Scenario: Source is older
- **WHEN** a valid source date precedes the Past 7 days boundary
- **THEN** it appears under Older

#### Scenario: Source date is unavailable
- **WHEN** a record has no valid observed source modification time
- **THEN** it appears under Date unavailable after every dated group

#### Scenario: Local midnight passes
- **WHEN** the explicit clock crosses a local calendar-day boundary while the app remains open or refreshes
- **THEN** records are regrouped according to the new local date
- **AND** their underlying observed timestamps remain unchanged

#### Scenario: Daylight-saving boundary occurs
- **WHEN** the prior seven local dates cross a daylight-saving transition
- **THEN** grouping uses calendar dates rather than fixed 24-hour nanosecond buckets

### Requirement: Prominent date and status metadata
Each ledger row SHALL present source date, work status, and available open/done counts as primary readable metadata rather than tiny secondary text.

#### Scenario: Current OpenSpec row is active
- **WHEN** an Active OpenSpec bundle has 7 open and 11 done tasks
- **THEN** its row displays Active and `7 open · 11 done` on the main metadata line
- **AND** a body-sized local source date or time is visible without opening the record

#### Scenario: Current OpenSpec row is done
- **WHEN** a Done OpenSpec bundle has 18 done tasks
- **THEN** its row displays Done and `0 open · 18 done`
- **AND** Done is not communicated by color alone

#### Scenario: Archived row is shown
- **WHEN** an archived OpenSpec bundle is visible
- **THEN** its row displays Archived as the primary status
- **AND** available task counts and source date remain independently readable

#### Scenario: Progress is unavailable
- **WHEN** a recognized OpenSpec bundle has unavailable progress
- **THEN** the row displays its lifecycle status and `Progress unavailable`
- **AND** it does not display invented zero counts

#### Scenario: Standalone Markdown row is shown
- **WHEN** an ordinary Markdown document is visible in All Markdown
- **THEN** its row displays document provenance and a prominent source date
- **AND** it receives no invented OpenSpec lifecycle or task counts

### Requirement: Grouped ledger remains accessible and bounded
Recency grouping SHALL preserve keyboard navigation, named semantics, complete result counts, bounded mounting, and responsive operation.

#### Scenario: Keyboard navigation crosses a group boundary
- **WHEN** the user moves from the last row in one recency group to the next row
- **THEN** focus moves to the first record in the following non-empty group
- **AND** group headings are not inserted as selectable work records

#### Scenario: Initial result exceeds the mount limit
- **WHEN** matching records span multiple groups and exceed the bounded initial batch
- **THEN** counts reflect the complete matching set while only the bounded batch mounts
- **AND** every mounted row appears beneath its correct group heading

#### Scenario: More records are revealed
- **WHEN** the user reveals the next ledger batch
- **THEN** rows continue in global newest-first order
- **AND** group headings are not duplicated incorrectly at the batch boundary

#### Scenario: Date cannot fit visually
- **WHEN** the ledger narrows at an existing responsive breakpoint
- **THEN** the visible date uses a concise local format with a complete accessible label
- **AND** lifecycle and open/done facts remain readable without horizontal scrolling

#### Scenario: Reduced motion is preferred
- **WHEN** records regroup after a refresh and reduced motion is enabled
- **THEN** the interface updates without nonessential movement animation

### Requirement: Recency replaces the redundant recent filter
The ledger SHALL use newest-first grouping as its default chronology and SHALL remove the separate Recently changed filter without removing access to any record.

#### Scenario: User opens Current work
- **WHEN** the Current ledger contains recent and older records
- **THEN** Today and Past 7 days records appear before Older records automatically
- **AND** no separate Recently changed action is required to discover them

#### Scenario: No recent records exist
- **WHEN** every matching record belongs to Older or Date unavailable
- **THEN** the ledger omits empty Today and Past 7 days headings
- **AND** all matching records remain reachable
