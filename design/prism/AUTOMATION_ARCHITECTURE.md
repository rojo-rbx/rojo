# Prism automation architecture

This document is a planning artifact. It records confirmed repository facts and proposes a small automation subsystem that can grow from the current `prism exec <file.lua>` primitive without creating unrelated queues, endpoint families, poll loops, and result formats for every future command.

External Roblox API notes were checked against Creator Hub pages on 2026-07-20:

- `Selection` documents plugin access to `Get`, `Set`, `Add`, `Remove`, and `SelectionChanged`: <https://create.roblox.com/docs/reference/engine/classes/Selection>
- `StudioCaptureService` documents plugin-security screenshot methods: <https://create.roblox.com/docs/reference/engine/classes/StudioCaptureService>
- `StudioScreenshotCapture` documents plugin-security `GetBuffer`, `GetErrors`, and `ScaleAsync`: <https://create.roblox.com/docs/reference/engine/classes/StudioScreenshotCapture>
- `RunService` documents `Run` and `Stop` as plugin-security methods, plus `IsEdit`, `IsRunning`, `IsRunMode`, `IsStudio`, `IsServer`, and `RunState`: <https://create.roblox.com/docs/reference/engine/classes/RunService>
- `StudioTestService` documents plugin-security `ExecutePlayModeAsync`, `ExecuteRunModeAsync`, and multiplayer test methods: <https://create.roblox.com/docs/reference/engine/classes/StudioTestService>
- `ChangeHistoryService` documents `TryBeginRecording` and `FinishRecording`, and says `SetWaypoint` is being replaced by recordings: <https://create.roblox.com/docs/reference/engine/classes/ChangeHistoryService>
- `Instance.UniqueId` is documented as Roblox security, so it is not a plugin-accessible persistent identifier for this design: <https://create.roblox.com/docs/reference/engine/classes/Instance>

## 1. Current Prism architecture

Confirmed facts:

- CLI subcommands are declared in `src/cli/mod.rs`. `Options::run` dispatches `Subcommand::Exec` to `ExecCommand::run`.
- `src/cli/exec.rs` implements `rojo exec <FILE>` with `--address` and `--port`, reads UTF-8 source, strips one leading BOM, enforces `MAX_SOURCE_SIZE_BYTES`, checks `/api/rojo`, submits MessagePack to `/api/exec/jobs`, polls `/api/exec/jobs/{id}`, and renders terminal results and logs.
- `src/exec.rs` owns the current server-side exec queue. `ExecJobStore` uses `Mutex`, `HashMap`, and `VecDeque`, supports one claimed job at once, and tracks `Pending`, `Claimed`, `Succeeded`, `Failed`, and `TimedOut`.
- `src/serve_session.rs` stores one `ExecJobStore` per `ServeSession` as `exec_job_store: ExecJobStore` and exposes `exec_job_store(&self) -> &ExecJobStore`.
- `src/web/mod.rs` builds the raw Hyper server with `make_service_fn` and `service_fn`, sends `/api...` requests to `src/web/api.rs`, and handles non-API requests through the UI module.
- `src/web/api.rs` manually parses `/api/exec/jobs`, `/api/exec/jobs/next`, `/api/exec/jobs/{id}`, and `/api/exec/jobs/{id}/complete`. It rejects non-loopback peers using the same locality check as `/api/open`, cleans expired jobs on exec requests, and returns MessagePack API errors.
- `src/web/interface.rs` defines the wire structs and enums for exec, the sync protocol, WebSocket packets, binary model serialization, and the common `ErrorResponse`.
- `src/web/util.rs` defines MessagePack helpers using `rmp_serde` with human-readable struct maps and `application/msgpack`.
- `plugin/src/ApiContext.lua` owns plugin HTTP methods. Exec currently adds `claimNextExecJob()` and `completeExecJob(jobId, payload)` using the existing Promise and MessagePack wrappers.
- `plugin/src/ServeSession.lua` owns the plugin connection lifecycle. It creates `Exec.new`, starts it only after initial sync and WebSocket creation, and stops it from `__stopInternal` before disconnecting `ApiContext`.
- `plugin/src/Exec.lua` owns the production exec poller and executor. It claims at roughly 250 ms intervals only in edit mode, executes a fresh temporary `ModuleScript`, records one undo action with `ChangeHistoryService:TryBeginRecording` and `FinishRecording`, captures bounded logs through `rojoExec.print`/`warn`, encodes structured scalar/array/table results, retries completion, and treats timeouts as soft.
- `plugin/src/Types.lua` validates API payloads with `t`, including exec claim/status/result/log structs.
- `plugin/http/init.lua`, `plugin/http/Error.lua`, and `plugin/http/Response.lua` provide Promise-based Roblox HTTP wrappers. Non-2xx responses reject as `Http.Error`; this is why `ApiContext.completeExecJob` has a compatibility check for HTTP 409 text.

Proposal:

Prism should treat the current exec flow as the first implementation of a general automation pattern: CLI submits a typed job to the server, the connected plugin claims and runs it on the current Studio DataModel, the plugin posts a typed completion, and the CLI renders the result. The generalization should be narrow and typed, not a generic remote RPC system.

## 2. Design goals

- One session-scoped automation store, one claim loop in the plugin, one result envelope, and one CLI polling pattern for short request/response commands.
- Typed automation handlers for inspect, search, selection, camera, focus, screenshot, play/stop, doctor plugin checks, snapshot, diff, preview, and future playtesting.
- Preserve arbitrary `exec` as a trusted escape hatch and a development tool, but do not build ordinary product commands as string-generated Luau wrappers around exec.
- Support small MessagePack job responses, large/binary artifacts, long-running streams, and previewable DataModel modifications as separate transport modes under one automation namespace.
- Keep WebSocket live sync and `MessageQueue` behavior unchanged unless a future phase deliberately extends them.
- Keep all automation endpoints local-only unless the user later designs authentication.

## 3. Non-goals

- No public network RPC framework.
- No authentication in the immediate MVP beyond loopback/local peer checks.
- No multi-user Studio collaboration semantics in the first version.
- No promise that Preview Mode is a security sandbox.
- No attempt to use undocumented persistent Roblox identifiers.
- No routing of play-client versus play-server automation until playtesting is designed.
- No reuse of the live-sync WebSocket packet schema for unrelated automation events.

## 4. Command classification

Confirmed command list comes from the user request. Classifications below are proposals.

| Command | Classification | Reasoning |
| --- | --- | --- |
| `exec` | Arbitrary exec wrapper and specialized automation job | Already implemented as trusted Luau source. It should remain special because it accepts arbitrary code and has source-size/undo semantics unlike typed commands. |
| `inspect` | Typed one-shot automation job | Needs Studio DataModel access and a structured request/result. Should not generate source. |
| `search` | Typed one-shot automation job | Pure DataModel traversal with filters and bounded results. |
| `selection` | Typed one-shot automation job | Documented `Selection:Get()` is plugin-accessible and returns current selection. |
| `select` | Typed one-shot automation job | Documented `Selection:Set()` is plugin-accessible; needs reference resolution and edit-mode behavior verified. |
| `camera` | Typed one-shot automation job | Reads `Workspace.CurrentCamera` properties. No source execution needed. |
| `focus` | Studio lifecycle/typed one-shot automation job | Mutates camera state to frame an object. Needs bounding box handling and edit-mode verification. |
| `screenshot` | Binary artifact job | Screenshot capture returns a buffer according to current docs. The CLI should download an artifact rather than receive a huge MessagePack completion. Requires Studio spike. |
| `play` | Studio lifecycle command | `RunService:Run` and `StudioTestService` play methods exist with plugin security, but actual installed-plugin behavior must be spiked. |
| `stop` | Studio lifecycle command | `RunService:Stop` and `StudioTestService:LeaveTest` exist with plugin/security indications, but behavior depends on mode. |
| `doctor` | CLI-only plus typed one-shot automation job | CLI can check server reachability/protocol. Plugin status and Studio mode require a plugin-side automation status handler. |
| `snapshot` | Typed one-shot automation job, possibly artifact job for large output | Needs bounded DataModel traversal and deterministic encoding. Large snapshots should use artifacts. |
| `diff` | CLI-only when comparing saved snapshots; typed one-shot when diffing live Studio | Diffing two local snapshot files is CLI-only. Diffing live Studio against a file or current server state needs automation. |
| `watch` | Persistent WebSocket stream | Live events must not use 250 ms HTTP polling. |
| `profile` | Typed one-shot automation job, unsupported parts require verification | Hierarchy/class/script counts are easy. Memory, script performance, and MicroProfiler data may be unavailable/private. |
| `preview` | Typed one-shot automation job with preview/apply protocol | Requires execution plus snapshot/diff/patch capture. Not a secure sandbox. |
| Automated playtesting | Unsupported without further Studio verification, later WebSocket/artifact/lifecycle system | Requires play/stop, input injection, observations, logs, screenshots, assertions, reports, and client/server routing. |

Features that can be wrappers over `exec`: quick user-authored scripts, early prototypes of inspect/search, and manual experiments. Features that deserve dedicated handlers: inspect, search, selection/select, camera/focus, screenshot, play/stop, doctor status, snapshot/diff, watch, profile, preview/apply. Dedicated handlers give typed validation, bounded traversal, stable output, and clearer safety behavior.

## 5. Shared automation job model

Proposal:

`ExecJobStore` should evolve into `AutomationJobStore`, but not by erasing all domain types into arbitrary maps. The generic store should own queue mechanics, deadlines, terminal retention, one active claim, capacity limits, local cleanup, and common state transitions. Job payloads should remain typed enums in Rust and Luau.

Suggested Rust shape:

```rust
pub struct AutomationJob {
    pub id: Uuid,
    pub kind: AutomationJobKind,
    pub request: AutomationRequest,
    pub state: AutomationJobState,
    pub created_at: SystemTime,
    pub claim_deadline: Instant,
    pub execution_deadline: Option<Instant>,
    pub result: Option<AutomationResult>,
    pub logs: Option<Vec<AutomationLog>>,
    pub error: Option<AutomationError>,
}

pub enum AutomationRequest {
    Exec(ExecRequest),
    Inspect(InspectRequest),
    Search(SearchRequest),
    Selection(SelectionRequest),
    Select(SelectRequest),
    Camera(CameraRequest),
    Focus(FocusRequest),
    Screenshot(ScreenshotRequest),
    Play(PlayRequest),
    Stop(StopRequest),
    Snapshot(SnapshotRequest),
    Preview(PreviewRequest),
}
```

`AutomationJobState` should keep the existing lifecycle names: `pending`, `claimed`, `succeeded`, `failed`, `timedOut`, plus future `canceled` only when a real cancellation endpoint exists. `kind` should be an explicit enum such as `exec`, `inspect`, `search`, `screenshot`, not a free-form string.

Arbitrary exec should remain a specialized job type:

- It carries trusted source, `scriptName`, source limits, undo label, and privileged result/log semantics.
- It should continue to be visually/audibly branded as Prism exec.
- Typed handlers should not depend on `ModuleScript.Source` or `require` unless they are intentionally executing arbitrary source.

## 6. HTTP and WebSocket layout

Confirmed current layout:

- `/api/rojo` returns server/session info.
- `/api/read/{ids}`, `/api/write`, `/api/serialize`, and `/api/ref-patch` support live sync and sync fallback.
- `/api/socket/{cursor}` is a WebSocket for server-to-plugin live-sync messages.
- `/api/exec/jobs...` is a local-only HTTP MessagePack job family.

Proposal:

Use one future endpoint family for typed one-shot jobs:

- `POST /api/automation/jobs`
- `GET /api/automation/jobs/next`
- `GET /api/automation/jobs/{id}`
- `POST /api/automation/jobs/{id}/complete`
- `POST /api/automation/jobs/{id}/cancel`

Keep `/api/exec/jobs...` during transition as a compatibility alias or thin wrapper around `AutomationJobKind::Exec`. New features should not add `/api/inspect/jobs`, `/api/search/jobs`, etc.

Use separate transport for artifacts:

- `POST /api/automation/jobs/{id}/artifacts` for plugin uploads.
- `GET /api/automation/artifacts/{artifactId}` for CLI downloads.
- `DELETE /api/automation/artifacts/{artifactId}` is optional; server cleanup by age and count is enough for MVP.

Use separate WebSocket stream endpoints for long-running subscriptions:

- `POST /api/automation/streams` to create a watch/profile/playtest stream request.
- `GET /api/automation/streams/{streamId}/socket` or `GET /api/automation/socket/{streamId}` for CLI event consumption.
- The plugin can either claim stream jobs over the automation job queue and then post events, or connect to a plugin-side automation socket later. MVP should prefer plugin HTTP event posts to the server and CLI WebSocket downloads from the server. That avoids requiring Studio to maintain a second outgoing WebSocket before the shape is proven.

Do not reuse `MessageQueue<AppliedPatchSet>` or `SocketPacketType::Messages` for automation. The current WebSocket is specifically live-sync server-to-plugin patch delivery.

## 7. Plugin session identity

Confirmed current state:

- `src/serve_session.rs` has a server-side `SessionId`, returned by `/api/rojo`.
- Existing read/write/serialize/ref-patch requests include `sessionId` to detect server changes.
- Exec routes do not include a plugin-session identifier.
- The server does not currently track connected Studio plugin sessions as first-class records.
- `plugin/src/App/init.lua` uses a Studio-side `__Rojo_SessionLock` in `ServerStorage` for Team Create sync ownership, but this is not an HTTP server session registry.
- Multiple WebSocket clients can technically connect to `/api/socket/{cursor}` because `src/web/api.rs` spawns a handler per upgrade. Exec prevents concurrent work by allowing only one claimed job, not by identifying the claimant.

Proposal for MVP:

- Add a `pluginSessionId` generated by the plugin after `/api/rojo` succeeds and passed in automation claim/complete calls.
- The server should remember the active automation plugin session for the current `ServeSession` after first claim/status heartbeat.
- If no plugin is connected, one-shot jobs can remain pending until claim timeout. `doctor` should report this as "server reachable, plugin not claiming automation jobs".

Future multi-session handling:

- Store `AutomationStudioSession { pluginSessionId, serverSessionId, placeId, gameId, studioUserId?, connectedAt, lastSeenAt, mode, label }`.
- A job may have `targetSession: "default" | { pluginSessionId }`.
- MVP should reject multiple active automation sessions or require an explicit chooser later. Do not auto-broadcast arbitrary exec to every Studio window.

## 8. Instance references

Problem:

Roblox Instances can be renamed, reparented, destroyed, duplicated, or recreated between commands. Documented `Instance.UniqueId` is Roblox security, so Prism should not rely on it. Full paths are human-friendly but not stable across rename/reparent. Debug IDs are not a good protocol foundation unless a live spike proves plugin access and stability, and even then they should be treated as diagnostic, not persistent.

MVP proposal:

- Accept target syntax as escaped DataModel paths, plus aliases:
  - `game`
  - `/` or `DataModel`
  - `workspace` or `Workspace`
  - `selection`
  - `@<session-local-id>` for references returned by previous commands in the same plugin session
- Return references as:

```json
{
  "kind": "instance",
  "id": "pinst-00000042",
  "path": "Workspace.Map.Model",
  "className": "Model",
  "name": "Model",
  "ancestry": [
    { "name": "Workspace", "className": "Workspace" },
    { "name": "Map", "className": "Folder" },
    { "name": "Model", "className": "Model" }
  ]
}
```

- The plugin owns a session-local weak registry from generated IDs to Instances. Returned references include path/class/name snapshots for human output and stale-reference diagnostics.
- On use, `@id` resolves through the registry and validates `Parent ~= nil`, `IsDescendantOf(game)`, class name if supplied, and optional ancestry hash. If stale, return a typed stale-reference error.
- Escaped paths should be deterministic and unambiguous: split on `.`, allow quoted segments for names containing `.`, `\`, control characters, or leading/trailing whitespace, for example `Workspace."Map.With.Dot"."Part \"A\""`. The exact grammar should be specified before implementation and tested.

Later evolution:

- Add reference leases with `expiresAt` and a CLI-visible `sessionId`.
- Add `refreshReference` and `resolvePath` handlers.
- If Roblox exposes a documented, plugin-accessible persistent identifier later, layer it in as an optional validation field, not as the only reference.

## 9. Roblox value encoding

Confirmed exec value schema:

- Rust `src/web/interface.rs` exposes tagged `ExecValue` variants: `nil`, `string`, `number`, `boolean`, `array`, and `table`.
- Plugin `plugin/src/Exec.lua` rejects NaN/infinity, cycles, sparse arrays, mixed array/dictionary tables, non-string dictionary keys, and unsupported values. It sorts table keys deterministically.

Proposal:

Create `AutomationValue` as a superset of `ExecValue`:

- Scalars: `nil`, `string`, finite `number`, `boolean`.
- Structured: dense `array`, sorted string-key `table`.
- Roblox primitives: `vector2`, `vector3`, `cframe`, `color3`, `udim`, `udim2`, `rect`, `enumItem`, `brickColor`, `numberRange`, `numberSequence`, `colorSequence`, `physicalProperties`.
- Instance references: `{ kind = "instanceRef", value = InstanceReference }`.
- Binary artifacts are never embedded as `AutomationValue`; use artifact IDs.

Keep encoding small and explicit. Values that cannot be represented safely should return a typed `unsupportedValue` error with a path such as `properties.CFrame` or `result.items[3]`.

## 10. Inspect and search

`prism inspect <target>` proposal:

Request:

```json
{
  "kind": "inspect",
  "target": "Workspace.Map",
  "depth": 1,
  "includeProperties": "default",
  "includeChildren": true,
  "maxChildren": 200
}
```

Response:

```json
{
  "reference": { "kind": "instance", "id": "pinst-42", "path": "Workspace.Map", "className": "Model" },
  "name": "Map",
  "className": "Model",
  "path": "Workspace.Map",
  "properties": {},
  "attributes": {},
  "tags": [],
  "children": []
}
```

Design choices:

- Target syntax should use the shared escaped path/reference grammar.
- Defaults: depth `1`, max children `200`, max total instances `2_000`, max properties `default`.
- `--json` should print deterministic JSON. Human output can be compact but should never be the only format.
- `--depth`, `--children`, and `--properties` map directly to request fields.
- Use a property allowlist per class for MVP. Avoid expensive or sensitive properties by default.
- Script source privacy: do not return `Source` from `Script`, `LocalScript`, or `ModuleScript` unless an explicit future flag is designed.
- Non-serializable Roblox datatypes return a placeholder error per property rather than failing the entire inspect.
- Sort attributes, tags, property keys, and children by current child order with deterministic tie behavior.

`prism search <query>` proposal:

- Avoid a full query language at first.
- Support flags: `--name`, `--class`, `--tag`, `--attribute key[=value]`, `--ancestor <target>`, `--case-sensitive`, `--limit`.
- Bare `<query>` should search name substring case-insensitively.
- Return an array of instance references with name, className, path, selected matched fields, and a truncated preview.

## 11. Selection

Confirmed external fact:

- Roblox documents `Selection:Get()` and `Selection:Set()` as usable by plugins and the command line.

Proposal:

- `prism selection` submits `{ kind = "selectionGet" }` and returns the selected instances as references in selection order.
- `prism select <target...>` resolves each target and submits `{ kind = "selectionSet", references = [...] }`.
- `prism select --clear` submits an empty set.
- In edit mode, selection changes should not be wrapped in `ChangeHistoryService` by default unless a live spike shows they are undoable and useful. Treat selection as Studio UI state, not DataModel mutation.

Studio verification required:

- Confirm installed plugin access to `Selection:Set()` in edit mode.
- Confirm behavior in Play Solo and whether changing selection during play should be rejected or allowed as a UI-only operation.
- Confirm how selection behaves for destroyed/stale references.

## 12. Camera and focus

Confirmed external facts:

- Roblox documents `Camera.CFrame`, `Focus`, `FieldOfView`, `ViewportSize`, `GetRenderCFrame`, and `ZoomToExtents`.
- `Workspace.CurrentCamera` is the current camera object in normal camera documentation.

Proposal:

- `prism camera` returns current camera CFrame, Focus, FieldOfView, FieldOfViewMode, CameraType, CameraSubject reference if serializable, and ViewportSize.
- `prism focus <target>` resolves an Instance and frames it:
  - `BasePart`: use `CFrame` and `Size`.
  - `Model`: use `Model:GetBoundingBox()`, reject empty models with no geometric descendants.
  - `Attachment`: frame around `WorldPosition` with a small default radius.
  - `selection`: frame union of selected supported objects.
- Mutate `Workspace.CurrentCamera` in edit mode by setting camera type/scriptable behavior only if needed and reversible enough for user expectations.

Studio verification required:

- Confirm installed plugin can reliably read and set `Workspace.CurrentCamera` in edit mode.
- Confirm whether `Camera:ZoomToExtents` works from plugin context.
- Confirm whether camera changes should be excluded from undo and snapshot diffs.

## 13. Screenshot feasibility and artifacts

Confirmed external facts:

- Current Creator Hub documents `StudioCaptureService:CanCaptureScreenshot()`, `RequestScreenshotPermissionAsync()`, and `CaptureScreenshot(options)` with plugin security.
- `StudioScreenshotCapture` exposes `GetBuffer(): buffer`, read-only `BufferFormat`, `BufferStatus`, `OriginalSize`, `Position`, `Resolution`, `UICaptureMode`, `GetErrors()`, and `ScaleAsync()`, all with plugin-security access where listed.
- `Capture.FilePathString` is Roblox script/security restricted on the separate in-experience `Capture` class, so local file-path handoff should not be assumed.

Proposal:

- Treat `screenshot` as a binary artifact job, not a normal completion payload.
- Plugin captures a screenshot, validates `BufferStatus`, reads `GetBuffer()`, and uploads the bytes to the server as an artifact associated with the job.
- Completion returns:

```json
{
  "kind": "screenshot",
  "artifactId": "...",
  "contentType": "image/png",
  "width": 1920,
  "height": 1080,
  "format": "png",
  "byteLength": 1234567
}
```

Artifact transfer options:

- Best MVP: plugin uploads artifact bytes to `POST /api/automation/jobs/{id}/artifacts`, CLI downloads `GET /api/automation/artifacts/{artifactId}` as raw bytes.
- Acceptable fallback: MessagePack `serde_bytes`/Luau buffer chunks if Roblox `RequestAsync` cannot send raw binary safely.
- Avoid base64 unless Roblox HTTP string-body constraints make binary unsafe; base64 adds size overhead and extra copies.
- Avoid local path handoff because plugin-accessible file paths are not established.

Recommended limits:

- Default max screenshot artifact: 16 MiB.
- Default max artifact retention: 5 minutes or 16 artifacts.
- Optional downscale in plugin before upload if requested size exceeds max.

Studio verification required:

- Confirm installed plugin can call `RequestScreenshotPermissionAsync` and how often permission prompts.
- Confirm `CaptureScreenshot` captures the 3D viewport, plugin widgets, game UI, or some combination.
- Confirm edit mode versus Play Solo differences.
- Confirm buffer format and whether `GetBuffer()` is PNG/JPEG bytes or raw pixels for each `BufferFormat`.
- Confirm whether user permission is required and how denial is reported.

## 14. Play and stop feasibility

Confirmed external facts:

- Roblox documents `RunService:Run()` and `RunService:Stop()` with plugin security.
- Roblox documents `StudioTestService:ExecutePlayModeAsync`, `ExecuteRunModeAsync`, `ExecuteMultiplayerTestAsync`, `CanLeaveTest`, and `LeaveTest`; several have plugin security.
- Existing Prism plugin code uses `RunService:IsEdit`, `IsRunning`, `IsStudio`, and `IsServer`, and already has Play Solo auto-connect logic in `plugin/src/App/init.lua`.

Proposal:

- `prism play` should first be a Studio lifecycle command with `{ kind = "play", mode = "playSolo" | "run" }`.
- `prism stop` should stop the active Studio test/run session if Prism started it or if a safe, explicit `--force` is later designed.
- The automation subsystem should record a `studioMode` in plugin status so CLI can explain when play/stop is unavailable.

Studio verification required:

- Confirm installed plugin can start Play Solo through `StudioTestService` or `RunService` without internal APIs.
- Confirm whether `RunService:Run()` starts Run mode, not Play Solo with a player.
- Confirm how to stop a play session from the edit-mode plugin versus a play-mode auto-connected plugin.
- Confirm whether `StudioTestService` methods require special beta flags or user permissions.

## 15. Doctor

Proposal:

CLI-only checks:

- Server reachable at address/port.
- `/api/rojo` returns MessagePack with expected protocol version.
- Host/origin/local-only behavior is sane for loopback.
- Server version, project name, address, port, expected place IDs, blocked place IDs.
- Exec/automation submission route exists.
- Queue capacity and stale jobs if the server exposes status.

Plugin-status checks:

- Plugin connected and claiming automation.
- Plugin session ID and active Studio mode.
- HTTP requests enabled in Studio. Current `plugin/http/Error.lua` maps disabled HTTP to a clear Prism error.
- Duplicate plugin/session detection.
- Exec availability and automation handler versions.
- Mismatched plugin/server version or protocol.
- Current DataModel place/game IDs.

Implementation proposal:

- Add `{ kind = "doctor" }` automation job returning a typed health object.
- Later add `GET /api/automation/status` for server-only state that does not need plugin execution.

## 16. Snapshot and diff

Snapshot proposal:

- A bounded DataModel snapshot should include session ID, root reference, traversal limits, deterministic node IDs, name, className, escaped path, attributes, tags, allowlisted properties, child order, and diagnostics for skipped values.
- Exclude script `Source` by default. Include script metadata only unless explicitly requested.
- Exclude or summarize terrain and binary content for MVP.
- Use deterministic hashes per node and for the whole snapshot.
- Large snapshots should be artifacts, not normal job result payloads.

Diff proposal:

- `+ instance`: created instance with class/name/parent/order and properties.
- `- instance`: removed instance reference plus last known snapshot.
- `~ property`: changed allowlisted property.
- `~ attribute`: changed attribute.
- `~ tag`: added/removed tag.
- `> reparent`: parent changed.
- `> reorder`: sibling order changed.

Storage recommendation:

- CLI memory for immediate `snapshot | diff` pipelines.
- Disk for user-requested `.prism-snapshot.msgpack` or JSON snapshot files.
- Server state only for temporary preview IDs and artifacts.
- Plugin state only for session-local instance references and active watch subscriptions.

## 17. Watch streams

Proposal:

`prism watch <target>` should use a stream, not HTTP polling.

Potential events:

- `descendantAdded`
- `descendantRemoved`
- `propertyChanged`
- `attributeChanged`
- `tagChanged`
- `ancestryChanged`
- `selectionChanged`

Transport:

- CLI opens a WebSocket to the server for a stream ID.
- Plugin subscribes to Roblox events for the target and posts batches to the server, or in a later phase opens a plugin automation WebSocket.
- Server forwards ordered batches to CLI and applies backpressure limits.

Requirements:

- Subscription IDs and explicit unsubscribe.
- Disconnect cleanup on CLI disconnect, plugin disconnect, and server shutdown.
- Event coalescing over short windows, for example 50-100 ms.
- Event count/byte limits with a `droppedEvents` marker.
- Initial snapshot option before live events.
- Ordering by plugin-side sequence number.
- No interference with `/api/socket/{cursor}` live-sync packets or `MessageQueue`.

## 18. Profile feasibility

Proposal:

Call the first version `profile` only if the output is honest about scope.

Likely supported through plugin DataModel traversal:

- Instance counts.
- Class counts.
- Script counts by class.
- Approximate hierarchy depth and child fanout.
- Counts of tagged/attributed instances.
- Counts of oversized strings or selected expensive properties if included.

Requires Studio verification or may be unsupported/private:

- Memory categories.
- Physics metrics.
- Script performance.
- MicroProfiler data.
- Render stats and GPU timings.

Do not call hierarchy counts a full profiler. Use output labels such as `hierarchySummary` and `unavailableMetrics`.

## 19. Preview-mode threat model

Preview Mode is not a secure sandbox. It is a workflow for previewing and reverting many ordinary DataModel edits from trusted automation.

Potentially reversible DataModel effects:

- Creating/destroying Instances.
- Property changes.
- Attributes.
- Tags.
- Parenting and reparenting.
- Sibling order.
- Many script source changes, subject to source access and snapshot limits.
- Some terrain changes only if explicit terrain snapshot/patch support is built.

Potentially non-reversible or external effects:

- HTTP requests.
- DataStore/Open Cloud actions.
- Marketplace, publishing, or asset generation actions.
- Plugin settings.
- Selection changes.
- Camera changes.
- External logging/analytics.
- Detached tasks that keep running after preview.
- User prompts.
- Play-mode effects.
- Filesystem-like plugin capabilities, if any are exposed.
- Side effects outside `ChangeHistoryService`.

Approach comparison:

- Begin recording, execute, capture diff, cancel recording: attractive but risky. `ChangeHistoryService` is for undo history, not security, and cancellation semantics need live proof for all mutation classes.
- Clone a bounded subtree and execute against the clone: safer for subtree-local edits but many scripts expect services such as `workspace`, `Lighting`, or `Selection`; transparent redirection is hard.
- Clone the whole DataModel into a temporary DataModel: not feasible with normal plugin APIs.
- Record mutations through an injected API: safest for deterministic apply, but not compatible with arbitrary Luau that mutates normal Roblox objects directly.
- Execute normally and derive a patch from before/after snapshots: feasible for trusted scripts and typed handlers; misses external effects and unsupported properties.
- Use Roblox undo as rollback: useful as a cleanup mechanism, not a security boundary.
- Re-run the script during apply: unsafe and non-deterministic because the script may depend on time, selection, random values, HTTP, or mutated state.
- Apply a captured deterministic patch instead of re-running: preferred when patch coverage is sufficient.

Recommended MVP:

- Name it Preview Mode, not sandbox.
- Run trusted source once in edit mode.
- Take a bounded before snapshot.
- Execute with a `ChangeHistoryService` recording.
- Take a bounded after snapshot.
- Compute a structured patch.
- Roll back live changes using Roblox undo or an explicit inverse patch, with live Studio verification deciding which is more reliable.
- Store the captured patch and before-hash in server state under a preview ID.
- `preview apply <preview-id>` applies the captured patch, not the original source.
- Reject apply if the referenced DataModel subtree changed since preview.
- Expire previews quickly.

## 20. Preview/apply protocol

Proposal:

Preview job request:

```json
{
  "kind": "preview",
  "scriptName": "destructive-edit.lua",
  "source": "...",
  "target": "game",
  "snapshotOptions": {
    "maxInstances": 10000,
    "propertySet": "default"
  }
}
```

Preview result:

```json
{
  "kind": "preview",
  "previewId": "...",
  "baseHash": "...",
  "patchArtifactId": "...",
  "summary": {
    "added": 3,
    "removed": 0,
    "propertiesChanged": 5,
    "attributesChanged": 1,
    "tagsChanged": 0,
    "reparented": 0,
    "reordered": 0
  },
  "rollback": {
    "ok": true,
    "method": "undo"
  }
}
```

Apply request:

```json
{
  "kind": "previewApply",
  "previewId": "...",
  "expectedBaseHash": "..."
}
```

Apply result:

```json
{
  "kind": "previewApply",
  "applied": true,
  "undoLabel": "Prism Preview Apply: destructive-edit.lua"
}
```

## 21. Undo and rollback limitations

Confirmed facts:

- Current live sync uses `ChangeHistoryService:TryBeginRecording("Prism: Patch ...")` and `FinishRecording(...Commit)` in `plugin/src/ServeSession.lua`.
- Current exec uses `TryBeginRecording("Prism Exec: <scriptName>")` and commits the recording in `plugin/src/Exec.lua`.
- Current code does not use `SetWaypoint` for exec. Creator Hub marks `SetWaypoint` as headed toward deprecation in favor of recordings.

Proposal:

- Typed mutating automation jobs should use `TryBeginRecording`/`FinishRecording`, not `SetWaypoint`.
- Read-only jobs should not open a recording.
- Runtime failures after partial mutation should commit one undoable action when the user asked for real execution.
- Preview should isolate user experience by rolling back, but must disclose that external effects are not rollback-protected.
- The controller, not user code, owns `FinishRecording`.

## 22. Automated playtesting prerequisites

Do not deeply design playtesting yet. Minimum prerequisites:

- Verified `play` and `stop`.
- Route execution to edit plugin, play server, and play client contexts.
- Stable plugin/session identity across edit/play transition.
- Input injection feasibility.
- Camera/screenshot support.
- UI inspection support.
- Log collection from relevant contexts.
- Assertion result schema.
- Artifact/report storage.
- Timeouts and cleanup for hung tests.
- Clear distinction between local Studio automation and publish/live-service effects.

## 23. Security and local-only boundaries

Confirmed current security boundaries:

- `src/web/mod.rs` rejects cross-origin requests before API routing.
- Exec routes in `src/web/api.rs` reject non-loopback peers using `reject_non_local`, matching `/api/open`.
- There is no authentication on exec.
- Exec is trusted-source only.

Proposal:

- Keep all `/api/automation/...` endpoints local-only for MVP.
- Keep source execution and mutating jobs opt-in from the CLI.
- Do not expose arbitrary automation to non-loopback peers without a separate authentication design.
- Include protocol and handler versions in automation status so the CLI can fail closed on mismatches.

## 24. Exact likely file plan

Rust CLI:

- Modify `src/cli/mod.rs` for new subcommands.
- Create or modify `src/cli/automation.rs` for shared submit/poll/render helpers.
- Create `src/cli/inspect.rs`, `src/cli/search.rs`, `src/cli/selection.rs`, `src/cli/select.rs`, `src/cli/camera.rs`, `src/cli/focus.rs`, `src/cli/screenshot.rs`, `src/cli/play.rs`, `src/cli/stop.rs`, `src/cli/doctor.rs`, `src/cli/snapshot.rs`, `src/cli/diff.rs`, `src/cli/watch.rs`, `src/cli/profile.rs`, and `src/cli/preview.rs` as phases justify.
- Keep `src/cli/exec.rs`, but route its HTTP job operations through shared automation helpers after the protocol migration.

Rust server/protocol:

- Create `src/automation.rs` for `AutomationJobStore`, job states, errors, cleanup, artifact registry, and common limits.
- Modify `src/serve_session.rs` to replace or wrap `exec_job_store` with `automation_job_store`.
- Modify `src/web/interface.rs` for `AutomationRequest`, `AutomationResult`, values, references, artifacts, status, and stream messages.
- Modify `src/web/api.rs` for `/api/automation/jobs...`, artifact routes, status routes, and stream setup.
- Keep `src/web/util.rs` MessagePack helpers.
- Keep `src/exec.rs` during migration or reduce it to exec-specific request/value helpers once automation owns queue mechanics.

Studio plugin:

- Create `plugin/src/Automation.lua` for the shared poller/dispatcher.
- Create `plugin/src/AutomationHandlers/*.lua` for inspect, search, selection, camera, screenshot, etc.
- Create `plugin/src/AutomationValue.lua` and `plugin/src/InstanceReferences.lua`.
- Modify `plugin/src/ApiContext.lua` to add generic automation claim/complete/artifact methods.
- Modify `plugin/src/ServeSession.lua` to start one automation poller after initial sync/WebSocket setup and stop it during disconnect.
- Modify `plugin/src/Types.lua` for automation validators.
- Keep `plugin/src/Exec.lua`, but make it an automation handler instead of its own poll loop.

Tests:

- Extend `tests/tests/serve.rs` and `tests/rojo_test/serve_util.rs` for generic automation HTTP routes and artifacts.
- Add Rust unit tests in `src/automation.rs`.
- Add plugin TestEZ specs for `Automation.lua`, `AutomationValue.lua`, `InstanceReferences.lua`, and each handler.

Documentation:

- Keep this file: `design/prism/AUTOMATION_ARCHITECTURE.md`.
- Later add command-specific design docs only for large uncertain areas such as screenshot, playtesting, and Preview Mode.

## 25. Testing strategy

Rust:

- Generic store: submit, claim, one active claim, complete success/failure/timeout/cancel, duplicate claim, duplicate completion, unknown job, capacity, size, cleanup, artifact retention.
- API routing: exact route matching, `/next` before `{id}`, malformed IDs, local-only rejection, MessagePack decode errors, source/payload/artifact limits.
- CLI: clap parsing, file/target validation, server check, status rendering, JSON output, HTTP error decoding, artifact download, timeouts.
- WebSocket streams: stream creation, event ordering, backpressure, disconnect cleanup, no interference with live-sync socket tests.

Plugin:

- One automation poller starts once from `ServeSession`.
- No polling before connection or after disconnect.
- No duplicate claim while executing/completing.
- Handler dispatch validates kind.
- Instance reference creation/resolution/stale detection.
- Value encoding rejects unsupported and cyclic values.
- Each typed handler returns deterministic payloads.
- Watch subscriptions coalesce events and clean up.
- Screenshot/play/preview specs use injected dependencies; live Studio spikes verify actual APIs.

Manual Studio:

- Inspect Workspace and selected Instances.
- Search by name/class/tag/attribute.
- Selection get/set/clear.
- Camera read and focus on BasePart, Model, Attachment, and selection.
- Screenshot permission, viewport capture, output image validity, Play Solo behavior.
- Play/stop lifecycle behavior.
- Preview mutation, rollback, apply, stale apply rejection.
- Disconnect/reconnect recovery.

## 26. Open Studio verification questions

- Can installed plugins use `StudioCaptureService` screenshot APIs without beta flags or special permissions?
- What exactly is contained in `StudioScreenshotCapture:GetBuffer()` for each buffer format?
- Does screenshot capture include only the 3D viewport, plugin widgets, CoreGui/game UI, or different modes controlled by `UICaptureMode`?
- Can installed plugins start Play Solo reliably through `StudioTestService:ExecutePlayModeAsync`, or only Run mode through `RunService:Run()`?
- Can an edit-mode plugin stop a play session it started after execution context changes?
- How does the current Prism auto-connect playtest path interact with an automation plugin session identity?
- Are `Selection:Set` changes acceptable in Play Solo, or should selection be edit-mode only?
- Can `Workspace.CurrentCamera` be set and focused reliably in edit mode from an installed plugin?
- Which property reads are expensive or restricted for inspect/snapshot across common services?
- Can undo cancellation reliably roll back all mutation classes needed for Preview Mode?
- How should Terrain be snapshotted, diffed, and rolled back if included?
- What memory/performance metrics are accessible to an installed plugin without private APIs?

## 27. Recommended implementation phases

1. Automation status and identity: add plugin session IDs, server automation status, and `doctor` checks without moving exec.
2. Generic `AutomationJobStore`: extract queue mechanics from `ExecJobStore`, keep `/api/exec/jobs...` compatible.
3. Generic automation HTTP routes: add `/api/automation/jobs...` and make exec an `AutomationRequest::Exec` adapter.
4. Plugin automation poller: replace the exec-specific poller with one dispatcher while preserving exec behavior.
5. Shared values and instance references: implement `AutomationValue` and session-local Instance refs.
6. Read-only typed commands: inspect, search, selection, camera.
7. Mutating typed commands: select and focus with clear undo/Studio-mode rules.
8. Screenshot spike and artifact transport.
9. Snapshot and diff.
10. Watch streams over a separate automation stream transport.
11. Play/stop feasibility spike, then lifecycle commands if verified.
12. Preview Mode MVP with before/after snapshots, rollback, stored patch, and apply without rerunning source.
13. Profile hierarchy summary, with honest unavailable metrics.
14. Automated playtesting design once play/stop, screenshots, streams, logs, and assertions exist.
