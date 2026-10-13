// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What in a terminal's output might be a link (FEAT-130).
 *
 * Two things are worth opening from a terminal: a commit, which `git log`
 * prints as a hex word, and a place in a file, which a compiler prints as
 * `path:line` or `path:line:col`. Finding what *looks* like one is this
 * file's job and is only half of it: a word of seven hex letters is as often
 * `decade1` as a commit, and nothing is underlined until the repository has
 * said it is real. See `checked` in the store.
 */

/** A hex word that may be a commit, and where it is on the line. */
export interface WordCandidate {
	text: string;
	start: number;
	end: number;
}

/** A `path:line[:col]` that may be a file, and where it is on the line. */
export interface PathCandidate {
	/** The whole match, as printed. */
	text: string;
	path: string;
	line: number;
	col: number | null;
	start: number;
	end: number;
}

/** Seven to forty hex characters, standing as a word of their own. */
const HEX = /(?<![0-9A-Za-z_./\\-])[0-9a-f]{7,40}(?![0-9A-Za-z_])/g;

/**
 * A path, a colon, a line, and perhaps a colon and a column.
 *
 * The path is what a compiler prints: letters, digits and `._-+@` between
 * slashes, with at least one dot or slash in it so that `error:12` and a
 * clock's `10:41` are not paths. A drive letter is not taken as part of one:
 * what is checked is a path in the repository.
 */
const PATH = /(?<![\w./\\@-])((?:\.{1,2}\/)?[\w@+-][\w.@+-]*(?:\/[\w.@+-]+)*):(\d+)(?::(\d+))?(?![\w:])/g;

/** The hex words on a line that could be commits. A word that is all digits is a number. */
export function wordsIn(line: string): WordCandidate[] {
	const found: WordCandidate[] = [];
	for (const match of line.matchAll(HEX)) {
		if (/^\d+$/.test(match[0])) continue;
		found.push({ text: match[0], start: match.index, end: match.index + match[0].length });
	}
	return found;
}

/** The `path:line[:col]` places on a line. */
export function pathsIn(line: string): PathCandidate[] {
	const found: PathCandidate[] = [];
	for (const match of line.matchAll(PATH)) {
		const path = match[1];
		// One word with no dot and no slash is a label, not a file.
		if (!path.includes('/') && !path.includes('.')) continue;
		found.push({
			text: match[0],
			path: path.replace(/^\.\//, ''),
			line: Number(match[2]),
			col: match[3] === undefined ? null : Number(match[3]),
			start: match.index,
			end: match.index + match[0].length
		});
	}
	return found;
}

/** A link that has been checked, and what opening it does. */
export type Link =
	| { kind: 'commit'; start: number; end: number; text: string; commit: string }
	| { kind: 'file'; start: number; end: number; text: string; path: string; line: number };

/** What the repository said is real: hex words by the commit they mean, and paths that are files. */
export interface Known {
	commits: ReadonlyMap<string, string>;
	paths: ReadonlySet<string>;
}

/**
 * The links on a line: the candidates the repository confirmed, and no others.
 * A word inside a path that is a link is the path's, not a commit's.
 */
export function linksIn(line: string, known: Known): Link[] {
	const files: Link[] = pathsIn(line)
		.filter((candidate) => known.paths.has(candidate.path))
		.map((candidate) => ({
			kind: 'file',
			start: candidate.start,
			end: candidate.end,
			text: candidate.text,
			path: candidate.path,
			line: candidate.line
		}));
	const commits: Link[] = wordsIn(line)
		.filter((candidate) => known.commits.has(candidate.text))
		.filter((candidate) => !files.some((file) => candidate.start < file.end && candidate.end > file.start))
		.map((candidate) => ({
			kind: 'commit',
			start: candidate.start,
			end: candidate.end,
			text: candidate.text,
			commit: known.commits.get(candidate.text) as string
		}));
	return [...files, ...commits].sort((a, b) => a.start - b.start);
}
