# Feature Specification: Resolver Rule Evaluation Engine

**Feature Branch**: `[013-resolver-engine]`  
**Created**: 2026-02-22  
**Status**: Draft  
**Input**: User description: "Implement the Resolver rule evaluation engine: evaluate `when` conditions against resolution context, perform automatic period expansion using Calendar hierarchy, render path/table/catalog templates, and return a list of `ResolvedLocation`s."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Resolve by First Matching Rule (Priority: P1)

As a data pipeline author, I want resolver rules to be evaluated in order so each request resolves to the expected storage location based on period and table context.

**Why this priority**: Correct location selection is the core value of this feature; without it, downstream data operations cannot reliably start.

**Independent Test**: Submit resolution requests with contexts that match different rules and verify that only the first matching rule is selected for each request.

**Acceptance Scenarios**:

1. **Given** an ordered resolver with multiple matching rules, **When** a resolution request is evaluated, **Then** the first matching rule is selected and later rules are ignored.
2. **Given** a resolver where one rule has no `when` condition, **When** earlier rules do not match, **Then** the rule without `when` is treated as a catch-all match.

---

### User Story 2 - Expand Periods to Data Granularity (Priority: P2)

As a data pipeline author, I want requested periods to expand to a finer data level using the defined calendar hierarchy so one request can resolve all required child periods.

**Why this priority**: Expansion ensures complete data coverage when requested and stored period levels differ.

**Independent Test**: Resolve requests where the requested period is coarser than the configured data level and verify child periods and returned locations match the hierarchy definition.

**Acceptance Scenarios**:

1. **Given** a request for a quarter and a rule with monthly data level, **When** resolution runs, **Then** the system returns one resolved location for each month in that quarter.
2. **Given** a request with data level set to `any`, **When** resolution runs, **Then** the system returns exactly one resolved location with no period expansion.

---

### User Story 3 - Explain No-Match Outcomes and Precedence (Priority: P3)

As an operator, I want clear diagnostics when no rules match and predictable resolver precedence so I can quickly correct configuration issues.

**Why this priority**: Troubleshooting and override behavior are essential for safe operations and fast recovery when configurations change.

**Independent Test**: Trigger no-match requests and precedence conflicts, then verify diagnostics and selected resolver source are explicit and correct.

**Acceptance Scenarios**:

1. **Given** no rules match a request, **When** resolution fails, **Then** the failure includes each evaluated rule and the reason it did not match.
2. **Given** project override, dataset resolver reference, and system default are all available, **When** resolution starts, **Then** the project override resolver is selected.

### Edge Cases

- Requested period is already at the same level as `data_level`; the system returns one location for that period without extra expansion.
- Calendar hierarchy has no path from requested level to configured `data_level`; resolution fails with a diagnostic that identifies the missing hierarchy link.
- A template references an unknown token; resolution fails and reports the unresolved token and affected rule.
- Multiple rules match due to overlapping conditions; only the first match is applied and this behavior is reflected in diagnostics.
- No resolver is available from project override, dataset selection, or system default; resolution fails with a clear resolver-selection error.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST evaluate resolver rules in defined order and select the first rule whose condition matches the resolution context.
- **FR-002**: The system MUST treat a rule without a `when` condition as always matching.
- **FR-003**: The system MUST evaluate rule conditions using available context values for requested period and target table.
- **FR-004**: The system MUST fail resolution when no rules match and include diagnostics for every evaluated rule.
- **FR-005**: The system MUST select the resolver source using precedence: project override, then dataset resolver reference, then system default.
- **FR-006**: The system MUST expand a requested period to child periods when configured `data_level` is finer than the requested level.
- **FR-007**: The system MUST use calendar hierarchy relationships (not inferred arithmetic) to perform period expansion.
- **FR-008**: The system MUST skip period expansion and return exactly one location when `data_level` is `any`.
- **FR-009**: The system MUST render templates for location fields using supported context tokens for period, identifier, and table name values.
- **FR-010**: The system MUST return one resolved location per expanded period for the selected rule.
- **FR-011**: The system MUST produce deterministic ordering of returned locations based on the calendar hierarchy order.
- **FR-012**: The system MUST include resolver and rule identity in each resolved location result for traceability.

### Key Entities *(include if feature involves data)*

- **Resolution Request**: Input describing the dataset/table context and requested period to resolve.
- **Resolver**: Ordered collection of resolution rules plus strategy details used to derive physical locations.
- **Resolver Rule**: A conditional mapping from context to location template and target data level.
- **Resolved Location**: Output location descriptor containing rendered location fields and traceability metadata.
- **Resolution Diagnostic**: Structured explanation of selection, non-matches, and failure reasons for a request.
- **Calendar Period Node**: Hierarchical period element used to traverse from requested level to finer data levels.

## Assumptions

- Rule conditions are already syntactically valid before evaluation begins.
- Calendar hierarchy definitions are available and include stable child ordering.
- Template token vocabulary is predefined and shared across all resolver strategies.
- Resolver outputs describe locations only; actual data loading is handled by a separate capability.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In validation scenarios, 100% of requests that should match a rule are resolved using the first eligible rule in order.
- **SC-002**: In no-match scenarios, 100% of failures include a diagnostic entry for each evaluated rule and its non-match reason.
- **SC-003**: For calendars where quarter contains 3 months and year contains 12 months, 100% of quarter-to-month and year-to-month resolutions return exactly 3 and 12 locations respectively.
- **SC-004**: In scenarios using `data_level: any`, 100% of requests return exactly one resolved location.
- **SC-005**: In acceptance testing for the pre-/post-cutover routing scenario, 100% of pre-cutover requests route to the legacy target and 100% of post-cutover requests route to the new target.
- **SC-006**: At least 95% of valid resolution requests in the representative test suite complete with a result in under 1 second.
