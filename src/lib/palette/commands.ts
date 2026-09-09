// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The commands the shell itself contributes to the palette.
 *
 * These are the ones that belong to no single screen: navigation, the view
 * controls on the graph, appearance, and the two repository-wide git verbs.
 * Anything that needs a selected commit registers itself from the screen that
 * owns that selection instead, so this file never has to import a feature in
 * order to name it.
 *
 * Called once, from the layout's `onMount`. Registration is idempotent — the
 * registry is keyed by id — so a hot reload that runs it twice is harmless.
 */

import { goto } from '$app/navigation';

import { shortcut } from '$lib/platform';

import { commandLog } from '$lib/commandlog/store.svelte';
import { fetchAll, pushCurrent } from '$lib/graph/actions';
import { columns, type ColumnId } from '$lib/graph/columns.svelte';
import { visibility } from '$lib/graph/visibility.svelte';
import { palette, type Command } from '$lib/palette/store.svelte';
import { repo } from '$lib/repo.svelte';
import { scale } from '$lib/scale.svelte';
import { settings } from '$lib/settings/store.svelte';
import { theme } from '$lib/theme.svelte';
import { worktrees } from '$lib/worktrees/store.svelte';
import { worktreeModal } from '$lib/worktrees/modal.svelte';
import { submodules } from '$lib/submodules/store.svelte';
import { submoduleModal } from '$lib/submodules/modal.svelte';


/** True when a repository is open. Most commands are meaningless without one. */
function hasRepo(): boolean {
	return repo.info !== null;
}

const NO_REPO = 'No repository open';

/** A command that needs an open repository, with the reason already attached. */
function repoCommand(command: Omit<Command, 'enabled' | 'unavailable'>): Command {
	return {
		...command,
		enabled: hasRepo,
		unavailable: () => (hasRepo() ? null : NO_REPO)
	};
}

const SCREENS: { id: string; title: string; path: string; keywords: string[] }[] = [
	{ id: 'go.graph', title: 'Go to Graph', path: '/', keywords: ['commits', 'history', 'log'] },
	{ id: 'go.changes', title: 'Go to Changes', path: '/changes', keywords: ['wip', 'status', 'stage'] },
	{ id: 'go.branches', title: 'Go to Branches', path: '/branches', keywords: ['refs', 'tags'] },
	{ id: 'go.stash', title: 'Go to Stashes', path: '/stash', keywords: ['shelf'] },
	{ id: 'go.search', title: 'Go to Search', path: '/search', keywords: ['find', 'grep'] },
	{ id: 'go.requests', title: 'Go to Pull requests', path: '/requests', keywords: ['pr', 'review'] },
	{ id: 'go.conflicts', title: 'Go to Conflicts', path: '/conflicts', keywords: ['merge'] },
	{ id: 'go.rebase', title: 'Go to Interactive rebase', path: '/rebase', keywords: ['reorder', 'squash'] },
	{ id: 'go.farm', title: 'Go to Farm', path: '/farm', keywords: ['tasks', 'dag', 'goal'] },
	{ id: 'go.farm.agents', title: 'Go to Farm agents', path: '/farm?pane=agents', keywords: ['agents', 'installed', 'claude', 'codex', 'cursor', 'farm'] },
	{ id: 'go.farm.settings', title: 'Go to Farm settings', path: '/farm?pane=settings', keywords: ['farm', 'settings', 'goal', 'policy', 'agents.md'] },
	{ id: 'go.badges', title: 'Go to Badges', path: '/badges', keywords: ['achievements', 'title', 'agents', 'delight'] },
	{ id: 'go.repos', title: 'Go to Repositories', path: '/repos', keywords: ['open', 'clone', 'recent'] },
	{ id: 'go.history', title: 'Go to File history', path: '/history', keywords: ['blame', 'file', 'evolution'] },
	{ id: 'go.settings', title: 'Go to Settings', path: '/settings', keywords: ['preferences', 'options'] }
];

/** The columns the header can toggle, named as the palette should say them. */
const TOGGLEABLE: { id: ColumnId; title: string }[] = [
	{ id: 'author', title: 'Author' },
	{ id: 'time', title: 'Date/Time' },
	{ id: 'sha', title: 'SHA' },
	{ id: 'refs', title: 'Branch/Tag' }
];

function navigation(): Command[] {
	return SCREENS.map(({ id, title, path, keywords }) => ({
		id,
		title,
		group: 'Go',
		keywords,
		shortcut: id === 'go.search' ? shortcut('F') : undefined,
		run: () => goto(path)
	}));
}

function view(): Command[] {
	return [
		repoCommand({
			id: 'view.showAll',
			title: 'Show all branches',
			group: 'View',
			keywords: ['unhide', 'unsolo', 'reset visibility'],
			run: () => visibility.showAll()
		}),
		repoCommand({
			id: 'view.smart',
			title: 'Smart branch visibility',
			group: 'View',
			keywords: ['focus', 'checked out', 'upstream'],
			run: () => visibility.setMode('smart')
		}),
		...TOGGLEABLE.map((column) => ({
			id: `view.column.${column.id}`,
			title: `Toggle ${column.title} column`,
			group: 'View',
			keywords: ['column', 'show', 'hide'],
			run: () => columns.toggle(column.id)
		})),
		{
			id: 'view.columns.reset',
			title: 'Reset columns',
			group: 'View',
			keywords: ['layout', 'widths', 'default'],
			run: () => columns.reset()
		},
		{
			id: 'view.author.clear',
			title: 'Clear author filter',
			group: 'View',
			keywords: ['team', 'unfilter'],
			enabled: () => columns.author.trim() !== '',
			unavailable: () => (columns.author.trim() === '' ? 'No author filter set' : null),
			run: () => columns.setAuthor('')
		}
	];
}

function appearance(): Command[] {
	return [
		{
			id: 'appearance.theme.toggle',
			title: 'Toggle dark mode',
			group: 'Appearance',
			keywords: ['light', 'dark', 'theme'],
			run: () => theme.toggle()
		},
		{
			id: 'appearance.zoom.in',
			title: 'Zoom in',
			group: 'Appearance',
			keywords: ['bigger', 'interface scale'],
			shortcut: shortcut('+'),
			run: () => scale.zoomIn()
		},
		{
			id: 'appearance.zoom.out',
			title: 'Zoom out',
			group: 'Appearance',
			keywords: ['smaller', 'interface scale'],
			shortcut: shortcut('−'),
			run: () => scale.zoomOut()
		},
		{
			id: 'appearance.zoom.reset',
			title: 'Reset zoom and text size',
			group: 'Appearance',
			keywords: ['100%', 'default scale'],
			shortcut: shortcut('0'),
			run: () => scale.reset()
		},
		{
			id: 'appearance.text.bigger',
			title: 'Increase text size',
			group: 'Appearance',
			keywords: ['font', 'larger', 'readable'],
			run: () => scale.setText(scale.text + 0.05)
		},
		{
			id: 'appearance.text.smaller',
			title: 'Decrease text size',
			group: 'Appearance',
			keywords: ['font', 'smaller', 'dense'],
			run: () => scale.setText(scale.text - 0.05)
		}
	];
}

function repository(): Command[] {
	return [
		repoCommand({
			id: 'repo.fetch',
			title: 'Fetch all',
			group: 'Repository',
			keywords: ['remote', 'update', 'prune'],
			run: () => fetchAll()
		}),
		repoCommand({
			id: 'repo.push',
			title: 'Push current branch',
			group: 'Repository',
			keywords: ['upload', 'publish', 'remote'],
			run: () => pushCurrent()
		}),
		{
			id: 'repo.commands',
			title: 'Show git commands',
			group: 'Repository',
			keywords: ['log', 'transcript', 'what ran', 'cli'],
			// Gated on the Settings toggle rather than hidden: a command listed with
			// the reason it cannot run is how the palette says the feature exists and
			// where it is turned on.
			enabled: () => settings.settings.showGitCommands,
			unavailable: () =>
				settings.settings.showGitCommands
					? null
					: 'Turn on \u201cShow the git command behind each action\u201d in Settings',
			run: () => commandLog.toggle()
		},
		repoCommand({
			id: 'repo.refresh',
			title: 'Refresh repository',
			group: 'Repository',
			keywords: ['reload', 'rescan'],
			run: () => repo.refresh()
		}),
		repoCommand({
			id: 'repo.worktrees',
			title: 'Worktrees…',
			group: 'Repository',
			keywords: ['worktree', 'linked', 'switch'],
			run: () => {
				void worktrees.fetch();
				worktreeModal.showManager();
			}
		}),
		repoCommand({
			id: 'repo.worktree_add',
			title: 'Add worktree…',
			group: 'Repository',
			keywords: ['worktree', 'create', 'new'],
			run: () => {
				worktreeModal.showAdd();
			}
		}),
		repoCommand({
			id: 'repo.submodules',
			title: 'Submodules…',
			group: 'Repository',
			keywords: ['submodule', 'nested', 'gitmodules'],
			run: () => {
				void submodules.fetch();
				submoduleModal.show();
			}
		}),
	];
}

/** Everything the shell contributes, in one call. */
export function registerCommands(): void {
	palette.register(...navigation(), ...view(), ...appearance(), ...repository());
}
