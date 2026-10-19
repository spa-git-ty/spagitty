// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Refuse a dependency that needs a newer Rust than the workspace says it
 * builds with (TASK-072).
 *
 * `Cargo.toml` declares a `rust-version`. Cargo compares a dependency's own
 * `rust-version` with the toolchain in use, not with the workspace's, and
 * every gate builds with `stable`, so the line could fall behind without
 * anything failing: it said 1.77 for months after Files' highlighter needed
 * 1.90. Clippy reads it too, to decide which newer calls to suggest.
 *
 * Usage:
 *   node tools/rust-version.mjs     reads `cargo metadata` and exits 1 on any
 */

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

/** `1.90`, `1.89.0` → comparable numbers. */
function parts(version) {
	return version.split('.').map((part) => Number.parseInt(part, 10) || 0);
}

/** Is `a` a newer Rust than `b`? */
export function newer(a, b) {
	const [x, y] = [parts(a), parts(b)];
	for (let index = 0; index < Math.max(x.length, y.length); index++) {
		const difference = (x[index] ?? 0) - (y[index] ?? 0);
		if (difference !== 0) return difference > 0;
	}
	return false;
}

/**
 * The packages of a `cargo metadata` document that are not the workspace's
 * own and declare a newer `rust-version` than `declared`, each as
 * `name version needs rust`.
 */
export function aheadOf(metadata, declared) {
	const own = new Set(metadata.workspace_members);
	return metadata.packages
		.filter((pkg) => !own.has(pkg.id) && pkg.rust_version && newer(pkg.rust_version, declared))
		.map((pkg) => `${pkg.name} ${pkg.version} needs ${pkg.rust_version}`)
		.sort();
}

/** The workspace's own line, from the root `Cargo.toml`. */
export function declaredIn(manifest) {
	return /^\s*rust-version\s*=\s*"([^"]+)"/m.exec(manifest)?.[1] ?? null;
}

function main() {
	const declared = declaredIn(readFileSync('Cargo.toml', 'utf8'));
	if (!declared) {
		console.error('Cargo.toml declares no rust-version');
		return 1;
	}
	const metadata = JSON.parse(
		execFileSync('cargo', ['metadata', '--format-version', '1', '--locked'], {
			encoding: 'utf8',
			maxBuffer: 256 * 1024 * 1024
		})
	);
	const ahead = aheadOf(metadata, declared);
	if (ahead.length === 0) return 0;
	console.error(`Cargo.toml says rust-version ${declared}, and these need more:`);
	for (const line of ahead) console.error(`  ${line}`);
	return 1;
}

// Run only as a CLI; importing the module for its functions runs nothing.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	process.exit(main());
}
