// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Write an extension as a set of handlers; the SDK speaks the protocol.
 *
 * ```ts
 * import { defineExtension, run } from '@spagitty/extension-sdk';
 *
 * run(defineExtension({
 *   commands: {
 *     async hello(ctx) {
 *       const repo = await ctx.host.describeRepository(ctx.context.repository!);
 *       await ctx.host.notify(`Hello from ${repo.branch}`);
 *       return { message: 'Said hello' };
 *     }
 *   }
 * }));
 * ```
 *
 * What it takes care of: line framing on stdin/stdout; answering the
 * handshake; dispatching each request concurrently while it keeps reading;
 * correlating the host's answers to your callbacks; sending **exactly one**
 * `operation.complete` per operation, whatever your handler does — returns,
 * throws, or is cancelled; and an `AbortSignal` that fires when the host asks
 * an operation to stop.
 *
 * Nothing goes to stdout but protocol. `console.log` is redirected to stderr
 * by `run`, because one stray line on stdout ends the worker.
 */

import {
	API_VERSION,
	ErrorCode,
	MAX_MESSAGE_BYTES,
	type CommandParams,
	type CommentReceipt,
	type PullRequestSnapshot,
	type ReviewReadiness,
	type Detected,
	type Finding,
	type InitializeParams,
	type InvocationContext,
	type Message,
	type OperationStatus,
	type PanelParams,
	type ReviewOutcome,
	type ReviewSnapshot,
	type ReviewStartParams,
	type ReviewStatusPanel,
	type RpcError,
	type SummaryPanel,
	type ToolRunResult
} from './protocol';

/** An error to answer a host request with. */
export class ExtensionError extends Error {
	constructor(
		readonly code: number,
		message: string,
		readonly data?: unknown
	) {
		super(message);
		this.name = 'ExtensionError';
	}
}

/** What the host answered a callback with, when it refused. */
export class HostError extends Error {
	constructor(
		readonly code: number,
		message: string,
		readonly data?: unknown
	) {
		super(message);
		this.name = 'HostError';
	}
}

export interface RepositoryDescription {
	name: string | null;
	branch: string | null;
	head: string | null;
	detached: boolean;
	remotes: { name: string; forge: string | null }[];
	forge: { kind: string; host: string; slug: string } | null;
}

/** The host's services, typed. Every call is checked by the host. */
export class Host {
	constructor(private readonly peer: Peer) {}

	describeRepository(repository: string): Promise<RepositoryDescription> {
		return this.peer.call('repository.describe', { repository }) as Promise<RepositoryDescription>;
	}

	changes(repository: string, operationId?: string): Promise<{ files: { path: string; status: string; origin: string }[]; truncated: boolean }> {
		return this.peer.call('repository.changes', { repository, operationId }) as Promise<never>;
	}

	reviewSnapshot(operationId: string): Promise<ReviewSnapshot> {
		return this.peer.call('review.snapshot', { operationId }) as Promise<ReviewSnapshot>;
	}

	detectTool(tool: string): Promise<Detected> {
		return this.peer.call('tools.detect', { tool }) as Promise<Detected>;
	}

	/**
	 * Run a declared tool through a declared profile. `onLine` receives each
	 * stdout line as it arrives; the promise settles when the process ends.
	 */
	async runTool(
		params: { operationId?: string; tool: string; profile: string; options?: Record<string, string>; workdir?: string },
		onLine?: (line: string) => void
	): Promise<ToolRunResult> {
		const listener = onLine ? this.peer.watchTool(params.operationId ?? null, onLine) : null;
		try {
			return (await this.peer.call('tools.run', params)) as ToolRunResult;
		} finally {
			listener?.();
		}
	}

	pullRequestSnapshot(repository: string, number: number): Promise<PullRequestSnapshot> {
		return this.peer.call('forge.pullRequest.snapshot', { repository, number }) as Promise<PullRequestSnapshot>;
	}

	/** Ask to post `body`. The person sees it first and may say no (`ErrorCode.Declined`). */
	commentOnPullRequest(repository: string, number: number, body: string, operationId?: string): Promise<CommentReceipt> {
		return this.peer.call('forge.pullRequest.comment', { repository, number, body, operationId }) as Promise<CommentReceipt>;
	}

	async storageGet<T = unknown>(key: string, repository?: string): Promise<T | null> {
		const answer = (await this.peer.call('storage.get', { key, repository })) as { value: T | null };
		return answer.value ?? null;
	}

	async storageSet(key: string, value: unknown, repository?: string): Promise<void> {
		await this.peer.call('storage.set', { key, value, repository });
	}

	async notify(message: string, level: 'info' | 'warn' | 'error' = 'info'): Promise<void> {
		await this.peer.call('ui.notify', { level, message });
	}
}

export interface OperationContext {
	operationId: string;
	host: Host;
	/** Fires when the host asks the operation to stop. */
	signal: AbortSignal;
	settings: Record<string, unknown>;
	/** Say what is happening; also resets the host's inactivity timer. */
	progress(message: string, phase?: string): void;
	/** Reset the inactivity timer without saying anything. */
	heartbeat(): void;
	log(message: string, level?: 'debug' | 'info' | 'warn' | 'error'): void;
}

export interface CommandContext extends OperationContext {
	command: string;
	context: InvocationContext;
}

export interface ReviewContext extends OperationContext {
	provider: string;
	reviewId: string;
	repository: string;
	workdir: string;
	snapshot: ReviewSnapshot;
	/** Publish findings as they are found. Ids must be unique within the review. */
	findings(findings: Finding[]): void;
}

export interface CommandResult {
	message?: string;
	status?: OperationStatus;
}

export interface Definition {
	activate?(info: InitializeParams, reason: string): Promise<{ unavailable?: { id: string; reason: string }[] } | void> | { unavailable?: { id: string; reason: string }[] } | void;
	deactivate?(): Promise<void> | void;
	settingsChanged?(settings: Record<string, unknown>): void;
	commands?: Record<string, (ctx: CommandContext) => Promise<CommandResult | void> | CommandResult | void>;
	checkReview?(ctx: OperationContext & { provider: string; repository: string }): Promise<ReviewReadiness> | ReviewReadiness;
	reviewProviders?: Record<string, (ctx: ReviewContext) => Promise<ReviewOutcome> | ReviewOutcome>;
	panels?: Record<string, (params: { context: InvocationContext; host: Host; settings: Record<string, unknown> }) => Promise<SummaryPanel | ReviewStatusPanel> | SummaryPanel | ReviewStatusPanel>;
}

export function defineExtension(definition: Definition): Definition {
	return definition;
}

/** Something to read lines from and write lines to. */
export interface Channel {
	onLine(handler: (line: string) => void): void;
	onClose(handler: () => void): void;
	write(line: string): void;
}

type Pending = { resolve: (value: unknown) => void; reject: (error: Error) => void };

/** One side of a conversation: the protocol mechanics, both directions. */
export class Peer {
	private next = 1;
	private pending = new Map<string, Pending>();
	private toolListeners = new Set<{ operation: string | null; onLine: (line: string) => void }>();

	constructor(
		private readonly channel: Channel,
		private readonly prefix: string,
		private readonly onRequest: (method: string, params: unknown) => Promise<unknown>,
		private readonly onNotification: (method: string, params: unknown) => void
	) {
		channel.onLine((line) => this.receive(line));
		channel.onClose(() => {
			for (const waiter of this.pending.values()) waiter.reject(new HostError(ErrorCode.NotActive, 'the connection closed'));
			this.pending.clear();
		});
	}

	send(message: Message): void {
		const line = JSON.stringify(message);
		if (Buffer.byteLength(line, 'utf8') > MAX_MESSAGE_BYTES) throw new ExtensionError(ErrorCode.Limit, 'message too large');
		this.channel.write(line);
	}

	notify(method: string, params: unknown): void {
		this.send({ jsonrpc: '2.0', method, params });
	}

	call(method: string, params: unknown): Promise<unknown> {
		// The host numbers its requests; a worker prefixes its own with `w`.
		const id: string | number = this.prefix ? `${this.prefix}${this.next++}` : this.next++;
		return new Promise((resolve, reject) => {
			this.pending.set(String(id), { resolve, reject });
			try { this.send({ jsonrpc: '2.0', id, method, params }); }
			catch (error) { this.pending.delete(String(id)); reject(error as Error); }
		});
	}

	watchTool(operation: string | null, onLine: (line: string) => void): () => void {
		const entry = { operation, onLine };
		this.toolListeners.add(entry);
		return () => this.toolListeners.delete(entry);
	}

	private receive(line: string): void {
		const text = line.replace(/\r$/, '');
		if (!text) return;
		let message: Record<string, unknown>;
		try {
			message = JSON.parse(text);
		} catch {
			return;
		}
		if (typeof message.method === 'string') {
			if ('id' in message) {
				const id = message.id as string | number;
				this.onRequest(message.method, message.params).then(
					(result) => this.send({ jsonrpc: '2.0', id, result: result ?? {} }),
					(error: unknown) => {
						const code = (error as { code?: unknown })?.code;
						const e =
							error instanceof ExtensionError
								? error
								: new ExtensionError(
										typeof code === 'number' ? code : ErrorCode.Internal,
										String((error as Error)?.message ?? error)
									);
						const payload: RpcError = { code: e.code, message: e.message };
						if (e.data !== undefined) payload.data = e.data;
						this.send({ jsonrpc: '2.0', id, error: payload });
					}
				);
			} else if (message.method === 'tools.output') {
				const params = message.params as { operationId?: string | null; line: string };
				for (const listener of this.toolListeners) {
					if (listener.operation === null || listener.operation === params.operationId) listener.onLine(params.line);
				}
			} else {
				this.onNotification(message.method, message.params);
			}
			return;
		}
		const key = String(message.id);
		const waiter = this.pending.get(key);
		if (!waiter) return;
		this.pending.delete(key);
		if ('error' in message) {
			const error = message.error as RpcError;
			waiter.reject(new HostError(error.code, error.message, error.data));
		} else waiter.resolve(message.result);
	}
}

/** The worker side, wired to a channel. Exported for tests; use `run`. */
export function serve(definition: Definition, channel: Channel): Peer {
	let info: InitializeParams | null = null;
	let settings: Record<string, unknown> = {};
	const operations = new Map<string, AbortController>();
	const completed = new Set<string>();
	let peer: Peer;

	const complete = (operationId: string, status: OperationStatus, message: string, review?: ReviewOutcome) => {
		if (completed.has(operationId)) return;
		completed.add(operationId);
		operations.delete(operationId);
		const params: Record<string, unknown> = { operationId, status, message };
		if (review) params.review = review;
		peer.notify('operation.complete', params);
	};

	const base = (operationId: string, controller: AbortController): OperationContext => ({
		operationId,
		host: new Host(peer),
		signal: controller.signal,
		settings,
		progress: (message, phase) => peer.notify('operation.progress', { operationId, message, phase }),
		heartbeat: () => peer.notify('operation.progress', { operationId }),
		log: (message, level = 'info') => peer.notify('log', { level, message })
	});

	const start = (operationId: string, work: (controller: AbortController) => Promise<void>) => {
		const controller = new AbortController();
		operations.set(operationId, controller);
		// Not awaited: the request is answered once the work is accepted.
		void work(controller).catch((error: unknown) => {
			const cancelled = controller.signal.aborted;
			complete(operationId, cancelled ? 'cancelled' : 'failed', cancelled ? 'Cancelled.' : String((error as Error)?.message ?? error));
		});
	};

	const onRequest = async (method: string, params: unknown): Promise<unknown> => {
		switch (method) {
			case 'extension.initialize': {
				info = params as InitializeParams;
				settings = info.settings ?? {};
				return { apiVersion: API_VERSION };
			}
			case 'extension.activate': {
				if (!info) throw new ExtensionError(ErrorCode.InvalidRequest, 'not initialized');
				const reason = (params as { reason?: string })?.reason ?? 'command';
				return (await definition.activate?.(info, reason)) ?? {};
			}
			case 'extension.deactivate':
				for (const controller of operations.values()) controller.abort();
				await definition.deactivate?.();
				return {};
			case 'command.execute': {
				const p = params as CommandParams;
				const handler = definition.commands?.[p.command];
				if (!handler) throw new ExtensionError(ErrorCode.MethodNotFound, `no command called ${p.command}`);
				start(p.operationId, async (controller) => {
					const result = (await handler({ ...base(p.operationId, controller), command: p.command, context: p.context })) ?? {};
					complete(p.operationId, controller.signal.aborted ? 'cancelled' : (result.status ?? 'completed'), result.message ?? '');
				});
				return { accepted: true };
			}
			case 'review.check': {
				if (!definition.checkReview) throw new ExtensionError(ErrorCode.MethodNotFound, 'This provider has no readiness check.');
				const p = params as { operationId: string; provider: string; repository: string };
				const controller = new AbortController();
				operations.set(p.operationId, controller);
				try { return await definition.checkReview({ ...base(p.operationId, controller), provider: p.provider, repository: p.repository }); }
				finally { operations.delete(p.operationId); }
			}
			case 'review.start': {
				const p = params as ReviewStartParams;
				const handler = definition.reviewProviders?.[p.provider];
				if (!handler) throw new ExtensionError(ErrorCode.MethodNotFound, `no review provider called ${p.provider}`);
				start(p.operationId, async (controller) => {
					const outcome = await handler({
						...base(p.operationId, controller),
						provider: p.provider,
						reviewId: p.reviewId,
						repository: p.repository,
						workdir: p.workdir,
						snapshot: p.snapshot,
						findings: (findings) => {
							if (!completed.has(p.operationId)) peer.notify('review.findings', { operationId: p.operationId, findings });
						}
					});
					const cancelled = controller.signal.aborted;
					complete(
						p.operationId,
						cancelled ? 'cancelled' : outcome.status === 'failed' ? 'failed' : 'completed',
						outcome.summary ?? '',
						cancelled ? undefined : outcome
					);
				});
				return { accepted: true };
			}
			case 'panel.resolve': {
				const p = params as PanelParams;
				const handler = definition.panels?.[p.panel];
				if (!handler) throw new ExtensionError(ErrorCode.MethodNotFound, `no panel called ${p.panel}`);
				return handler({ context: p.context, host: new Host(peer), settings });
			}
			default:
				throw new ExtensionError(ErrorCode.MethodNotFound, `no method called ${method}`);
		}
	};

	const onNotification = (method: string, params: unknown) => {
		if (method === 'operation.cancel') {
			operations.get((params as { operationId: string }).operationId)?.abort();
		} else if (method === 'settings.changed') {
			settings = (params as { settings: Record<string, unknown> }).settings ?? {};
			definition.settingsChanged?.(settings);
		}
	};

	peer = new Peer(channel, 'w', onRequest, onNotification);
	return peer;
}

/** Line channel over Node/Bun streams. */
export function streamChannel(input: NodeJS.ReadableStream, output: { write(chunk: string): unknown }): Channel {
	let buffer = '';
	const lineHandlers: ((line: string) => void)[] = [];
	const closeHandlers: (() => void)[] = [];
	input.setEncoding?.('utf8');
	input.on('data', (chunk: string | Buffer) => {
		buffer += chunk.toString();
		let at: number;
		while ((at = buffer.indexOf('\n')) >= 0) {
			const line = buffer.slice(0, at);
			buffer = buffer.slice(at + 1);
			for (const handler of lineHandlers) handler(line);
		}
		if (Buffer.byteLength(buffer, 'utf8') > MAX_MESSAGE_BYTES) buffer = '';
	});
	input.on('end', () => closeHandlers.forEach((h) => h()));
	return {
		onLine: (handler) => lineHandlers.push(handler),
		onClose: (handler) => closeHandlers.push(handler),
		write: (line) => void output.write(`${line}\n`)
	};
}

/** Run `definition` as this process's worker, on stdin and stdout. */
export function run(definition: Definition): void {
	// stdout belongs to the protocol. Anything a library logs goes to stderr.
	const toStderr = (...args: unknown[]) => process.stderr.write(`${args.map(String).join(' ')}\n`);
	console.log = toStderr;
	console.info = toStderr;
	console.debug = toStderr;
	const channel = streamChannel(process.stdin, process.stdout);
	channel.onClose(() => process.exit(0));
	serve(definition, channel);
}
