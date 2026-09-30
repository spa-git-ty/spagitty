// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The sentences the Settings screen says about a value.
 *
 * Kept apart from the markup because they are the part that can be wrong: a
 * screen that says "global" about a value coming from `/etc/gitconfig` sends
 * someone to edit a file that will not change anything.
 */

import type {
	Dependency,
	IdentityOrigin,
	IdentityScope,
	IdentityValue,
	SigningFormat,
	SigningProblem
} from '../types';

/** Where the value git would use is coming from, in a sentence. */
export function describeOrigin(origin: IdentityOrigin): string {
	switch (origin) {
		case 'unset':
			return 'Not set anywhere. Git refuses to commit without it.';
		case 'system':
			return 'From this machine’s system configuration, which Spagitty does not write.';
		case 'global':
			return 'From your global configuration.';
		case 'local':
			return 'From this repository.';
		case 'environment':
			return 'From the environment Spagitty was started in, which Spagitty does not write.';
	}
}

/**
 * The warning shown when the scope being edited is not the one in effect.
 *
 * Editing a field that something else overrides looks like it worked and
 * changes nothing, which is the confusion this whole screen is shaped around.
 * Null when there is nothing to warn about.
 */
export function describeOverride(value: IdentityValue, scope: IdentityScope): string | null {
	if (value.origin === 'unset') return null;

	if (scope === 'global' && value.origin === 'local') {
		return 'This repository sets its own, so editing the global value will not change what it commits with.';
	}
	if (value.origin === 'system' || value.origin === 'environment') {
		return 'Something outside both of these files is winning, so a change here will not take effect.';
	}
	return null;
}

/** A package's terms, naming the gap rather than hiding it. */
export function describeLicense(dependency: Dependency): string {
	return dependency.license ?? 'not declared';
}

/** How many of a list declare no license at all. */
export function undeclared(dependencies: Dependency[]): number {
	return dependencies.filter((dependency) => dependency.license === null).length;
}

/**
 * The dependencies matching a filter, by name or by license.
 *
 * Both, because the two questions asked of this list are "is X in here" and
 * "what is under license Y" — the second being the one a license audit asks.
 * An empty filter is every dependency rather than none.
 */
export function matching(dependencies: Dependency[], query: string): Dependency[] {
	const needle = query.trim().toLowerCase();
	if (needle === '') return dependencies;

	return dependencies.filter(
		(dependency) =>
			dependency.name.toLowerCase().includes(needle) ||
			describeLicense(dependency).toLowerCase().includes(needle)
	);
}

/**
 * What a signing problem means, in a sentence a reader can act on.
 *
 * Beside the identity's sentences for the same reason: this is the part that
 * can be wrong, and "signing is on but nothing can sign" is exactly the claim
 * that has to be right before a commit is attempted rather than after one
 * fails.
 */
export function describeSigningProblem(problem: SigningProblem): string {
	switch (problem.kind) {
		case 'missingProgram':
			return `${problem.detail} could not be run, so signed commits will fail.`;
		case 'noSigningKey':
			return 'Signing with SSH needs user.signingkey.';
	}
}

/** Which machinery git would sign with, by the name people know it by. */
export function describeSigningFormat(format: SigningFormat): string {
	switch (format) {
		case 'openPgp':
			return 'GPG';
		case 'ssh':
			return 'SSH';
		case 'x509':
			return 'S/MIME';
	}
}
