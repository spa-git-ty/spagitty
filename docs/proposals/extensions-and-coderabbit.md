<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Proposal and implementation brief: Spagitty extensions and CodeRabbit

**Prepared:** 2026-10-06  
**Audience:** Claude Code implementing this feature in the Spagitty repository  
**Status:** Proposed; no application implementation is included in this document

## 1. Outcome

Make Spagitty extensible through a documented, versioned public contract. Developers must be able to build, test, package, and distribute an extension without modifying Spagitty's source or rebuilding the application. Users must be able to install, configure, enable, disable, update, and remove those extensions from Spagitty.

Build **CodeRabbit for Spagitty** as the first official extension using exactly that public contract. It must review local changes and farm task changes, present findings in Spagitty, send selected findings back through the farm's existing repair workflow, and display and request CodeRabbit reviews on GitHub pull requests.

The first extension is also the proof that the platform works: removing its package must remove its contributions without removing a special CodeRabbit feature from the host. A second, independently packaged example extension must work with the same host without a new Rust enum variant, Tauri command, or Svelte component for that extension.

Deliver the entire required v1 scope below. The phases describe implementation order, not alternative stopping points.

## 2. Read this before implementation

Read the root `AGENTS.md`, the canonical shared amendments book and its Appendix A, `CONTRIBUTING.md`, and the current architecture, testing, CI, branding, and agile records. Follow the current rules when translating this proposal into work items, branches, commits, reviews, and releases.

**Rules limitation in preparing this proposal:** `docs/AMENDMENTS.md` is a pointer. The canonical `/home/maxmya/dev/agents/docs/AMENDMENTS.md` was unavailable on this Windows checkout and also returned “No such file or directory” through WSL. This proposal does not claim compliance with unread amendments. Before implementation, locate and read the canonical book; report an unresolved rules prerequisite rather than inventing its contents or treating a summary as authoritative.

Inspect the checkout and preserve unrelated changes. During proposal preparation, the checkout already contained changes to `src-tauri/Cargo.toml`, an untracked `design_handoff_review/` directory, and an untracked generated Windows schema. Recheck their state when beginning; they are not part of this proposal's scope.

Do not use this document to override repository rules. If current rules conflict with a design choice here, record the conflict and adjust the proposal with a concrete explanation. Do not silently weaken verification, review, network, licensing, or release requirements to make the feature easier to ship.

## 3. What exists today and what to reuse

The inspected checkout identifies itself as version `0.8.1`. It is a local-first desktop Git client and agent farm built with Tauri 2, Svelte 5/SvelteKit, Rust, and Bun. No user extension framework currently exists in the inspected source; the existing Tauri dialog plugin is an application dependency, not an extension system for users.

| Existing seam | Evidence in this checkout | Required treatment |
| --- | --- | --- |
| Git operations | `crates/spagitty-core/src/` | Keep all Git operations here; extensions call typed host operations instead of reimplementing Git inside the host. |
| HTTP requests | `crates/spagitty-core/src/forge/http.rs` | Keep the application's single HTTP client boundary; add typed forge operations through this boundary. |
| Forge credentials | `crates/spagitty-core/src/forge/keychain.rs` | Reuse connected accounts; never give an extension or webview the forge token. |
| Farm orchestration | `crates/spagitty-farm/src/service.rs` | Keep task transitions, repair routing, autonomy, and merge authority in the farm service. |
| Review decisions | `crates/spagitty-farm/src/review/decision.rs` | Preserve the current independent review contract; add richer extension results alongside it. |
| Reviewer independence | `crates/spagitty-farm/src/review/reviewer.rs` | Keep the rule that an implementing agent cannot approve its own work. |
| Commit evidence | `crates/spagitty-farm/src/verification/evidence.rs` | Bind required extension reviews to the exact task change and invalidate stale evidence. |
| Process cancellation | `crates/spagitty-farm/src/execution/tree.rs` | Reuse its Unix process-group and Windows Job Object behavior through a shared module. |
| UI/backend calls | `src/lib/api.ts`; the documented subsystem precedent in `src/lib/farm/api.ts` | Add typed wrappers at an explicit bridge, never direct calls scattered through components. |
| Command registry | `src/lib/palette/store.svelte.ts` | Register and unregister extension commands through the existing registry. |
| PR interface | `src/lib/requests/` | Add extension contributions to the current review workspace. |
| Settings and feedback | `src/lib/settings/`, `src/lib/ui/` | Reuse the existing theme, dialogs, notices, and settings patterns. |
| Network boundary tests | `src/lib/requests/requests.test.ts` | Preserve their guarantees and extend their scan to new first-party crates where necessary. |

Some prose in existing files predates the code: for example, the forge module header says read-only while `forge/review.rs` implements writes, and architecture prose describes a single process boundary despite the farm's separate agent runner. Inspect executable behavior and update documentation touched by this feature accurately; do not interpret stale comments as permission to add uncontrolled boundaries.

## 4. Scope and decisions

### Required for v1

- A native extension host with a language-neutral, versioned process protocol.
- A manifest schema, capability model, lifecycle, compatibility rules, package format, and installation management.
- Public command, context-menu, settings, panel, and review-provider contribution points.
- A TypeScript developer SDK, scaffold, package validator, protocol test harness, and a working example extension.
- A first-party CodeRabbit package that is bundled with Spagitty but disabled until configured and enabled.
- CodeRabbit reviews of local changes and clean committed farm task changes.
- Findings, progress, cancellation, history, stale-result handling, and selected-finding handoff to an agent.
- Optional repository policy making CodeRabbit an additional required farm review gate.
- GitHub PR review display and explicit incremental/full review requests.
- Tests, documented developer and user workflows, and build validation on supported release targets.

### Deferred beyond v1

A hosted marketplace, automatic extension downloads or updates, a cryptographic publisher registry, extension dependencies, hot replacement during a running review, arbitrary HTML/JavaScript UI, a WASM sandbox, a public inbound webhook server, CodeRabbit-driven automatic edits, and CodeRabbit PR integration for GitLab/Bitbucket.

The host API should use forge-neutral types. The first PR adapter can expose GitHub support explicitly and return a clear unsupported result for other forges. Local CLI review is a separate capability and must not depend on GitHub PR support.

### Runtime choice

Use **separate native worker processes communicating over stdin/stdout**. Do not load third-party dynamic libraries into Spagitty's process, execute extension JavaScript inside the main webview, or require users to have a JavaScript runtime installed just to use a packaged extension.

An extension may be written in any language that implements the protocol. A release package contains self-contained executables for its supported targets. The TypeScript scaffold may use the project's Bun toolchain to produce such executables; test the actual packaging approach on each claimed target before documenting it as supported. The first-party CodeRabbit worker may be implemented in Rust for straightforward integration with the existing release toolchain.

**Trust model:** v1 extensions are trusted native software running with the user's operating-system privileges. Process separation improves crash isolation, and host capabilities restrict access through Spagitty's API; neither provides an OS sandbox. A native worker can access ambient files or make its own network requests. Say this plainly in installation information and developer documentation. Do not advertise capabilities as filesystem or network containment.

## 5. Proposed architecture

```text
Svelte screens and shared UI
  | typed wrappers; host-rendered extension contributions
  v
Tauri desktop composition and command/event bridge
  |                      |
  v                      v
spagitty-extensions      spagitty-farm
  |                      | declares a supplemental-review interface
  |                      | implemented by the desktop composition layer
  +-------> spagitty-core <------+
  |
  +-------> supervised native extension workers
                 |
                 +--- typed host callbacks for Git, forge data, tools, storage

Shared process-tree primitives are used by farm runs and extension runs.
```

Add `crates/spagitty-extensions` for manifests, package validation, registry, protocol, lifecycle, operation tracking, permissions, and contribution metadata. It must have no Tauri or frontend dependency. It may depend on `spagitty-core` for typed repository services; it must not depend on `spagitty-farm`.

Declare the supplemental review interface in the farm and supply it from the desktop composition layer. This avoids an extensions↔farm dependency cycle and prevents an extension host from owning farm state transitions. The host never receives a callback that can directly set a task to `Done`, merge a branch, or overwrite verification evidence.

Extract only the reusable process-tree primitives into a small internal `spagitty-process` crate if needed. The current `tree` module is private to the farm. Keep its cancellation tests and platform behavior; do not duplicate the Windows Job Object implementation or reuse the farm's merged stdout/stderr transcript reader for the extension protocol. Protocol stdout and diagnostic stderr must remain separate.

Add a thin `src-tauri/src/extensions.rs` bridge and a `src/lib/extensions/` store and host-rendered UI. Prefer extension command wrappers in `src/lib/api.ts`; if a separate subsystem bridge is chosen, document its allowed boundary as the farm does and test that components never invoke Tauri directly.

The boundary change must be described honestly: Spagitty still has one first-party HTTP client, but a user-enabled CodeRabbit CLI and trusted extension workers can communicate with external services. Update privacy and architecture statements that currently imply only forge traffic can leave the machine.

## 6. Standard extension contract

### 6.1 Identity and manifest

Use `extension.json` at the package root. Publish an authoritative JSON Schema, valid and invalid fixtures, and generated extension-protocol types where useful. Keep the existing application's hand-mirrored wire types consistent with its current conventions.

The following is a **proposed Spagitty manifest**, not an existing CodeRabbit configuration or API:

```json
{
  "manifestVersion": 1,
  "id": "spagitty.coderabbit",
  "name": "CodeRabbit",
  "version": "1.0.0",
  "publisher": "Spagitty",
  "description": "Review changes with CodeRabbit and bring findings into Spagitty.",
  "license": "GPL-3.0-or-later",
  "engines": {
    "spagitty": ">=0.9.0 <1.0.0",
    "extensionApi": "^1.0.0"
  },
  "runtime": {
    "kind": "native-process",
    "entrypoints": {
      "x86_64-pc-windows-msvc": "bin/windows-x64/coderabbit-extension.exe",
      "x86_64-unknown-linux-gnu": "bin/linux-x64/coderabbit-extension",
      "x86_64-apple-darwin": "bin/macos-x64/coderabbit-extension",
      "aarch64-apple-darwin": "bin/macos-arm64/coderabbit-extension"
    }
  },
  "activation": ["onCommand", "onReviewProvider"],
  "capabilities": {
    "required": ["repository.read", "review.provide", "tools.execute"],
    "optional": ["forge.pullRequest.read", "forge.pullRequest.comment"]
  },
  "externalTools": [
    {"id": "coderabbit", "executableNames": ["coderabbit", "cr"]}
  ],
  "contributes": {
    "commands": [
      {"id": "reviewChanges", "title": "Review changes with CodeRabbit"},
      {"id": "reviewTask", "title": "Review task with CodeRabbit"},
      {"id": "requestPullRequestReview", "title": "Request CodeRabbit review"}
    ],
    "reviewProviders": [
      {"id": "review", "targets": ["workingCopy", "farmTask"]}
    ],
    "panels": [
      {"id": "findings", "title": "CodeRabbit", "renderer": "reviewFindings"}
    ],
    "settings": [
      {"key": "region", "type": "enum", "values": ["us", "eu"], "default": "us"},
      {"key": "farmMode", "type": "enum", "values": ["off", "advisory", "required"], "default": "off"}
    ]
  }
}
```

The example application version range is illustrative: set the actual range to the version that introduces this API under the repository's release rules. A listed entrypoint is a support claim; only ship target entries with built and tested binaries. The manifest's publisher text must not determine whether an extension receives the official badge.

Define IDs, allowed characters, size limits, duplicate rules, required fields, platform triples, version ranges, settings types, and contribution shapes in the schema. Use namespaced IDs such as `com.example.hello`; the host qualifies local command IDs with the extension ID. Reject duplicate ownership and attempts to replace a bundled official package through an ordinary third-party import.

Compatibility has three independent dimensions: manifest version, extension API version, and application version. Negotiate API versions during the handshake. Unsupported major versions or unmet application ranges produce an install/enable explanation before execution. Unknown capabilities are errors; safe additive protocol fields may be ignored within a compatible version.

### 6.2 Protocol and lifecycle

Use JSON-RPC 2.0 with one compact UTF-8 JSON message per line. stdout is protocol-only; stderr is bounded diagnostic output. Use separate request-ID namespaces for host and worker messages and support concurrent callbacks without deadlocking a running review.

Specify these host-to-worker operations:

| Method | Purpose |
| --- | --- |
| `extension.initialize` | Negotiate versions, provide granted capabilities, session identity, locale, and host limits. |
| `extension.activate` | Activate for a permitted context; return validated contribution readiness. |
| `command.execute` | Run a namespaced command against a typed context. |
| `review.start` | Start a review with a host-owned review ID and snapshot description. |
| `operation.cancel` | Request cancellation; the host can also terminate the owned process tree. |
| `settings.changed` | Notify a worker of validated setting changes. |
| `extension.deactivate` | Release subscriptions and stop work before disable or removal. |

Specify worker-to-host progress and result notifications, namespaced logs, and a terminal operation result. Every operation carries an extension ID, session token, operation ID, and relevant review/snapshot ID. Exactly one terminal result is accepted. Terminal operations reject later findings and callbacks; duplicate completion is harmless and cannot advance a farm task twice.

Use explicit states: installed, disabled, starting, active, stopping, failed, and incompatible. Static commands and settings may be registered from a validated manifest without starting the worker. Start workers lazily. Opening a repository must not start a review or send its source to CodeRabbit.

Define initialization and cancellation deadlines, bounded message/output sizes, per-extension concurrency, operation duration limits, and error codes. A reasonable initial policy is a short handshake timeout, a five-second cooperative cancellation grace period, and a configurable review limit long enough for substantial reviews. Use provider heartbeats for inactivity detection and a separate absolute deadline. Do not kill a healthy long review merely because it has produced no findings yet.

On a crash, disconnect the worker, fail its running operations, unregister its active contributions, and leave Git/farm/UI services responsive. Avoid restart loops. An explicit retry or bounded restart policy must never duplicate an external comment, billable review, or farm repair request.

### 6.3 Typed host API and capabilities

| Public service | Capability and behavior |
| --- | --- |
| Repository context | `repository.read`; returns a scoped opaque repository/worktree handle and requested metadata. |
| Diff and change snapshot | `repository.read`; host computes the scope, files, revisions, and snapshot identity through the core. |
| Review publication | `review.provide`; structured findings, progress, and final outcome for a host-issued review ID. |
| External tool execution | `tools.execute`; declared, selected executable with structured argument arrays, approved cwd, streaming output, cancellation, and recorded outcome. |
| Forge PR snapshot | `forge.pullRequest.read`; typed PR metadata, reviews, discussion, inline comments, checks, and pagination completeness. |
| Forge PR comment | `forge.pullRequest.comment`; post an exact previewed body to the selected PR under the current action policy. |
| Extension settings/storage | Implicit private namespace; settings and data belong to this extension and repository scope. |
| UI notices and panels | Host-rendered data only; no webview handle, arbitrary DOM access, or executable markup. |

Enforce grants on every host callback, not only in the UI. Check the extension's identity, repository scope, operation context, and active state. Reject expired handles and callbacks after disable. Do not expose raw arbitrary Tauri invocation, unrestricted HTTP proxying, arbitrary shell strings, Git write commands, farm status setters, or token reads in v1.

External tool definitions must include validated command profiles and argument schemas in the final manifest contract; the compact example above only illustrates detection. Validate tool identity, required options, scope selectors, argument sizes, and approved working directories. Use a direct executable plus argv, never concatenate review paths or branch names into a shell command. Do not accept repository-provided executable paths as implicitly trusted.

Use the existing forge account inside the backend. A worker receives the response data needed for its operation, not credentials or arbitrary authenticated URLs. For optional capabilities, disabling PR access must leave local CodeRabbit review usable.

### 6.4 Contribution points

Implement these as reusable host components:

- Command palette commands with context, enabled state, and a reason when unavailable.
- Context-menu actions for working changes, a farm task, and a selected PR.
- Validated settings: boolean, enum, text, number with bounds, and executable selection. Secrets are not ordinary settings fields.
- A review findings panel with summary, progress, severity, source location, details, history, and action buttons.
- A PR panel that can render discussion, review status, and links supplied through typed data.

Use finite declarative context predicates, not JavaScript expressions or `eval`. Contributions return data, never imported Svelte components. Reuse Spagitty typography, spacing, icons, focus behavior, dialogs, and notices. Markdown rendering must suppress executable HTML and automatic remote media loading; only approved links open after a user action.

A failed or disabled extension must leave no dead commands or orphaned subscriptions. Register contributions by stable namespaced ID and dispose them as a group.

## 7. Developer workflow and packaging standard

Provide a documented developer kit under `packages/extension-sdk/` and supporting tooling under `tools/extensions/`. It can be developed in this repository before any external package publication. Do not publish to npm or another service merely to finish the implementation.

The TypeScript SDK should provide manifest/protocol types, a transport implementation, handler registration, typed host callbacks, progress/results, cancellation, validation helpers, and a test host. Include a short implementation guide for other languages; JSON-RPC and the schema are the standard, not the SDK's internal classes.

Offer these documented tool actions, with names finalized in implementation:

```text
create       Scaffold an extension with a manifest, command, settings, and tests.
dev          Attach an explicitly selected development directory to Spagitty.
validate     Validate schema, IDs, compatibility, capabilities, and package files.
test         Run protocol and lifecycle conformance tests against a fake host.
pack         Produce a validated .spagitty-extension package with checksums.
inspect      Display identity, contents, targets, grants, and integrity information.
```

Local tools must have runnable repository scripts and documented commands. Do not write documentation for a scaffold or package publisher that does not exist.

The developer path must be demonstrable:

1. Create `com.example.hello` using the scaffold.
2. Add a command and a declarative panel through public APIs.
3. Run the test harness and load it in explicit developer mode.
4. Package a native target, import it into an unchanged Spagitty build, and run it.
5. Disable and uninstall it; verify that its contributions disappear.

Release packages use a ZIP-based `.spagitty-extension` container with `extension.json`, target binaries, license/notice files, an optional README, and an integrity index. Validate extraction before running anything: reject traversal, absolute paths, drive/UNC paths, symlinks/reparse entries, duplicate paths including case collisions, excessive expansion, unexpected executables, and missing declared binaries. Checksums prove integrity, not publisher identity.

Store installed packages under the application's platform-specific data directory, not inside the repository. Keep immutable version directories and an atomic installed-version pointer. Use restrictive permissions where supported. Development directories are a separate explicit mode; ordinary repository files never auto-install or auto-enable an extension.

Repository settings and review history may use the existing excluded `.spagitty/` convention with atomic writes. Keep grants in user application state keyed by extension/version/capability and repository identity; a committed repository file cannot grant itself permission. Define how main checkouts and farm worktrees share that repository identity.

Updates must stage and validate before switching versions. Finish or cancel active operations first, keep the previous version available for rollback, and require grant review for added capabilities. Migration failure preserves the previous working installation. Uninstall removes only the validated extension-owned directory; offer a clear choice about keeping history and leave the separately installed CodeRabbit CLI intact.

The first-party package is trusted as bundled content through Spagitty's release process, not through a manifest assertion. Update that bundled package through an application update in v1; a local import cannot overwrite its reserved identity or acquire its official badge. Support local package imports and file-based updates for third-party extensions; a marketplace is not required for v1.

## 8. Shared review data model

Introduce a general extension review model with wire fixtures shared across the Rust host and frontend. The field names below are proposed host types; they do not describe CodeRabbit's own output.

```ts
type ReviewRunStatus =
  | 'queued' | 'running' | 'completed' | 'incomplete'
  | 'skipped' | 'failed' | 'cancelled' | 'stale' | 'actionRequired';

type ReviewGate = 'notEvaluated' | 'pass' | 'changesRequested' | 'blocked';

interface ReviewSnapshot {
  id: string;
  repositoryId: string;
  target: 'workingCopy' | 'farmTask' | 'pullRequest';
  taskId?: string;
  pullRequestNumber?: number;
  baseCommit: string;
  headCommit: string;
  scope: 'committed' | 'uncommitted' | 'tracked' | 'includeUntracked';
  contentDigest: string;
}

interface ReviewFinding {
  id: string;
  reviewId: string;
  providerId: string;
  severity: 'critical' | 'high' | 'medium' | 'low' | 'info' | 'unknown';
  providerSeverity?: string;
  path?: string;
  startLine?: number;
  endLine?: number;
  side?: 'old' | 'new';
  title: string;
  message: string;
  suggestion?: string;
  sourceUrl?: string;
  disposition: 'open' | 'acknowledged' | 'dismissed' | 'sentToAgent';
}

interface ReviewResult {
  reviewId: string;
  providerId: string;
  providerVersion: string;
  snapshotId: string;
  status: ReviewRunStatus;
  summary: string;
  findings: ReviewFinding[];
  completeness: 'complete' | 'partial' | 'unknown';
  startedAt: string;
  finishedAt?: string;
}
```

The host computes the gate from completion, validated evidence, current snapshot, policy, and findings. A provider cannot set the farm's merge decision merely by emitting a `pass` string. Provider completion and provider approval are different concepts.

Optional line locations stay absent when not supplied; never manufacture a line number or attach a finding to a guessed diff hunk. Resolve paths safely against the relevant snapshot, supporting deleted/renamed files without falling through to unrelated files on disk.

Give findings stable IDs within their review and preserve source IDs where available. Group related findings for display without discarding independent results. Acknowledging, dismissing, or sending a finding to an agent does not prove that code changed or a required gate passed. If later policy supports waivers, store them separately with an actor and reason; they must never look like CodeRabbit approval.

Persist minimal review records with a schema version, provider/version, snapshot, timestamps, completeness, dispositions, and result. Avoid persisting full patches or source text by default. Offer history deletion and a documented retention policy. Treat raw provider logs and review messages as potentially sensitive and redact before storage, UI events, and diagnostic export.

## 9. First official extension: CodeRabbit

### 9.1 Integration surface and setup

Use the official CodeRabbit CLI for local review and the existing forge API for PR discussion. Do not invent a public CodeRabbit REST review endpoint or scrape its website. Spagitty's extension is separate from CodeRabbit's existing Claude Code plugin; users should not need Claude Code installed to review changes in Spagitty. The native Claude plugin is documented in the [CodeRabbit Claude Code integration guide](https://docs.coderabbit.ai/cli/claude-code-integration).

Detect `coderabbit` first, with `cr` as an alias, or let the user select the executable. Resolve an absolute path, show the version, and verify required command support. Prefer a tested minimum of CLI 0.7.7 for the modern completion contract, then publish the actual tested version range after fixture validation. An incompatible version gets installation/update guidance, not a brittle plain-text parser fallback.

Current documentation provides a signed native Windows x64 installation path. Support Windows directly rather than requiring WSL, while checking the actual supported architecture and prerequisites. Link to the official installation page; do not silently download or execute an installer. [CodeRabbit Windows installation](https://docs.coderabbit.ai/cli/windows).

Default authentication to the user's existing CLI login. Inspect authentication using `coderabbit auth status --agent`. Provide an explicit sign-in action with region selection that delegates to the CLI's browser login. Let the CLI own its credential store; do not copy its credentials into Spagitty or pass secrets in review argv. API-key/headless authentication is a later opt-in path unless a secure documented mechanism is implemented. [CodeRabbit headless authentication](https://docs.coderabbit.ai/cli/headless-cli-integration).

Show a connection state that distinguishes missing tool, unsupported version, signed out, ready, denied capability, and failed diagnostics. Checking a binary locally is different from running diagnostics that contact a service; keep network diagnostics explicit.

### 9.2 Local reviews

Offer “Review changes with CodeRabbit” from Changes and the command palette. Show the base, scope, affected files, and exclusions before a first upload. Select the repository's actual base from the current context; do not hard-code `main` or fetch a remote just to guess it.

Run the selected tool through the host broker using an argument array. Typical documented commands include:

```text
coderabbit review --agent --uncommitted
coderabbit review --agent --committed --base <selected-base>
coderabbit review --agent --uncommitted --include-untracked
```

Keep raw untracked files opt-in, distinguish staged new files from untracked files, and do not alter staging just to include a file. Default to the scope the user selected. Reviews are cloud-connected work even though invoked against local changes. [CodeRabbit CLI usage](https://docs.coderabbit.ai/cli).

Stream progress into the generic findings panel. Show meaningful state and elapsed time; do not invent a percentage from the number of findings. Cancellation must stop the owned process tree and invalidate the operation even if late output arrives.

For interactive working-copy reviews, fingerprint relevant revisions, index state, and file contents before and after. If they change, mark the result stale. Such results are advisory; a mutable working-copy review must never satisfy the committed farm merge gate.

For required farm review, use a clean committed snapshot, a pinned base revision, and the existing worktree lease/evidence mechanisms. Prefer the CLI's supported committed-snapshot/base-commit options where verified against installed help and fixtures. If the tool cannot review the exact intended snapshot, create a managed review worktree through the core or block the gate with a clear reason. Never quietly review another branch, an updated base, or a broader/narrower diff and call it equivalent.

### 9.3 Structured output adapter

CodeRabbit's agent stream is newline-delimited JSON. Dispatch by `type`; findings expose `severity`, `fileName`, `codegenInstructions`, `suggestions`, and a fallback `comment`. Other documented events include context, status, heartbeat, completion, and errors. The provider severities are critical, major, minor, trivial, info, and none. Map them conservatively to the host model and preserve the original. A completion label alone is insufficient: require the process outcome, inspect failure/completeness fields, and reject unreviewed files. A no-change skip is distinct from approval. [CodeRabbit output and completion reference](https://docs.coderabbit.ai/cli/reference).

Build the adapter from captured, redacted fixtures for the supported CLI versions. Unknown additive fields are tolerated; unknown event types are logged within limits. Malformed framing, conflicting terminal events, or unsupported mandatory result shapes produce an incomplete/error result. A provider finding's suggested command remains text and is never automatically executed.

Use `critical → critical`, `major → high`, `minor → medium`, `trivial → low`, and `info`/`none → info` as the initial policy mapping. Unknown severities remain unknown and block a required review until assessed. Render missing locations at file/review level rather than inventing anchors.

The host must distinguish successful analysis with findings from complete analysis without findings, retained findings from a prior run, failure after partial output, and a skipped empty scope. Do not turn an empty stdout stream or exit code zero by itself into approval.

### 9.4 Data sharing, quota, and billing

Keep CodeRabbit disabled by default. During first repository enablement, explain that reviewing local code sends selected code/context to the user's CodeRabbit service and uses their account. Store that consent separately from installing the extension. Enabling an extension must not immediately review the current repository.

Provide US/EU region selection consistent with the user's CLI account; show the selected region and account information without exposing a credential. For restricted networks, point to the provider's [network requirements](https://docs.coderabbit.ai/cli/network-requirements); support the platform's established proxy/certificate behavior rather than bypassing TLS checks.

CodeRabbit's documented agent mode can return an action-required billing result instead of assuming consent. Keep that state visible. Do not automatically add `--use-credits`, retry with paid credits, or treat a required payment action as a successful review. Any paid continuation must be an explicit, separate action tied to the current review content and price information. [CodeRabbit CLI consent behavior](https://docs.coderabbit.ai/cli).

Respect rate limits and provider retry information. Do not retry an ambiguous review start or a write automatically, and do not run unlimited fix/review loops. Expose “Retry” and keep the previous result identifiable.

### 9.5 Fix workflow

From a completed or partial review, users may select findings and choose “Send to agent.” Send structured issue context through Spagitty's service, with the reviewed snapshot and source attribution. Do not directly message agents from the extension worker and do not allow provider text to replace the task's instructions or repository rules.

For a farm task, use its existing change-request/repair path. For an ordinary local review, offer creation of a repair task in a separate farm worktree using the existing task APIs. Preserve the user's original checkout. If the reviewed work is uncommitted and cannot be represented in that task safely, require an explicit supported snapshot/commit step rather than modifying or staging it silently.

After repairs, rerun verification and review against the new commit. Show a bounded retry count; a practical default is at most two automatic repair cycles within previously authorized autonomy. Beyond that, leave the task awaiting a person. “Sent to agent” never means “fixed.”

## 10. Farm integration and review authority

Add repository-scoped policy with three modes:

| Mode | Behavior |
| --- | --- |
| Off | No automatic CodeRabbit review; an explicitly requested review can still run. |
| Advisory | Run after verification when autonomy/consent permits; show findings without creating a new merge requirement. |
| Required | A current, complete CodeRabbit result with no blocking findings is an additional merge requirement. |

Keep **Off** as the default. Persist the chosen policy, provider ID, blocking severity threshold, and repair budget. Do not rewrite the existing autonomy modes or pretend that installing CodeRabbit enables unattended cloud work.

In Required mode the sequence is:

```text
Implementation finishes
  -> repository verification
  -> supplemental CodeRabbit review of the verified committed change
  -> repair if required, then verification and supplemental review again
  -> existing independent agent/human review under current autonomy rules
  -> current evidence and policy checks
  -> merge only through the farm's existing authority
```

Default blocking threshold: medium or higher after severity normalization. Low/info findings remain visible. A different threshold is an explicit repository policy setting recorded with the review.

CodeRabbit supplements the independent reviewer in v1. Do not replace `AgentId` checks with an extension identity or synthesize a reviewer agent that secretly approves its own task. Existing human review paths remain subject to their current rules. CodeRabbit output cannot declare acceptance criteria met, tests passed, or a task done.

Enforce a required supplemental gate at the common merge-check boundary for both automatic and manual merge paths. UI button state is insufficient. A disabled, removed, crashed, unauthenticated, incompatible, incomplete, cancelled, unknown, or stale required provider blocks with a specific reason; it must not become a silent bypass. A no-change result cannot pass for a task with a non-empty expected diff.

Bind evidence to repository, task, provider/package version, base commit, head commit, reviewed scope, policy version, and relevant content/configuration identity. A change to any material input invalidates the gate. Recheck at merge time, after restart, and after repairs. Existing independent review and verification evidence must still match the committed task worktree.

If a user intentionally changes Required to Advisory/Off, record a policy change and its actor. Do not report that as CodeRabbit approval. Reconcile saved tasks without dropping work or relying on an in-memory-only review cache.

Add farm events for supplemental review requested, started, completed, failed/cancelled, stale, and findings handed back. The desktop bridge emits frontend events; the extension never writes the farm's append-only log directly.

## 11. GitHub pull request integration

Provide a CodeRabbit panel inside the existing PR workspace with the latest bot summary, findings, associated review/check information, source links, and refresh state. It must distinguish “not observed,” “requested,” “running,” “completed,” “stale,” and “unavailable” without deriving a green review result from the absence of comments.

Reuse Spagitty's connected GitHub account. Extend typed forge operations to read PR metadata including head/base SHAs, top-level discussion comments, inline review comments, review records, and check runs. Top-level PR discussion uses issue-comment endpoints, while inline findings use review-comment endpoints. Preserve actor IDs/types and commit attribution in the wire data. [GitHub discussion comments](https://docs.github.com/en/rest/issues/comments), [review comments](https://docs.github.com/en/rest/pulls/comments), [reviews](https://docs.github.com/en/rest/pulls/reviews), and [check runs](https://docs.github.com/en/rest/checks/runs).

The current inline comment model drops some attribution fields and defaults `resolved` to false. Extend it or introduce a dedicated snapshot model. Do not claim a thread is unresolved merely because resolution data was never fetched; obtain it through a documented supported API or display an unknown state.

Identify CodeRabbit by verified platform bot/app identity where available, not display name alone or a mention in a human comment. Handle configured enterprise/service-account identities explicitly. Treat provider-authored content as untrusted display data, including when it contains agent instructions.

Offer two explicit write actions: request an incremental review and request a full review. Preview the exact PR and comment body, then send through the existing HTTP boundary. CodeRabbit documents `@coderabbitai review` as incremental and `@coderabbitai full review` as a full pass; installations with another handle require that account's handle. These actions can consume the user's review allowance. [CodeRabbit PR review commands](https://docs.coderabbit.ai/reference/review-commands).

Add a typed top-level PR comment function: the existing inline reply operation is not an interchangeable substitute. Scope credentials to the connected host and require the applicable write permission. If a POST times out after possible delivery, report uncertainty and refresh before offering a deliberate resend; do not duplicate a request with an automatic retry.

Refresh on explicit action and existing PR refresh signals. After a requested review, a bounded background wait may use conditional requests/backoff while that PR is active, with cancellation on navigation or repository close. Do not introduce permanent global polling or require a webhook server.

Track the requested head revision and associate review/check results with the revision they actually cover. A PR push makes prior results stale. A successful comment POST means “requested,” not “review completed.” A passed check is provider status, not an assertion that every finding is resolved. Keep local CLI and PR reviews separately attributed.

For sources that cannot prove exact revision coverage or completeness, display the result but do not reuse it as committed farm gate evidence. GitHub's existing branch protections remain authoritative for remote merging; the extension must not change them or add automatic merge/approve/resolve/autofix actions in v1.

## 12. Product experience

Add an Extensions settings section with installed extensions, official/development/local provenance, version, enablement scope, compatibility, requested/granted capabilities, configuration, diagnostics, update-from-file, and removal. The bundled CodeRabbit card should have a short setup path: locate tool, verify version/login, choose region, explain data sharing, enable for this repository.

Use these primary entry points:

- Changes: “Review changes with CodeRabbit.”
- Farm task: “Review task with CodeRabbit” and a review result beside existing verification/review evidence.
- PR workspace: CodeRabbit panel and “Request CodeRabbit review.”
- Command palette: the same actions with availability reasons.
- Findings: jump to file/diff when valid, acknowledge/dismiss locally, select, and send to agent.

Do not expose protocol names or raw manifest fields in the normal user journey. Provide diagnostic details behind an explicit action. Existing user work stays usable if CodeRabbit is missing or unavailable. Required farm policies should explain why that task is waiting without freezing the entire repository.

Keyboard access, visible focus, loading/cancellation states, light/dark contrast, empty states, and long finding text must be covered by the UI tests and manual sweep. Preserve the established Spagitty visual system and approved brand wording.

## 13. Files and deliverables

The exact filenames may be adjusted to current conventions, but all these responsibilities must be delivered:

```text
crates/spagitty-extensions/
  src/{manifest,package,registry,protocol,host,capabilities,review,storage}.rs
  tests/                     Real lifecycle, package, and callback tests
crates/spagitty-process/      Shared process-tree primitives, if extracted
extensions/coderabbit/
  extension.json             Official package manifest
  worker/                    Native protocol worker and CodeRabbit adapter
  fixtures/                  Redacted provider streams and PR snapshots
  README.md                  Setup, scopes, limits, privacy, troubleshooting
packages/extension-sdk/
  src/                       Types, transport, handlers, fake host
  templates/                 Working scaffold
tools/extensions/            Create, validate, test, pack, inspect tooling
examples/extensions/hello/    Independent public-API example
schemas/extensions/          Manifest and protocol schemas/fixtures
src-tauri/src/extensions.rs  Thin desktop bridge/composition
src/lib/extensions/          Store, settings, generic panels and tests
docs/extensions/             Developer and user guides
```

Also update `Cargo.toml` workspace membership, package scripts/lockfiles only as needed, Tauri registration and bundled resources, the PR/core wire models, farm supplemental-review interface/persistence, relevant screens, architecture/privacy/testing docs, CI scope and release packaging, licenses, and the changelog under the repository's rules.

A new `extensions/` tree and SDK path must be included in CI's shipping-change classification; otherwise an extension change could skip application release packaging. Build/package workers before Tauri bundles them. Locate bundled workers through the resource API on every target, not the checkout's working directory. Match the **actual** release matrix: this checkout's gate currently enables Linux and Windows and comments out macOS lanes, despite broader support claims elsewhere. Validate supported targets and clearly record any release-lane limitation.

Prefer existing dependencies. Likely additions are a standards-compliant version-range parser and a safe ZIP reader/writer if the existing dependency surface has no suitable public API. For each addition, name its purpose, alternatives, version, license, compatibility, and generated notice impact in the handoff. Do not introduce another HTTP client or async runtime merely for this feature. Internal workspace crates are architectural modules, not a reason to duplicate dependencies.

The official extension's source must follow Spagitty's license/header rules. CodeRabbit CLI is an external user-installed tool; do not bundle its proprietary executable, represent its service as free software, or assume redistribution permission. The extension package must explain this external requirement.

## 14. Implementation phases

### Phase 0 — Rules, records, and contract

Read the canonical rules, inspect the current tree, identify reusable work items, and allocate new IDs through `agile/` conventions. Record the design decisions and write the implementation/test records before starting the corresponding work. Do not invent or mark completed placeholder work-item IDs in this proposal.

Finalize manifest/schema, protocol methods, trust/capability wording, review types, versioning, and the dependency graph. Add wire fixtures and a fake extension that exercises bidirectional callbacks. This phase is done when two independent implementations can agree on the contract without sharing private host classes.

### Phase 1 — Host and public developer path

Implement supervised workers, registry, validation, grants, safe packaging, installation/settings, contribution registration, and SDK/tooling. Demonstrate the independently packaged hello extension and its removal. Test disconnects and cancellation before involving the external service.

### Phase 2 — CodeRabbit local reviews

Build the official worker through the same protocol. Implement detection, authentication inspection, region/setup, structured parsing, progress, findings, cancellation, history, and stale interactive results. Add captured fixtures and an opt-in real service smoke test. An unavailable external service must not prevent deterministic CI.

### Phase 3 — Farm reviews and repair

Add the supplemental provider interface, policy modes, committed evidence, merge-boundary checks, persistence, event bridge, and selected-finding handoff. Verify that a required CodeRabbit gate and the existing independent reviewer both remain in the completion path. Verify no changes reach the user's checkout through a repair task.

### Phase 4 — GitHub PR integration

Extend typed forge snapshots and the top-level comment operation. Implement identity attribution, PR panel, explicit request actions, refresh/backoff, revision staleness, and incomplete/unknown data handling. Keep raw credentials and provider-specific HTTP calls out of workers and the UI.

### Phase 5 — Distribution, documentation, and final review

Bundle the extension, validate imported packages, prove update/rollback/uninstall, run platform and accessibility sweeps, update docs/license notices/changelog/CI classification, and complete the review required by repository rules. Do not claim the feature done while developer tooling, PR behavior, or required farm-gate failure cases remain stubs.

## 15. Verification plan

Use real temporary Git repositories for snapshot and worktree behavior and deterministic fake processes for extension/protocol behavior. Provider fixtures must represent actual captured output, be redacted, and name the tested provider version. Never put a real token or private repository patch in fixtures.

| Area | Required evidence |
| --- | --- |
| Contract | Valid/invalid manifests, version mismatches, unsupported targets, duplicate identities, unknown capability rejection, Rust/TS wire compatibility. |
| Installation | Safe extraction across Unix/Windows path shapes, corrupt integrity data, expansion limits, staging failure, atomic switch, migration failure, rollback, safe removal. |
| Isolation/lifecycle | Worker crash, failed handshake, concurrent callbacks, disable mid-operation, whole-tree termination, no orphan contribution, bounded logs, late/duplicate result rejection. |
| Host grants | Every protected callback rejects missing/revoked grants, wrong repository handles, expired context, and unauthorized tool selection. |
| Provider adapter | Successful/non-empty/empty runs, incomplete analysis, failure after findings, unknown severity, malformed stream, progress silence, terminal conflicts, quota/payment action, old retained findings. |
| Snapshot identity | Changed base/head/index/content/configuration, external edits during review, missing/deleted/renamed paths, stale results after restart, invalidation after a repair. |
| Farm authority | Required gate enforced through every merge path; all unavailable provider states block; independent review and verification remain required under current policy; repairs honor autonomy/budget. |
| PR behavior | Discussion vs inline endpoints, identity attribution, pagination limits, unknown resolution, changed head, stale checks, uncertain comment delivery, unsupported forge, no duplicate requests. |
| UI | Configuration/review states, accessibility, generic panel rendering, injection-resistant content, no credential in events, findings selection/handoff, long text, command disposal. |
| Developer standard | Scaffold → harness → package → install → execute → disable/remove on an unchanged host; no CodeRabbit special case needed for hello. |
| Distribution | Worker location/executable bits/signing behavior, runtime independence, bundle contents, actual release matrix, CI classifies extension/SDK changes correctly. |

Run the repository's current required checks, including the following where available:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun run check
bun run test
bun run coverage
cargo llvm-cov --workspace --fail-under-lines 70
bun run build
bun run tauri build
```

Use the coverage flags/exclusions specified by the current CI configuration and maintain its existing floors: Rust 70% lines and frontend 65% branches in the inspected documentation. Also run required license, security, and brand checks when affected. New SDK/worker packages need to be included deliberately in test/coverage/build scope rather than being silently missed by the root scripts.

Run manual end-to-end checks with the application: hello extension import/removal; CodeRabbit sign-in and review; cancellation; a farm repair cycle; required-gate blocking; GitHub request and refresh; navigation/repository switching during a long run; disable/update while work exists. Real service tests are explicit and account-dependent, separate from deterministic CI. An unavailable credential, service, platform, or prerequisite must be recorded as unverified, not passed.

## 16. Acceptance criteria

The implementation is complete only when all of the following are demonstrated and the repository's definition of done is met:

1. A developer can produce and import a working extension using only the public schema/SDK/tooling without modifying Spagitty.
2. Users can inspect, install, enable per repository, configure, disable, update with rollback, and remove an extension.
3. Incompatible, malformed, or unsafe packages are rejected before execution; the documented native trust model matches the implementation.
4. Commands, menus, settings, panels, and review providers are generic and disappear cleanly on disable/removal.
5. CodeRabbit is a normal official extension package, usable without Claude Code and disabled until setup/consent.
6. Local review produces attributable findings, progress, cancellation, and history, with explicit handling of stale, incomplete, failed, and action-required outcomes.
7. The application remains responsive and usable when a worker or CodeRabbit process crashes or cannot run.
8. CodeRabbit can review committed farm task changes with exact evidence, and Required policy is enforced at every merge entry point.
9. Existing verification, independent reviewer rules, autonomy, and isolated-worktree guarantees still hold.
10. Selected findings reach the existing agent repair flow, remain identifiable, and require fresh verification/review after changes.
11. GitHub PR results display with accurate attribution/revision state, and explicit incremental/full review requests use the existing authenticated network boundary.
12. CodeRabbit credentials remain CLI-owned, forge tokens remain backend-owned, and normal logs/settings/UI events contain neither.
13. The independently packaged hello extension proves that third-party contributions require no host source change.
14. Bundled workers and tooling are built/tested on every claimed target, with actual CI/release limitations identified honestly.
15. Relevant automated/manual checks, test coverage, production builds, license notices, documentation, agile records, changelog, and required review are complete.

## 17. Final handoff Claude Code must provide

Lead with the resulting user and developer workflows. Link the work items and implementation records, list significant files, explain architecture decisions and new dependencies, and report exact commands/results for verification and builds. Include supported extension API/manifest versions, tested CodeRabbit versions/platforms, demonstrated farm and PR scenarios, and any remaining unverified service checks.

Describe any deviations from this proposal and their practical effects. Distinguish shipped behavior from deferred marketplace/sandbox/provider expansion. Do not claim a successful live review, supported platform, passed build, or completed independent review without evidence. Do not publish a package, tag a release, merge protected branches, or change external account configuration beyond the authorization and repository rules in effect.
