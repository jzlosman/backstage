## ADDED Requirements

### Requirement: Backstage has a distinctive v0 product mark
The system SHALL use one original Backstage mark selected for clear recognition at titlebar and application-icon sizes and for consistency with the Accession Desk visual direction.

#### Scenario: Product titlebar renders the selected mark
- **WHEN** the Backstage workspace is displayed
- **THEN** the titlebar shows the selected product mark next to the Backstage wordmark
- **AND** the mark does not replace the accessible Backstage title

#### Scenario: Packaged macOS app uses the selected mark
- **WHEN** the macOS application bundle is built
- **THEN** its icon resources derive from the same selected mark used by the product chrome

#### Scenario: Mark remains legible at compact size
- **WHEN** the titlebar is displayed at a supported narrow viewport
- **THEN** the mark retains a clear silhouette without depending on text or fine illustrative detail

### Requirement: Brand refinement preserves the operating interface
The system MUST preserve the Accession Desk layout, color semantics, product name, and responsive titlebar behavior while introducing the mark.

#### Scenario: Existing titlebar hierarchy remains intact
- **WHEN** the selected mark replaces the generic glyph
- **THEN** `BACKSTAGE` remains the primary title
- **AND** existing search, status, and command controls retain their behavior
