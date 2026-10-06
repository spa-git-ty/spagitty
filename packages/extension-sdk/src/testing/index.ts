// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A fake Spagitty for testing a worker (FEAT-096).
 *
 * `FakeHost` plays the host's half of the protocol against a real worker
 * process — or an in-memory one — and checks it while it does: every stdout
 * line must be JSON-RPC, a worker's own request ids must start with `w`, each
 * operation must complete exactly once, and nothing may arrive for an
 * operation after it completed. Violations are collected, not thrown, so a
 * test can assert on all of them at once.
 *
 * ```ts
 * const host = FakeHost.spawn(['bun', 'run', 'src/main.ts']);
 * await host.start();
 * const result = await host.command('hello', { kind: 'workingCopy', repository: 'repo:1' });
 * expect(result.status).toBe('completed');
 * expect(host.violations).toEqual([]);
 * await host.stop();
 * ```
 */

import { spawn, type ChildProcess } from 'node:child_process';
import { PassThrough } from 'node:stream';
import { API_VERSION, ErrorCode, type Capability, type Finding, type InvocationContext, type ReviewSnapshot } from '../protocol';
import { Peer, serve, streamChannel, type Channel, type Definition } from '../worker';

/** Refuse a callback with a protocol error code. */
export class Refusal extends Error {
	constructor(
		readonly code: number,
		message: string
	) {
		super(message);
	}
}

export type Service = (params: Record<string, unknown>) => unknown | Promise<unknown>;

export interface Options {
	capabilities?: Capability[];
	settings?: Record<string, unknown>;
	/** Answers to the worker's callbacks, by method. Unlisted methods are refused. */
	services?: Record<string, Service>;
	/** How long to wait for any one answer. */
	timeoutMs?: number;
}

export interface Outcome {
	status: string;
	message: string;
	review?: Record<string, unknown>;
	progress: string[];
	findings: Finding[];
	logs: string[];
}

interface Running extends Outcome {
	done: boolean;
	resolve: (outcome: Outcome) => void;
}

export class FakeHost {
	readonly violations: string[] = [];
	readonly notices: { level: string; message: string }[] = [];
	readonly toolCalls: Record<string, unknown>[] = [];
	readonly logs: string[] = [];
	private peer: Peer;
	private operations = new Map<string, Running>();
	private next = 1;
	private stderr = '';
	private storage = new Map<string, unknown>();

	private constructor(
		channel: Channel,
		private readonly options: Options,
		private readonly child: ChildProcess | null
	) {
		this.peer = new Peer(
			{
				onLine: (handler) =>
					channel.onLine((line) => {
						if (!line.trim()) return;
						try {
							const parsed = JSON.parse(line);
							if (parsed?.jsonrpc !== '2.0') this.violations.push(`not JSON-RPC 2.0: ${line.slice(0, 80)}`);
							else if (typeof parsed.method === 'string' && 'id' in parsed && !String(parsed.id).startsWith('w')) {
								this.violations.push(`a worker request id must start with w: ${JSON.stringify(parsed.id)}`);
							}
						} catch {
							this.violations.push(`stdout carried something that is not JSON: ${line.slice(0, 80)}`);
							return;
						}
						handler(line);
					}),
				onClose: (handler) => channel.onClose(handler),
				write: (line) => channel.write(line)
			},
			'',
			(method, params) => this.answer(method, (params ?? {}) as Record<string, unknown>),
			(method, params) => this.notified(method, (params ?? {}) as Record<string, unknown>)
		);
	}

	/** Start a worker process. */
	static spawn(command: string[], options: Options & { cwd?: string } = {}): FakeHost {
		const child = spawn(command[0], command.slice(1), {
			cwd: options.cwd,
			stdio: ['pipe', 'pipe', 'pipe'],
			env: { ...process.env, SPAGITTY_EXTENSION_API: '1' }
		});
		const channel = streamChannel(child.stdout!, child.stdin!);
		const host = new FakeHost(channel, options, child);
		child.stderr!.on('data', (chunk) => (host.stderr += chunk.toString()));
		return host;
	}

	/** Run a definition in this process, for fast unit tests. */
	static inMemory(definition: Definition, options: Options = {}): FakeHost {
		const toWorker = new PassThrough();
		const toHost = new PassThrough();
		serve(definition, streamChannel(toWorker, toHost));
		return new FakeHost(streamChannel(toHost, toWorker), options, null);
	}

	get diagnostics(): string {
		return this.stderr;
	}

	private timeout<T>(promise: Promise<T>, what: string): Promise<T> {
		const ms = this.options.timeoutMs ?? 10_000;
		return Promise.race([
			promise,
			new Promise<T>((_, reject) => setTimeout(() => reject(new Error(`${what} took longer than ${ms} ms`)), ms))
		]);
	}

	/** The handshake and activation, as the host does them. */
	async start(reason = 'command', id = 'com.example.test', version = '1.0.0'): Promise<Record<string, unknown>> {
		const answer = (await this.timeout(
			this.peer.call('extension.initialize', {
				apiVersions: [API_VERSION],
				host: { name: 'Spagitty (fake)', version: '0.9.0' },
				extension: { id, version },
				session: 's-test',
				capabilities: this.options.capabilities ?? [],
				locale: 'en',
				platform: 'test',
				limits: { maxMessageBytes: 1048576, maxConcurrentOperations: 4, inactivityMs: 120000, cancelGraceMs: 5000 },
				settings: this.options.settings ?? {}
			}),
			'the handshake'
		)) as { apiVersion?: string };
		if (!answer.apiVersion || answer.apiVersion.split('.')[0] !== API_VERSION.split('.')[0]) {
			this.violations.push(`handshake answered API ${String(answer.apiVersion)}`);
		}
		return (await this.timeout(this.peer.call('extension.activate', { reason }), 'activation')) as Record<string, unknown>;
	}

	private begin(operation: string): Promise<Outcome> {
		return new Promise((resolve) => {
			this.operations.set(operation, { status: '', message: '', progress: [], findings: [], logs: [], done: false, resolve });
		});
	}

	/** Run a command and wait for its one completion. */
	async command(command: string, context: InvocationContext = { kind: 'global' }): Promise<Outcome> {
		const operationId = `op-${this.next++}`;
		const done = this.begin(operationId);
		await this.peer.call('command.execute', { operationId, command, context });
		return this.timeout(done, `command ${command}`);
	}

	/** Start a command and return its id, to cancel it. */
	async startCommand(command: string, context: InvocationContext = { kind: 'global' }): Promise<{ operationId: string; done: Promise<Outcome> }> {
		const operationId = `op-${this.next++}`;
		const done = this.begin(operationId);
		await this.peer.call('command.execute', { operationId, command, context });
		return { operationId, done: this.timeout(done, `command ${command}`) };
	}

	/** Run a review and wait for its one completion. */
	async review(provider: string, snapshot: Partial<ReviewSnapshot> = {}): Promise<Outcome> {
		const operationId = `op-${this.next++}`;
		const done = this.begin(operationId);
		await this.peer.call('review.start', {
			operationId,
			provider,
			reviewId: `rv-${operationId}`,
			repository: 'repo:1',
			workdir: 'wd:1',
			snapshot: {
				id: 'snap-test',
				repositoryId: 'repo-test',
				target: 'workingCopy',
				baseCommit: '0'.repeat(40),
				headCommit: '1'.repeat(40),
				scope: 'uncommitted',
				contentDigest: `sha256:${'2'.repeat(64)}`,
				takenAt: new Date(0).toISOString(),
				...snapshot
			}
		});
		return this.timeout(done, `review ${provider}`);
	}

	panel(panel: string, context: InvocationContext = { kind: 'global' }): Promise<unknown> {
		return this.timeout(this.peer.call('panel.resolve', { panel, context }), `panel ${panel}`);
	}

	cancel(operationId: string): void {
		this.peer.notify('operation.cancel', { operationId });
	}

	changeSettings(settings: Record<string, unknown>): void {
		this.peer.notify('settings.changed', { settings });
	}

	/** Deactivate and wait for the process to exit. */
	async stop(): Promise<number | null> {
		try {
			await this.timeout(this.peer.call('extension.deactivate', {}), 'deactivation');
		} catch (error) {
			this.violations.push(String((error as Error).message));
		}
		if (!this.child) return 0;
		this.child.stdin?.end();
		return new Promise((resolve) => {
			const timer = setTimeout(() => {
				this.violations.push('the worker did not exit after its stdin closed');
				this.child?.kill();
				resolve(null);
			}, 5000);
			this.child!.on('exit', (code) => {
				clearTimeout(timer);
				resolve(code);
			});
			if (this.child!.exitCode !== null) {
				clearTimeout(timer);
				resolve(this.child!.exitCode);
			}
		});
	}

	private notified(method: string, params: Record<string, unknown>): void {
		const id = String(params.operationId ?? '');
		const op = this.operations.get(id);
		switch (method) {
			case 'operation.progress':
				if (!op) this.violations.push(`progress for an unknown operation ${id}`);
				else if (op.done) this.violations.push(`progress after ${id} completed`);
				else if (typeof params.message === 'string') op.progress.push(params.message);
				break;
			case 'review.findings':
				if (!op || op.done) this.violations.push(`findings after ${id} completed`);
				else op.findings.push(...((params.findings as Finding[]) ?? []));
				break;
			case 'operation.complete':
				if (!op) this.violations.push(`completion for an unknown operation ${id}`);
				else if (op.done) this.violations.push(`${id} completed twice`);
				else {
					op.done = true;
					op.status = String(params.status);
					op.message = String(params.message ?? '');
					op.review = params.review as Record<string, unknown> | undefined;
					op.resolve(op);
				}
				break;
			case 'log':
				this.logs.push(`[${String(params.level)}] ${String(params.message)}`);
				break;
			default:
				this.violations.push(`an unknown notification: ${method}`);
		}
	}

	private async answer(method: string, params: Record<string, unknown>): Promise<unknown> {
		const custom = this.options.services?.[method];
		if (custom) {
			try {
				return await custom(params);
			} catch (error) {
				if (error instanceof Refusal) throw Object.assign(new Error(error.message), { code: error.code });
				throw error;
			}
		}
		switch (method) {
			case 'ui.notify':
				this.notices.push({ level: String(params.level), message: String(params.message) });
				return {};
			case 'storage.get':
				return { value: this.storage.get(String(params.key)) ?? null };
			case 'storage.set':
				this.storage.set(String(params.key), params.value);
				return {};
			case 'tools.run':
				this.toolCalls.push(params);
				throw Object.assign(new Error('no tools in the fake host'), { code: ErrorCode.ToolMissing });
			default:
				throw Object.assign(new Error(`the fake host does not answer ${method}`), { code: ErrorCode.NotGranted });
		}
	}
}
