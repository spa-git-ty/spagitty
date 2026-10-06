// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Semantic versions and the manifest's version requirements.
 *
 * The requirement dialect is Cargo's, which is what the host reads:
 * comma-separated comparators (`=`, `>`, `>=`, `<`, `<=`, `~`, `^`, or a bare
 * version meaning `^`), plus the host's one leniency — whitespace between two
 * comparators means the same as a comma (`>=0.9.0 <1.0.0`). Partial versions
 * (`1`, `1.2`) and `*` wildcards follow Cargo's rules. Pre-releases only match
 * a comparator that names a pre-release of the same version.
 */

export interface Version {
	major: number;
	minor: number;
	patch: number;
	pre: string[];
}

const NUM = '(0|[1-9]\\d*)';
const IDENT = '[0-9A-Za-z-]+';
const SEMVER = new RegExp(`^${NUM}\\.${NUM}\\.${NUM}(?:-(${IDENT}(?:\\.${IDENT})*))?(?:\\+(${IDENT}(?:\\.${IDENT})*))?$`);

export function parseVersion(text: string): Version | null {
	const match = SEMVER.exec(text.trim());
	if (!match) return null;
	return { major: +match[1], minor: +match[2], patch: +match[3], pre: match[4] ? match[4].split('.') : [] };
}

function comparePre(a: string[], b: string[]): number {
	if (!a.length && !b.length) return 0;
	if (!a.length) return 1;
	if (!b.length) return -1;
	for (let i = 0; i < Math.max(a.length, b.length); i++) {
		if (a[i] === undefined) return -1;
		if (b[i] === undefined) return 1;
		const an = /^\d+$/.test(a[i]);
		const bn = /^\d+$/.test(b[i]);
		if (an && bn) {
			const d = +a[i] - +b[i];
			if (d) return Math.sign(d);
		} else if (an !== bn) {
			return an ? -1 : 1;
		} else if (a[i] !== b[i]) {
			return a[i] < b[i] ? -1 : 1;
		}
	}
	return 0;
}

export function compare(a: Version, b: Version): number {
	return (
		Math.sign(a.major - b.major) || Math.sign(a.minor - b.minor) || Math.sign(a.patch - b.patch) || comparePre(a.pre, b.pre)
	);
}

type Op = '=' | '>' | '>=' | '<' | '<=' | '~' | '^';

interface Comparator {
	op: Op;
	major: number;
	minor: number | null;
	patch: number | null;
	pre: string[];
}

const COMPARATOR = /^(=|>=|<=|>|<|~|\^)?\s*(\d+|\*)(?:\.(\d+|\*))?(?:\.(\d+|\*))?(?:-([0-9A-Za-z.-]+))?$/;

function parseComparator(text: string): Comparator | null {
	const match = COMPARATOR.exec(text.trim());
	if (!match) return null;
	const op = (match[1] ?? '^') as Op;
	const part = (s: string | undefined) => (s === undefined || s === '*' ? null : +s);
	if (match[2] === '*') return { op: '>=', major: 0, minor: 0, patch: 0, pre: [] };
	const minor = part(match[3]);
	const patch = minor === null ? null : part(match[4]);
	return { op, major: +match[2], minor, patch, pre: match[5] ? match[5].split('.') : [] };
}

/** Split into comparators the way the host does: on commas, or between comparators. */
function split(range: string): string[] {
	const text = range.trim();
	if (text.includes(',')) return text.split(',');
	return text.match(/(?:=|>=|<=|>|<|~|\^)?\s*[^\s<>=~^,]+/g) ?? [];
}

export function parseRange(range: string): Comparator[] | null {
	if (!range.trim() || range.length > 100) return null;
	const parts = split(range).map(parseComparator);
	return parts.length && parts.every((p): p is Comparator => p !== null) ? parts : null;
}

function matchesOne(c: Comparator, v: Version): boolean {
	const lower = (major: number, minor: number, patch: number, pre: string[] = []) =>
		compare(v, { major, minor, patch, pre }) >= 0;
	const below = (major: number, minor: number, patch: number) => compare(v, { major, minor, patch, pre: ['0'] }) < 0;
	const minor = c.minor ?? 0;
	const patch = c.patch ?? 0;
	const exact = { major: c.major, minor, patch, pre: c.pre };
	switch (c.op) {
		case '=':
			if (c.minor === null) return v.major === c.major;
			if (c.patch === null) return v.major === c.major && v.minor === c.minor;
			return compare(v, exact) === 0;
		case '>':
			if (c.minor === null) return v.major > c.major;
			if (c.patch === null) return v.major > c.major || (v.major === c.major && v.minor > minor);
			return compare(v, exact) > 0;
		case '>=':
			return compare(v, exact) >= 0;
		case '<':
			return compare(v, exact) < 0;
		case '<=':
			if (c.minor === null) return v.major <= c.major;
			if (c.patch === null) return v.major < c.major || (v.major === c.major && v.minor <= minor);
			return compare(v, exact) <= 0;
		case '~':
			if (c.minor === null) return lower(c.major, 0, 0) && below(c.major + 1, 0, 0);
			return lower(c.major, minor, patch, c.pre) && below(c.major, minor + 1, 0);
		case '^':
			if (c.major > 0 || c.minor === null) return lower(c.major, minor, patch, c.pre) && below(c.major + 1, 0, 0);
			if (minor > 0 || c.patch === null) return lower(0, minor, patch, c.pre) && below(0, minor + 1, 0);
			return lower(0, 0, patch, c.pre) && below(0, 0, patch + 1);
	}
}

/** Whether `version` satisfies `range`, with the host's pre-release rule. */
export function satisfies(range: string, version: string): boolean {
	const comparators = parseRange(range);
	const v = parseVersion(version);
	if (!comparators || !v) return false;
	if (!comparators.every((c) => matchesOne(c, v))) return false;
	if (!v.pre.length) return true;
	return comparators.some(
		(c) => c.pre.length > 0 && c.major === v.major && (c.minor ?? 0) === v.minor && (c.patch ?? 0) === v.patch
	);
}
