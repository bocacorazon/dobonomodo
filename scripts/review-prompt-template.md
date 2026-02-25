Review the changes on this branch against `develop`. This branch implements spec {{SPEC_ID}}.

Focus on:
1. Does the implementation match the spec at specs/{{BRANCH_NAME}}/spec.md?
2. Are there bugs, logic errors, or security issues?
3. Do trait implementations satisfy their contracts (especially the IO boundary traits: DataLoader, OutputWriter, MetadataStore, TraceWriter)?
4. Are error handling patterns consistent with the codebase (anyhow for applications, thiserror for libraries)?
5. Is the TDD discipline maintained — do tests exist for all new functionality?
6. Are Polars lazy API patterns used correctly (no premature .collect())?

Do NOT comment on:
- Code style or formatting (enforced by rustfmt)
- Naming conventions (enforced by clippy)
- Import ordering

For each finding, classify as:
- **CRITICAL**: Must fix before merge — bugs, security vulnerabilities, spec violations, broken contracts
- **IMPORTANT**: Should fix — logic issues, missing edge cases, incomplete error handling
- **MINOR**: Nice to have — better abstractions, documentation improvements

Only set `HUMAN_DECISION_REQUIRED: yes` when there are multiple valid fix approaches and a human must choose among alternatives. If there is a clear technical fix, set it to `no` and provide the fix directly.

Output format:
```
## Review Summary
- CRITICAL: <count>
- IMPORTANT: <count>
- MINOR: <count>
HUMAN_DECISION_REQUIRED: <yes|no>

## Findings

### [CRITICAL|IMPORTANT|MINOR] <short title>
**File:** <path>
**Line:** <line or range>
**Issue:** <description>
**Suggested fix:** <suggestion>
```
