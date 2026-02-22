# S21: API Server

## Feature
Build the `api-server` binary: REST endpoints for entity CRUD, Run dispatch via K8s Jobs, Run status monitoring, activation endpoint, and sandbox redirect for draft Projects.

## Context
- Read: `docs/architecture/system-architecture.md` (API server responsibilities, technology choices: axum, kube-rs)
- Read: `docs/capabilities/activate-project.md` (activation flow, sandbox behaviour)
- Read: `docs/entities/run.md` (Run creation, status transitions)

## Scope

### In Scope
- `api-server/src/main.rs` with `axum` router
- Entity CRUD endpoints:
  - `POST/GET/PUT/DELETE /datasets/{id}`
  - `POST/GET/PUT/DELETE /projects/{id}`
  - `POST/GET/PUT /resolvers/{id}`
  - `GET /calendars/{id}`, `GET /periods/{id}`
  - `GET /runs/{id}`, `GET /runs?project_id=&period_id=`
- `POST /projects/{id}/activate` — invoke validation (S13), transition status, pin Dataset version
- `POST /projects/{id}/runs` — create Run, dispatch K8s Job
- `GET /runs/{id}/status` — poll Run status
- K8s Job creation: use `kube-rs` to create a Job that runs the `engine-worker` binary with the Run ID
- Sandbox redirect: when Project is `draft`, replace output destinations in `RunSpec` with deployment-level sandbox DataSource
- Health check endpoint: `GET /health`
- OpenAPI schema generation (via `utoipa` or similar)

### Out of Scope
- Auth/multi-tenancy (placeholder middleware)
- Scheduled Run triggers (future spec)
- WebSocket/SSE for real-time Run status (future)

## Dependencies
- **S17** (Metadata Store), **S19** (Engine Worker)

## Parallel Opportunities
This is the final spec — no downstream dependencies.

## Success Criteria
- All CRUD endpoints work with correct status codes
- Activation endpoint validates and transitions or returns errors
- Run dispatch creates a K8s Job and returns Run ID
- Sandbox redirect replaces output destinations for draft Projects
- Health check returns 200
