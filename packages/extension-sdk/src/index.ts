// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Spagitty extension SDK (FEAT-096).
 *
 * - `defineExtension` / `run` — write a worker as handlers.
 * - `Host` — the host's services, typed.
 * - `validate` / `parseManifest` — the host's manifest rules.
 * - `satisfies` — the host's version requirement rules.
 * - `@spagitty/extension-sdk/testing` — a fake host for tests.
 * - `@spagitty/extension-sdk/package` — pack and inspect packages.
 */

export * from './protocol';
export { defineExtension, run, serve, streamChannel, Host, Peer, ExtensionError, HostError } from './worker';
export type { Channel, CommandContext, CommandResult, Definition, OperationContext, ReviewContext, RepositoryDescription } from './worker';
export { validate, parseManifest, ManifestError, TARGETS, CAPABILITIES, isExtensionId, isLocalId, isPackagePath } from './manifest';
export type { Manifest, Target, ToolOption } from './manifest';
export { satisfies, parseVersion, parseRange, compare } from './version';
