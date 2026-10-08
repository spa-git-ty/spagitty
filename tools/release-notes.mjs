// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Print one version's section of CHANGELOG.md, for a release's notes.
 *
 * Every tag carries notes, and the notes are that version's
 * changelog section — not notes generated from the commit log at release time.
 * Gate 6 runs this with the version it is about to tag, and the prerelease
 * workflow runs it with `Unreleased`; either way, an empty answer is a failure,
 * because a tag published without notes is an incomplete release.
 *
 * Usage: node tools/release-notes.mjs <version> [changelog-path]
 *   <version>  "0.1.0", "1.2.3-alpha.4", or the literal "Unreleased"
 */

import { readFileSync } from 'node:fs';

/**
 * The section for `version`, without its own heading, trimmed.
 *
 * A section runs from its `## ` heading to the next `## ` heading or the end
 * of the file. Headings are matched the way this changelog writes them —
 * `## [0.1.0] - 2026-08-28` or `## [Unreleased]` — and finding the version
 * anywhere else (prose, a link, a nested heading) is not finding the section.
 *
 * Returns null when the section is missing; '' when it exists but says
 * nothing. The caller decides that both are failures — kept apart here so the
 * error can say which.
 */
export function sectionFor(changelog, version) {
	const lines = changelog.split('\n');
	const opens = (line) => line.startsWith(`## [${version}]`);

	const start = lines.findIndex(opens);
	if (start === -1) return null;

	let end = lines.length;
	for (let i = start + 1; i < lines.length; i += 1) {
		if (lines[i].startsWith('## ')) {
			end = i;
			break;
		}
	}

	return lines.slice(start + 1, end).join('\n').trim();
}

function main(argv) {
	const [version, path = 'CHANGELOG.md'] = argv;
	if (!version) {
		process.stderr.write('usage: release-notes.mjs <version> [changelog-path]\n');
		return 2;
	}

	let changelog;
	try {
		changelog = readFileSync(path, 'utf8');
	} catch {
		process.stderr.write(`${path} is missing — every release needs it\n`);
		return 1;
	}

	const section = sectionFor(changelog, version);
	if (section === null) {
		process.stderr.write(`${path} has no "## [${version}]" section — write it before releasing\n`);
		return 1;
	}
	if (section === '') {
		process.stderr.write(`the "## [${version}]" section of ${path} is empty — a release without notes is incomplete\n`);
		return 1;
	}

	process.stdout.write(section + '\n');
	return 0;
}

// Run only as a CLI; importing the module for its function runs nothing.
if (import.meta.url === `file://${process.argv[1]}`) {
	process.exit(main(process.argv.slice(2)));
}
