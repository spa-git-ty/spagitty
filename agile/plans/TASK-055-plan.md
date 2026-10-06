<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-055 — implementation plan

Implemented together on the existing `feature/FEAT-097-coderabbit-reviews-local-changes` checkout to preserve Claude’s work.

Build the selected native worker before Tauri bundles resources; narrow the staged
manifest to that target. Clean only private generated staging with checked paths.
Use resource discovery on Windows/Linux and the existing bundled-only sidecar
fallback on macOS; let Tauri import/sign with its normal keychain. Classify extension,
SDK, tooling and schema edits as shipping. Test hello's full package/import/remove
path and native worker lifecycle. Run type/lint/tests/coverage and production builds.
Record actual platform, live service and signing limitations without invented evidence.
