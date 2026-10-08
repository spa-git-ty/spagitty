// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Spagitty's icon set.
 *
 * Every icon in the application was a Unicode glyph — `⇩` for fetch, `⑃` for
 * branch, `▤` for stash — picked because the application shipped no icon set.
 * They are a font's idea of those shapes rather than this application's: the
 * weights disagree with each other, half of them fall back to a different face
 * depending on what is installed, and none of them line up on a baseline with
 * the others. That is the single thing that made the interface look drawn
 * rather than designed.
 *
 * These are paths instead, on a 24×24 grid, stroked in `currentColor` at a
 * uniform weight. No icon font, no network, no build step — the set is data,
 * the same way the palettes are, and `Icon.svelte` is the only thing that knows
 * how to draw one.
 *
 * Drawing rules, so a new icon does not look like a visitor:
 *
 * - 24×24 box, and the shape lives inside a 20×20 area — the two-unit margin is
 *   what keeps a circle and a square looking the same size;
 * - one stroke weight throughout, set by the component, never baked into a path;
 * - round caps and joins;
 * - no fills. A filled icon beside stroked ones reads as selected even when it
 *   is not.
 */

export type IconName =
	| 'undo'
	| 'redo'
	| 'pull'
	| 'fetch'
	| 'push'
	| 'clone'
	| 'branch'
	| 'stash'
	| 'rebase'
	| 'graph'
	| 'edit'
	| 'conflict'
	| 'merge'
	| 'swap'
	| 'base'
	| 'tag'
	| 'request'
	| 'search'
	| 'history'
	| 'folder'
	| 'settings'
	| 'plus'
	| 'close'
	| 'minimize'
	| 'maximize'
	| 'chevron-left'
	| 'chevron-right'
	| 'chevron-up'
	| 'chevron-down'
	| 'terminal'
	| 'check'
	| 'circle'
	| 'circle-check'
	| 'refresh'
	| 'badge'
	| 'farm'
	| 'review'
	| 'ruler'
	| 'unfold'
	| 'pause'
	| 'play'
	| 'warning'
	| 'agent'
	| 'cloud'
	| 'machine'
	| 'stop'
	| 'lock'
	| 'upload'
	| 'person'
	| 'arrow-right';

/**
 * The paths, keyed by name. A value is one or more `d` attributes, drawn in
 * order — a circle-plus-line icon is two entries rather than one clever path,
 * because a path that doubles back on itself renders a visible seam at the
 * join under a round cap.
 */
export const ICONS: Record<IconName, string[]> = {
	// An arrow curving back on itself, anticlockwise.
	undo: ['M3 8h11a5.5 5.5 0 0 1 0 11H8', 'M7 4 3 8l4 4'],
	redo: ['M21 8H10a5.5 5.5 0 0 0 0 11h6', 'M17 4l4 4-4 4'],
	// Down into a tray: what a pull does to your working copy.
	pull: ['M12 3v12', 'M7.5 10.5 12 15l4.5-4.5', 'M4 20h16'],
	// Down into a box, but not out of it — a fetch touches nothing you can see.
	fetch: ['M12 3v9', 'M8.5 8.5 12 12l3.5-3.5', 'M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3'],
	push: ['M12 21V9', 'M7.5 13.5 12 9l4.5 4.5', 'M4 4h16'],
	// Two overlapping sheets.
	clone: [
		'M9 9a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-8a2 2 0 0 1-2-2z',
		'M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1'
	],
	// The git branch mark: a trunk, a node that leaves it, a node it rejoins.
	branch: [
		'M7 4v10a4 4 0 0 0 4 4h2a4 4 0 0 0 4-4V9',
		'M7 4.5a2.5 2.5 0 1 0 0-.02',
		'M17 8.5a2.5 2.5 0 1 0 0-.02',
		'M7 20.5a2.5 2.5 0 1 0 0-.02'
	],
	// A drawer, pulled out.
	stash: ['M3 8h18v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z', 'M3 8l2-4h14l2 4', 'M9.5 13h5'],
	// Commits lifted off one line and set down on another.
	rebase: [
		'M6 20V10a4 4 0 0 1 4-4h8',
		'M6 20.5a2.5 2.5 0 1 0 0-.02',
		'M6 5.5a2.5 2.5 0 1 0 0-.02',
		'M14 2.5 17.5 6 14 9.5'
	],
	// The commit graph itself: a lane with a node on it and a lane merging in.
	graph: [
		'M8 6v12',
		'M8 8.5a2.5 2.5 0 1 0 0-.02',
		'M8 18.5a2.5 2.5 0 1 0 0-.02',
		'M16 13.5a2.5 2.5 0 1 0 0-.02',
		'M16 11V9a3 3 0 0 0-3-3H9'
	],
	// A pencil: the working copy is the one place you are writing.
	edit: ['M4 20h4l10.5-10.5a2.83 2.83 0 0 0-4-4L4 16z', 'M14 6l4 4'],
	// Two arrows meeting head on.
	conflict: ['M4 9h8', 'M9 6 12 9l-3 3', 'M20 15h-8', 'M15 12l-3 3 3 3'],
	// Two lanes meeting and flowing on down: Merger (FEAT-100).
	merge: ['M6 3v3a6 6 0 0 0 6 6 6 6 0 0 0 6-6V3', 'M12 12v9', 'M9 18l3 3 3-3'],
	// Two arrows passing each other: swap two things' places.
	swap: ['M4 8h14', 'M15 5l3 3-3 3', 'M20 16H6', 'M9 13l-3 3 3 3'],
	// A peak on a ground line: what both sides started from (FEAT-102).
	base: ['M4 18h16', 'M7 14l5-8 5 8'],
	tag: [
		'M20.5 12.5 12.5 20.5a2 2 0 0 1-2.83 0l-6.17-6.17a2 2 0 0 1-.5-2L4 4l8.33-1a2 2 0 0 1 1.67.57l6.5 6.5a2 2 0 0 1 0 2.43z',
		'M8 8.01v-.01'
	],
	// A change proposed from one place to another.
	request: ['M7 5v14', 'M7 5.5a2.5 2.5 0 1 0 0-.02', 'M7 19.5a2.5 2.5 0 1 0 0-.02', 'M17 19V9a3 3 0 0 0-3-3h-3', 'M14 3l-3 3 3 3'],
	search: ['M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14z', 'M16.5 16.5 21 21'],
	// An open eye: the Review screen, where a change is read rather than
	// browsed (FEAT-087).
	review: [
		'M2.5 12s3.5-6.5 9.5-6.5 9.5 6.5 9.5 6.5-3.5 6.5-9.5 6.5S2.5 12 2.5 12z',
		'M12 14.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z'
	],
	// Two rails with a line between them: the focus ruler (FEAT-087).
	ruler: ['M4 9.5h16', 'M4 14.5h16'],
	// Two chevrons opening apart: unchanged lines folded away (FEAT-091).
	unfold: ['M6 9.5l6-5 6 5', 'M6 14.5l6 5 6-5'],
	// The Farm's pill (FEAT-109): pause and resume a farm.
	pause: ['M9 5.5v13', 'M15 5.5v13'],
	play: ['M8 5.5v13l10-6.5z'],
	// A stuck task: a triangle with a mark in it.
	warning: ['M12 4 21.5 20h-19z', 'M12 10v4.5', 'M12 17.5v.01'],
	// An agent (2.0): a hexagon, the mark its proposals and its origin carry.
	// Not a face, a robot or a spark: nothing in Spagitty is a model, and the
	// mark says *which* work is the agent's rather than what it is.
	agent: ['M12 3.5 19.4 7.75v8.5L12 20.5l-7.4-4.25v-8.5z'],
	// A model reached over an API.
	cloud: ['M7.5 18.5h9.5a4 4 0 0 0 .5-7.97A5.5 5.5 0 0 0 7 9.6a4.5 4.5 0 0 0 .5 8.9z'],
	// An endpoint on this machine.
	machine: ['M4 5h16v11H4z', 'M9 20h6', 'M12 16v4'],
	// Stop an agent's run.
	stop: ['M7 7h10v10H7z'],
	// Kept in the keychain.
	lock: ['M6.5 11h11v9h-11z', 'M8.5 11V8a3.5 3.5 0 0 1 7 0v3'],
	// What leaves the machine.
	upload: ['M12 15V4', 'M7.5 8.5 12 4l4.5 4.5', 'M5 15v4h14v-4'],
	// The person: *you do it*.
	person: ['M12 11.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z', 'M5 20a7 7 0 0 1 14 0'],
	// Send a line to the agent.
	'arrow-right': ['M5 12h14', 'M13 6l6 6-6 6'],
	// A clock turned back: where HEAD has been.
	history: ['M3.5 12a8.5 8.5 0 1 0 2.6-6.1', 'M3 4v4h4', 'M12 8v4.5l3 1.8'],
	folder: ['M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'],
	settings: [
		'M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z',
		'M19.4 14.5a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1.03 1.56V21a2 2 0 0 1-4 0v-.1a1.7 1.7 0 0 0-1.11-1.55 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.56-1.03H3a2 2 0 0 1 0-4h.1a1.7 1.7 0 0 0 1.55-1.11 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34H9a1.7 1.7 0 0 0 1-1.56V3a2 2 0 0 1 4 0v.1a1.7 1.7 0 0 0 1 1.56 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87V9a1.7 1.7 0 0 0 1.56 1H21a2 2 0 0 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1.1z'
	],
	plus: ['M12 5v14', 'M5 12h14'],
	close: ['M6 6l12 12', 'M18 6 6 18'],
	minimize: ['M5 12h14'],
	maximize: ['M5 5h14v14H5z'],
	'chevron-left': ['M14.5 6 8.5 12l6 6'],
	'chevron-right': ['M9.5 6l6 6-6 6'],
	// The same chevron, turned. A drawer opens upward and closes downward, so
	// the glyph points where the panel is going rather than where it is.
	'chevron-up': ['M6 14.5 12 8.5l6 6'],
	'chevron-down': ['M6 9.5l6 6 6-6'],
	terminal: ['M4 4h16v16H4z', 'M8 9.5 10.5 12 8 14.5', 'M13 15h3'],
	check: ['M5 12.5 10 17.5 19 7'],
	// A ring, and the ring ticked: a file still to read, and one read (TASK-053).
	circle: ['M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17z'],
	'circle-check': ['M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17z', 'M8.3 12.4l2.6 2.6 5-5.4'],
	refresh: ['M20 11a8 8 0 1 0-.5 4', 'M20 4v7h-7'],
	// A medal: a disc with a ribbon behind it. Two strokes for the ribbon
	// rather than one V, so the join under a round cap stays clean.
	badge: ['M12 15a5 5 0 1 0 0-10 5 5 0 0 0 0 10z', 'M8.5 14 6 21l6-2.6L18 21l-2.5-7'],
	// The farm: one node above, three below, connected. The same shape as a
	// branch point in the graph, which is what a farm is — several agents
	// working from one commit — and it reads as "many from one" at 16px, which
	// a barn or a tractor would not.
	farm: [
		'M12 3.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
		'M12 7.5v3.5',
		'M5 11h14',
		'M5 11v3',
		'M12 11v3',
		'M19 11v3',
		'M5 16.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
		'M12 16.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z',
		'M19 16.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z'
	]
};
