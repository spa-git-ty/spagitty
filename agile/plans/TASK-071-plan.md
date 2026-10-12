<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-071 — Plan

**Item:** [TASK-071](../items/TASK-071-remove-agy.md)

Take `Agy` out of `AgentProvider` and let the compiler name what depended on it: the adapter, `adapter_for`, the label and slug, the list in `workspace/branch.rs`, the read-only flags and the launch-failure check in `assign/local.rs`. Then the saved choice and its command in `src-tauri`, and the provider, chips and API call in the frontend, where `Record<AgentProvider, …>` does the same job.

One thing does not fall out of that. A provider that is no longer in the set cannot be read back from a file, and `store::load_registry` gives an empty registry for a file it cannot read. So `AgentRegistry` reads its definitions as values first and keeps the ones that parse. `agents.json` needs nothing: `Machine` ignores a key it does not know.

Tests that only borrowed agy keep what they were testing and borrow something else. The two that tested agy itself go with it.

## What decides it is right

- A search for `agy` in `src/`, `src-tauri/src/` and `crates/` finds only `readable` and its test.
- A registry with an agy definition deserialises to the other definitions.
- The Rust and frontend suites pass, `bun run check` is clean and the build succeeds.
