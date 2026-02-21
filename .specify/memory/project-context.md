# DobONoMoDo — Project Context

## Project Summary

**DobONoMoDo** is a computation engine that performs calculations over a **Dataset**. Calculations are prescribed in a domain-specific language (DSL) that defines a limited — but extensible — set of operations to be performed on the dataset. A **Dataset** is a consolidated view of all the tables in the system.

## AI Tooling Strategy

This project is developed entirely with AI assistance using a multi-model approach:

| Task Type | Preferred Model(s) |
|---|---|
| Product definition, specs, planning | Anthropic (Claude) |
| Implementation, code generation | Anthropic or OpenAI — task-dependent |
| Spec review, analysis | Either, depending on context |

**Framework**: [GitHub Spec-Kit](https://github.com/github/spec-kit) — spec-driven development where specifications become directly executable inputs for AI agents.

## Development Workflow

All development follows the Spec-Kit pipeline in order:

```
Product Context → /speckit.constitution → /speckit.specify → /speckit.plan → /speckit.tasks → /speckit.implement
```

1. **Constitution** (`.specify/memory/constitution.md`): Governing principles and non-negotiables
2. **Spec** (`docs/<feature>/spec.md`): What to build and why — no implementation details
3. **Plan** (`docs/<feature>/plan.md`): Technical design, architecture, stack choices
4. **Tasks** (`docs/<feature>/tasks.md`): Ordered, actionable task list for implementation
5. **Implement**: AI agent executes tasks sequentially

## Current Development Phase

**Phase**: Product Management / Entity Definition

**Active goal**: Define entities, their interactions, inputs, outputs, and behaviors in sufficient detail to produce a high-quality product spec that feeds the Spec-Kit workflow.

**Key artifacts to produce**:
- Entity model (entities, their attributes, relationships)
- Interaction model (how entities communicate, trigger operations)
- DSL specification (operations, syntax, semantics)
- Data model (Dataset structure, table schema conventions)

## Key Concepts

### Computation Engine
- Executes calculations defined by the DSL
- Inputs: a Dataset + a DSL expression/program
- Output: computed results (structure TBD)

### Dataset
- Consolidated view of all tables in the system
- Source of truth for all computation inputs

### DSL (Domain-Specific Language)
- Defines operations the engine can perform
- Limited but extensible set of operations
- Operates on Dataset entities

## Repository Layout

```
dobonomodo/
├── DobONoMoDo.md          # High-level project description
├── Dataset.md             # Dataset concept definition
├── docs/                  # Feature specs and planning artifacts
├── .specify/
│   ├── memory/            # Persistent AI agent context (this file lives here)
│   │   ├── constitution.md
│   │   └── project-context.md  ← YOU ARE HERE
│   └── templates/         # Spec-Kit templates
└── .github/
    ├── agents/            # GitHub Copilot agent definitions
    └── prompts/           # Spec-Kit prompt files
```

## Agent Instructions

- Always read this file at the start of a session for project context.
- All spec work goes in `docs/<feature>/` — use `spec.md`, `plan.md`, `tasks.md` naming.
- Constitution at `.specify/memory/constitution.md` must be established before any feature specs.
- Outputs at each step must be self-contained enough to serve as inputs for the next agent in the pipeline, with no assumed shared memory.
- Write artifacts in Markdown, structured for AI consumption: clear headings, explicit definitions, no ambiguity.
