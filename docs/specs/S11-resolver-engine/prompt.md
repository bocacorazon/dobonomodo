# S11: Resolver Engine

## Feature
Implement the Resolver rule evaluation engine: evaluate `when` conditions against resolution context, perform automatic period expansion using Calendar hierarchy, render path/table/catalog templates, and return a list of `ResolvedLocation`s.

## Context
- Read: `docs/entities/resolver.md` (full entity — rules, strategies, resolution algorithm, context variables, period expansion)
- Read: `docs/entities/calendar.md` (Calendar hierarchy, levels, identifier_pattern)
- Read: `docs/entities/period.md` (Period structure, parent/child relationships)
- Read: `docs/capabilities/resolve-dataset.md` (ResolutionResult, DataHandle, diagnostics)
- Read: `docs/architecture/sample-datasets.md` (TS-10 resolver rule evaluation)

## Scope

### In Scope
- `core::resolver` module
- Rule evaluation: iterate rules in order, evaluate `when` Expression against resolution context variables (`period.*`, `table.name`), stop at first match
- No-match: return error with diagnostics listing each rule name + why it didn't match
- Period expansion: when `data_level` is finer than requested Period's level, traverse Calendar hierarchy downward to enumerate child Periods at `data_level`
- Template rendering: substitute `{{YYYY}}`, `{{MM}}`, `{{QQ}}`, `{{identifier}}`, `{{table_name}}` tokens in `path`, `table`, `schema`, `params` templates
- `data_level: "any"` — no expansion, single location returned
- Strategy dispatch: produce `ResolvedLocation` with correct fields per strategy type
- Resolver selection: implement precedence logic (Project override → Dataset `resolver_id` → system default)
- Unit tests for rule matching, period expansion, template rendering
- Test scenario TS-10 (rule evaluation with pre-2025 CSV vs post-2025 Parquet)

### Out of Scope
- Actually loading data from resolved locations (that's `DataLoader` in S16)
- `catalog` strategy HTTP calls (return `ResolvedLocation` with endpoint info; actual HTTP is an IO adapter)

## Dependencies
- **S01** (DSL Parser): compile `when` condition expressions

## Parallel Opportunities
Can start as soon as **S01** is done — runs in parallel with all Phase 1 operations (S04–S09).

## Key Design Decisions
- First matching rule wins; no fallthrough
- `when` omitted = always match (catch-all; should be last)
- Period expansion uses Calendar hierarchy — not date arithmetic
- Template tokens come from Calendar `identifier_pattern`
- Returns `Vec<ResolvedLocation>`, not loaded data

## Success Criteria
- Rules evaluate in order; first match wins
- No-match returns error with full diagnostic trace
- Period expansion: quarter → 3 months, year → 12 months (or however the Calendar defines it)
- Template rendering substitutes all tokens correctly
- `data_level: "any"` returns exactly one location
- Resolver precedence: project override wins over dataset default
