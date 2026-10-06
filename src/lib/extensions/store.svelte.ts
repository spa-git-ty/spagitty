// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The extensions the window knows about, and what they are doing (FEAT-096).
 *
 * One store, mounted by the shell, because an operation started from the
 * palette can finish after the person has moved to another screen, and a
 * confirmation the backend asks for has to be answerable from anywhere.
 *
 * **Contributions are registered as a group and removed as a group.** Every
 * time the listing changes, the palette's extension commands are replaced
 * wholesale: an extension that was disabled, failed or uninstalled leaves no
 * command behind, and nothing has to remember to unregister it.
 */

import { untrack } from 'svelte';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from './api';
import {
	NO_FACTS,
	paletteCommands,
	unavailable,
	type Facts,
	type Placed
} from './contributions';
import {
	CONFIRM_EVENT,
	EXTENSION_EVENT,
	type CommandContribution,
	type Confirmation,
	type ContextKind,
	type ExtensionView,
	type HostEvent,
	type Invocation,
	type Listing,
	type ReviewRecord,
	type ReviewRequest,
	type ReviewScope,
	type ReviewTarget,
	type ScopePreview
} from './types';
import { palette, type Command } from '$lib/palette/store.svelte';
import { notice } from '$lib/ui/notice.svelte';

/** A running operation, as the screens show it. */
export interface Running {
	operation: string;
	extension: string;
	title: string;
	reviewId: string | null;
	message: string | null;
	findings: number;
	elapsedMs: number;
	startedAt: number;
	cancelling: boolean;
}

/** What a screen is showing, published so commands can run against it. */
export interface ScreenContext {
	taskId?: string | null;
	taskHasCommit?: boolean;
	pullRequest?: { number: number; headSha?: string | null } | null;
	forgeConnected?: boolean;
	/** For the farm: the branch the task merges into. */
	base?: string | null;
	/** For the farm: the task's worktree, reviewed instead of the checkout. */
	workdir?: string | null;
}

/** A review being set up: the person is looking at its scope. */
export interface ReviewDraft {
	extension: ExtensionView;
	provider: string;
	target: ReviewTarget;
	workdir: string;
	request: ReviewRequest;
	scopes: ReviewScope[];
	bases: string[];
	preview: ScopePreview | null;
	error: string | null;
	busy: boolean;
	/** True the first time this repository sends code to this provider. */
	firstUpload: boolean;
}

let listing = $state<Listing | null>(null);
let workdir = $state<string | null>(null);
let error = $state<string | null>(null);
let running = $state<Running[]>([]);
let finished = $state<Record<string, { status: string; message: string; reviewId: string | null }>>({});
let confirmations = $state<Confirmation[]>([]);
let previewGeneration = 0;
let reviews = $state<Record<string, ReviewRecord[]>>({});
let draft = $state<ReviewDraft | null>(null);
let contexts = $state<Partial<Record<ContextKind, ScreenContext>>>({});
let workingChanges = $state(false);
let registered: string[] = [];
let unlisteners: UnlistenFn[] = [];
let refreshTimer: ReturnType<typeof setTimeout> | null = null;
let generation = 0;

function facts(): Facts {
	const task = contexts.farmTask;
	const pr = contexts.pullRequest;
	return {
		...NO_FACTS,
		repositoryOpen: workdir !== null,
		hasWorkingChanges: workingChanges,
		taskSelected: Boolean(task?.taskId),
		taskHasCommit: Boolean(task?.taskHasCommit),
		pullRequestSelected: Boolean(pr?.pullRequest),
		forgeConnected: Boolean(pr?.forgeConnected)
	};
}

function invocationFor(kind: ContextKind): Invocation {
	const context = contexts[kind] ?? {};
	return {
		kind,
		workdir: context.workdir ?? workdir,
		taskId: context.taskId ?? null,
		pullRequest: context.pullRequest ?? null
	};
}

function describe(error: unknown): string {
	if (error && typeof error === 'object' && 'message' in error) {
		return String((error as { message: unknown }).message);
	}
	return typeof error === 'string' ? error : String(error);
}

/** Replace the palette's extension commands with the current set. */
function registerPalette(): void {
	if (registered.length) palette.unregister(...registered);
	const placed = paletteCommands(listing?.extensions ?? []);
	const commands: Command[] = placed.map((entry) => toPaletteCommand(entry));
	registered = commands.map((c) => c.id);
	if (commands.length) palette.register(...commands);
}

function toPaletteCommand({ extension, command, key }: Placed): Command {
	return {
		id: `extension:${key}`,
		title: command.title,
		group: extension.name,
		keywords: [extension.name, extension.publisher],
		enabled: () => unavailable(current(extension.id) ?? extension, command, facts()) === null,
		unavailable: () => unavailable(current(extension.id) ?? extension, command, facts()),
		run: () => extensions.run(extension.id, command)
	};
}

function current(id: string): ExtensionView | undefined {
	return listing?.extensions.find((e) => e.id === id);
}

function onEvent(event: HostEvent): void {
	switch (event.kind) {
		case 'changed':
			scheduleRefresh();
			break;
		case 'operationStarted':
			running = [
				...running.filter((r) => r.operation !== event.operation),
				{
					operation: event.operation,
					extension: event.extension,
					title: event.title,
					reviewId: event.reviewId,
					message: null,
					findings: 0,
					elapsedMs: 0,
					startedAt: Date.now(),
					cancelling: false
				}
			];
			break;
		case 'operationProgress':
			running = running.map((r) =>
				r.operation === event.operation
					? {
							...r,
							message: event.message ?? r.message,
							findings: Math.max(r.findings, event.findings),
							elapsedMs: Math.max(r.elapsedMs, event.elapsedMs),
							cancelling: r.cancelling || event.message === 'Cancelling…'
						}
					: r
			);
			break;
		case 'operationFinished': {
			const was = running.find((r) => r.operation === event.operation);
			running = running.filter((r) => r.operation !== event.operation);
			finished = {
				...finished,
				[event.operation]: { status: event.status, message: event.message, reviewId: event.reviewId }
			};
			if (event.reviewId) void extensions.loadReviews(event.extension);
			const name = current(event.extension)?.name ?? event.extension;
			const title = was?.title ?? name;
			if (event.status === 'completed' && !event.reviewId) {
				if (event.message) notice.ok(title, event.message);
			} else if (event.status === 'failed') {
				notice.failed(`${title} failed`, event.message);
			}
			break;
		}
		case 'notice': {
			const name = current(event.extension)?.name ?? 'Extension';
			if (event.level === 'error') notice.failed(name, event.message);
			else notice.ok(name, event.message);
			break;
		}
	}
}

function scheduleRefresh(): void {
	if (refreshTimer) return;
	refreshTimer = setTimeout(() => {
		refreshTimer = null;
		void extensions.refresh();
	}, 60);
}

export const extensions = {
	get listing(): Listing | null {
		return listing;
	},
	get all(): ExtensionView[] {
		return listing?.extensions ?? [];
	},
	get workdir(): string | null {
		return workdir;
	},
	get error(): string | null {
		return error;
	},
	get running(): Running[] {
		return running;
	},
	get confirmations(): Confirmation[] {
		return confirmations;
	},
	get draft(): ReviewDraft | null {
		return draft;
	},
	get facts(): Facts {
		return facts();
	},
	finishedOf(operation: string) {
		return finished[operation] ?? null;
	},
	reviewsOf(id: string): ReviewRecord[] {
		return reviews[id] ?? [];
	},
	get(id: string): ExtensionView | undefined {
		return current(id);
	},

	/** Subscribe to the host. Called once by the shell. */
	async start(): Promise<void> {
		if (unlisteners.length) return;
		unlisteners.push(await listen<HostEvent>(EXTENSION_EVENT, (e) => onEvent(e.payload)));
		unlisteners.push(
			await listen<Confirmation>(CONFIRM_EVENT, (e) => {
				confirmations = [...confirmations, e.payload];
			})
		);
		unlisteners.push(await listen<string>('extension-confirm-ended', (e) => {
			confirmations = confirmations.filter((c) => c.id !== e.payload);
		}));
	},

	stop(): void {
		for (const unlisten of unlisteners) unlisten();
		unlisteners = [];
	},

	/** The repository the screens are about. Reads the list for it. */
	async setRepository(next: string | null): Promise<void> {
		if (untrack(() => next === workdir && listing !== null)) return;
		workdir = next;
		draft = null;
		previewGeneration++;
		reviews = {};
		await this.refresh();
	},

	setWorkingChanges(any: boolean): void {
		workingChanges = any;
	},

	/** A screen says what it is showing; `null` when it goes away. */
	setContext(kind: ContextKind, context: ScreenContext | null): void {
		// Untracked: screens call this from an effect, and reading the state
		// being written would make that effect depend on its own output.
		const next = { ...untrack(() => contexts) };
		if (context) next[kind] = context;
		else delete next[kind];
		contexts = next;
	},

	async refresh(): Promise<void> {
		const mine = ++generation;
		try {
			const next = await api.list(workdir);
			if (mine !== generation) return;
			listing = next;
			error = null;
		} catch (failure) {
			if (mine !== generation) return;
			error = describe(failure);
		}
		registerPalette();
	},

	/** Run a command. One that starts a review opens its scope first. */
	async run(id: string, command: CommandContribution): Promise<void> {
		const extension = current(id);
		if (!extension) return;
		const context = command.context ?? 'global';
		const reason = unavailable(extension, command, facts());
		if (reason) {
			notice.failed(command.title, reason);
			return;
		}
		if (command.reviewProvider) {
			const target: ReviewTarget = context === 'farmTask' ? 'farmTask' : context === 'pullRequest' ? 'pullRequest' : 'workingCopy';
			await this.beginReview(id, command.reviewProvider, target);
			return;
		}
		try {
			await api.runCommand(id, command.id, invocationFor(context));
		} catch (failure) {
			notice.failed(command.title, describe(failure));
		}
	},

	/** Open the scope dialog for a review. Nothing is sent yet. */
	async beginReview(id: string, provider: string, target: ReviewTarget): Promise<void> {
		const extension = current(id);
		const context = target === 'farmTask' ? contexts.farmTask : undefined;
		const dir = context?.workdir ?? workdir;
		if (!extension || !dir) return;
		const declared = extension.manifest.contributes?.reviewProviders?.find((p) => p.id === provider);
		const scopes: ReviewScope[] =
			target === 'farmTask'
				? ['committed']
				: declared?.scopes?.length
					? declared.scopes
					: ['uncommitted', 'includeUntracked', 'committed'];
		const generation = ++previewGeneration;
		let bases: string[] = [];
		try {
			bases = await api.suggestedBases(dir);
		} catch {
			bases = [];
		}
		if (generation !== previewGeneration) return;
		const firstUpload = this.reviewsOf(id).length === 0 && (declared?.sendsCodeTo ?? null) !== null;
		draft = {
			extension,
			provider,
			target,
			workdir: dir,
			request: {
				target,
				scope: scopes[0],
				base: target === 'farmTask' ? (context?.base ?? null) : null,
				taskId: context?.taskId ?? null
			},
			scopes,
			bases,
			preview: null,
			error: null,
			busy: false,
			firstUpload
		};
		await this.previewDraft();
	},

	async updateDraft(change: Partial<ReviewRequest>): Promise<void> {
		if (!draft) return;
		draft = { ...draft, request: { ...draft.request, ...change } };
		await this.previewDraft();
	},

	async previewDraft(): Promise<void> {
		const open = draft;
		if (!open) return;
		const generation = ++previewGeneration;
		const needsBase = open.request.scope === 'committed' || open.request.scope === 'tracked';
		const request = needsBase && !open.request.base ? { ...open.request, base: open.bases[0] ?? null } : open.request;
		draft = { ...open, request, busy: true, error: null };
		try {
			const preview = await api.previewReview(open.extension.id, open.provider, request, open.workdir);
			if (generation === previewGeneration && draft?.extension.id === open.extension.id) draft = { ...draft, preview, busy: false };
		} catch (failure) {
			if (generation === previewGeneration && draft) draft = { ...draft, preview: null, error: describe(failure), busy: false };
		}
	},

	closeDraft(): void {
		previewGeneration++;
		draft = null;
	},

	/** Start the drafted review. */
	async confirmDraft(): Promise<void> {
		const open = draft;
		if (!open || !open.preview) return;
		const mine = previewGeneration;
		draft = { ...open, busy: true };
		try {
			await api.startReview(open.extension.id, open.provider, open.request, open.workdir);
			if (mine === previewGeneration) draft = null;
		} catch (failure) {
			if (mine === previewGeneration) draft = { ...open, busy: false, error: describe(failure) };
		}
	},

	async cancel(operation: string): Promise<void> {
		running = running.map((r) => (r.operation === operation ? { ...r, cancelling: true } : r));
		try {
			await api.cancel(operation);
		} catch (failure) {
			notice.failed('Could not cancel', describe(failure));
		}
	},

	async loadReviews(id: string): Promise<void> {
		if (!workdir) return;
		const mine = generation;
		const directory = workdir;
		try {
			const records = await api.reviews(id, directory);
			if (mine === generation) reviews = { ...reviews, [id]: records };
		} catch (failure) {
			if (mine === generation) error = describe(failure);
		}
	},

	/** Send selected findings to an agent (FEAT-097). */
	async send(id: string, record: ReviewRecord, findings: string[]): Promise<void> {
		if (!workdir) return;
		try {
			const sent = await api.sendFindings(id, workdir, record.result.reviewId, findings);
			notice.ok(`Sent ${findings.length} to an agent`, sent.message);
			await this.loadReviews(id);
		} catch (failure) {
			notice.failed('Not sent', describe(failure));
		}
	},

	/** Answer a confirmation the backend asked for. */
	async answer(id: string, approved: boolean): Promise<void> {
		confirmations = confirmations.filter((c) => c.id !== id);
		await api.confirm(id, approved);
	},

	invocationFor,

	/** Test seam. */
	reset(): void {
		this.stop();
		listing = null;
		workdir = null;
		error = null;
		running = [];
		finished = {};
		confirmations = [];
		reviews = {};
		draft = null;
		contexts = {};
		workingChanges = false;
		if (registered.length) palette.unregister(...registered);
		registered = [];
		generation += 1;
		previewGeneration++;
	},

	/** Test seam: feed an event as if it came from the host. */
	receive(event: HostEvent): void {
		onEvent(event);
	}
};
