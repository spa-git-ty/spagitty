// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Refuse agent, tool and session attribution in commit messages and pull
 * request descriptions (TASK-064).
 *
 * `AGENTS.md` forbids it: no `Co-Authored-By:` naming an agent, no
 * `Claude-Session:`, no "Generated with" line. A person can still be a
 * co-author, so a trailer is refused only when what it names is an agent, a
 * tool or a session; and Spagitty's own `Co-authored-by` trailer for a merge an
 * agent resolved is written into the user's commits by the application, which
 * never passes through here.
 *
 * Claude, Devin and Cody are people's names too (BUG-078). A co-author is an
 * agent when the name is one only a product has, when a model or "AI" follows
 * a name a person could have, or when the address is one only an agent writes
 * from. "Claude Dupont" is a person; "Claude Opus 5" is not.
 *
 * Usage:
 *   node tools/attribution.mjs commits <range>   every message in `git log <range>`
 *   node tools/attribution.mjs text <file|->      a pull request description
 */

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

/** Names only a product has: nobody is called these. */
const PRODUCT =
	/^(?:(?:openai\s+)?codex|chatgpt|gpt-?\d\S*|(?:google\s+)?gemini|(?:github\s+)?copilot|cursor|aider|windsurf|coderabbit|opencode|agy|antigravity|anthropic|openai)\b/i;

/** A name a person can have, which is a product once a model, "Code" or "AI" follows it. */
const PRODUCT_BY_WHAT_FOLLOWS =
	/^(?:claude\s+(?:code|opus|sonnet|haiku|fable|instant|\d)|devin\s+ai|cody\s+ai|sourcegraph\s+cody)\b/i;

/** Addresses only an agent or a tool writes from. */
const AGENT_ADDRESS =
	/@(?:[\w-]+\.)*(?:anthropic\.com|openai\.com|cursor\.com|aider\.chat|cognition\.ai|codeium\.com|windsurf\.com|coderabbit\.ai)$|\[bot\]/i;

/** Whether a `Co-Authored-By:` value, `Name <address>`, names an agent or a tool. */
function isAgent(coAuthor) {
	const [, name = '', address = ''] = /^(.*?)\s*(?:<([^>]*)>?)?\s*$/.exec(coAuthor) ?? [];
	return PRODUCT.test(name) || PRODUCT_BY_WHAT_FOLLOWS.test(name) || AGENT_ADDRESS.test(address.trim());
}

/** What a "Generated with" line names when it is attribution. */
const TOOL =
	'claude|anthropic|codex|openai|chatgpt|gpt-?\\d\\w*|gemini|copilot|cursor|aider|devin|windsurf|cody|coderabbit|opencode|agy|antigravity|an?\\s+ai';

/** Lines that are attribution whoever they name. */
const ALWAYS = [
	/^\s*claude-session\s*:/i,
	new RegExp(`(?<![a-z])generated\\s+(?:with|by)\\s+\\[?(?:${TOOL})\\b`, 'i'),
	/(?<![a-z])created\s+(?:with|by)\s+\[?claude\s+code/i
];

/**
 * The lines of `text` that are agent, tool or session attribution, each as it
 * was written. A `Co-Authored-By:` line counts when it names an agent or a
 * tool, or carries an address only one writes from.
 */
export function findAttribution(text) {
	const found = [];
	for (const line of text.split(/\r?\n/)) {
		if (ALWAYS.some((pattern) => pattern.test(line))) {
			found.push(line.trim());
			continue;
		}
		const trailer = /^\s*co-authored-by\s*:\s*(.*)$/i.exec(line);
		if (trailer && isAgent(trailer[1])) {
			found.push(line.trim());
		}
	}
	return found;
}

function main(argv) {
	const [mode, target] = argv;
	let text;
	if (mode === 'commits' && target) {
		text = execFileSync('git', ['log', '--format=%B', target], { encoding: 'utf8' });
	} else if (mode === 'text' && target) {
		text = readFileSync(target === '-' ? 0 : target, 'utf8');
	} else {
		console.error('usage: attribution.mjs commits <range> | text <file|->');
		return 2;
	}

	const found = findAttribution(text);
	if (found.length === 0) return 0;
	console.error('Agent, tool or session attribution is not allowed (AGENTS.md):');
	for (const line of found) console.error(`  ${line}`);
	return 1;
}

// Run only as a CLI; importing the module for its function runs nothing.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	process.exit(main(process.argv.slice(2)));
}
