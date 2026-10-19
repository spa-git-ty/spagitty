// SPDX-License-Identifier: GPL-3.0-or-later

import type { NetworkOperation } from '../types';

/**
 * git's last line about a fetch or a push, in words (BUG-091).
 *
 * The line is git's ref-update table row, `75b242b47..880b35657  topic ->
 * origin/topic`, and it was shown as it came, beside the branch, until the
 * next fetch or push replaced it. Two abbreviated hashes and an arrow say
 * nothing to somebody who only pressed Push. What they say is which branch
 * moved and how, and that is what this keeps. A line it does not recognise is
 * passed on as git wrote it rather than guessed at.
 */
export function plainSummary(operation: NetworkOperation, line: string | null): string | null {
	if (line === null) return null;
	const text = line.trim();
	if (text === '') return null;
	if (/^Everything up-to-date$/i.test(text) || /\[up to date\]/.test(text)) return 'Already up to date';

	// ` + a...b  src -> dst (forced update)`, `   a..b  src -> dst`, ` * [new branch]  src -> dst`
	const row = /^([ +*\-!=t])?\s*(\[[^\]]+\]|[0-9a-f]{4,}\.{2,3}[0-9a-f]{4,})\s+(\S+)\s+->\s+(\S+)(?:\s+\((.+)\))?$/.exec(
		text
	);
	if (!row) return text;
	const [, flag, what, from, to, note] = row;
	const name = operation === 'push' ? from : to;
	const verb = operation === 'push' ? 'Pushed' : 'Fetched';

	if (flag === '!' || what === '[rejected]') return `${name} was refused${note ? `: ${note}` : ''}`;
	if (what === '[new branch]') return `${verb} new branch ${name}`;
	if (what === '[new tag]') return `${verb} new tag ${name}`;
	if (what === '[deleted]') return `Deleted ${to}`;
	if (flag === '+' || note === 'forced update') return `${verb} ${name}, replacing what was there`;
	if (what.startsWith('[')) return `${verb} ${name}`;
	return operation === 'push' ? `Pushed ${name}` : `Fetched: ${name} moved on`;
}
