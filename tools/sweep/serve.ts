// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The sweep portal: `bun run sweep`.
 *
 * A page on this machine, and only this machine, for running the manual
 * sweeps: every ticket with its steps, a result to set, a note, and
 * screenshots or a screen recording to attach. A result is written into the
 * sweep document it belongs to, in the one cell or line it lives in, and
 * `agile/testing/sweeps.md` is written again after it.
 *
 * Attached files go to `agile/testing/evidence/<item>/<ticket>/`, which is
 * not committed: screenshots and recordings would make the repository heavy
 * for everybody who clones it. The result links them.
 *
 * With `--phone` it is also on the network this machine is on, for a sweep
 * run from a phone on the same Wi-Fi or hotspot. Anybody on that network
 * could reach it then, so it answers only a request that carries a key made
 * when it starts: the address it prints has the key in it, and the page,
 * once opened with it, keeps it in a cookie.
 *
 * Usage:
 *   bun tools/sweep/serve.ts [--port 5199] [--no-open] [--phone]
 */

import { randomBytes, timingSafeEqual } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { networkInterfaces } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { spawn } from 'node:child_process';
import { INDEX, TESTING, current, listed, readAll } from './index';
import { STATES, parse, resultText, write, type State } from './sweeps';

const EVIDENCE = join(TESTING, 'evidence');
/** From the repository's root, as `TESTING` is. */
const PAGE = join('tools', 'sweep', 'portal.html');
/** A recording of a few minutes fits; a film does not. */
const LARGEST = 500 * 1024 * 1024;
const KINDS: Record<string, string> = {
	png: 'image/png',
	jpg: 'image/jpeg',
	jpeg: 'image/jpeg',
	gif: 'image/gif',
	webp: 'image/webp',
	mp4: 'video/mp4',
	webm: 'video/webm',
	mov: 'video/quicktime',
	txt: 'text/plain; charset=utf-8',
	log: 'text/plain; charset=utf-8',
	json: 'application/json'
};

const SWEEP_FILE = /^(?:FEAT|TASK|BUG)-\d{3}[a-z]?-sweep\.md$/;
const TICKET = /^[A-Za-z0-9][A-Za-z0-9-]*$/;

function today(): string {
	const now = new Date();
	const two = (value: number) => String(value).padStart(2, '0');
	return `${now.getFullYear()}-${two(now.getMonth() + 1)}-${two(now.getDate())}`;
}

/** Where a ticket's files are kept, made safe from what the page sent. */
function folder(file: string, ticket: string): string {
	if (!SWEEP_FILE.test(file) || !TICKET.test(ticket)) throw new Error('not a sweep ticket');
	return join(EVIDENCE, file.replace(/-sweep\.md$/, ''), ticket);
}

/** The files kept for a ticket, as links from its sweep document. */
function evidenceOf(file: string, ticket: string): string[] {
	const dir = folder(file, ticket);
	if (!existsSync(dir)) return [];
	return readdirSync(dir)
		.filter((name) => statSync(join(dir, name)).isFile())
		.sort()
		.map((name) => `evidence/${file.replace(/-sweep\.md$/, '')}/${ticket}/${name}`);
}

function everything() {
	const items = listed(readFileSync('agile/README.md', 'utf8'));
	return {
		day: today(),
		states: STATES,
		sweeps: readAll().map((sweep) => ({
			...sweep,
			title: items.get(sweep.item)?.title ?? sweep.heading,
			status: items.get(sweep.item)?.status ?? '',
			tickets: sweep.tickets.map((ticket) => ({ ...ticket, evidence: evidenceOf(sweep.file, ticket.id) }))
		}))
	};
}

interface Asked {
	file: string;
	id: string;
	state: State;
	note: string;
}

/** Write one result, and the index after it. */
function record(asked: Asked) {
	if (!SWEEP_FILE.test(asked.file)) throw new Error('not a sweep document');
	if (!STATES.includes(asked.state) || asked.state === 'noted') throw new Error('not a result');
	const path = join(TESTING, asked.file);
	const text = readFileSync(path, 'utf8');
	const ticket = parse(asked.file, text).tickets.find((one) => one.id === asked.id);
	if (!ticket) throw new Error(`${asked.id} is not in ${asked.file}`);
	const links = evidenceOf(asked.file, asked.id).map((link) => `[${link.split('/').pop()}](${link})`);
	writeFileSync(path, write(text, ticket, resultText(asked.state, today(), asked.note, links)));
	writeFileSync(INDEX, current());
	return parse(asked.file, readFileSync(path, 'utf8')).tickets.find((one) => one.id === asked.id);
}

/** Keep an attached file. Its name is made safe and given the time. */
async function keep(file: string, ticket: string, name: string, body: ArrayBuffer): Promise<string> {
	if (body.byteLength > LARGEST) throw new Error('larger than 500 MB');
	const extension = name.split('.').pop()?.toLowerCase() ?? '';
	if (!(extension in KINDS)) throw new Error(`.${extension} is not a kind kept here`);
	const dir = folder(file, ticket);
	mkdirSync(dir, { recursive: true });
	const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
	const safe = name.replace(/[^A-Za-z0-9._-]+/g, '-').slice(-80);
	const kept = `${stamp}-${safe}`;
	writeFileSync(join(dir, kept), new Uint8Array(body));
	return `evidence/${file.replace(/-sweep\.md$/, '')}/${ticket}/${kept}`;
}

/** A file under `evidence/`, never anything outside it. */
function served(path: string): Response {
	const root = resolve(EVIDENCE);
	const wanted = resolve(TESTING, decodeURIComponent(path));
	if (!wanted.startsWith(root + sep) || !existsSync(wanted)) return new Response('not found', { status: 404 });
	const kind = KINDS[wanted.split('.').pop()?.toLowerCase() ?? ''] ?? 'application/octet-stream';
	return new Response(Bun.file(wanted), { headers: { 'content-type': kind } });
}

const COOKIE = 'sweep-key';

function same(a: string, b: string): boolean {
	const left = Buffer.from(a);
	const right = Buffer.from(b);
	return left.length === right.length && timingSafeEqual(left, right);
}

export type Let = { pass: true } | { pass: false; to: string } | { pass: false; to: null };

/**
 * With `--phone`, the key or nothing. A request with the key in its
 * address is sent back to the same address without it (`to`), holding the
 * key in a cookie, so the key does not stay in the address bar or in a link
 * copied from it. `to: null` turns it away.
 */
export function admit(address: string, cookies: string | null, key: string): Let {
	const url = new URL(address);
	const given = url.searchParams.get('key');
	if (given !== null && same(given, key)) {
		url.searchParams.delete('key');
		return { pass: false, to: url.pathname + url.search };
	}
	const held = (cookies ?? '')
		.split(';')
		.map((part) => part.trim().split('='))
		.find(([name]) => name === COOKIE)?.[1];
	return held !== undefined && same(held, key) ? { pass: true } : { pass: false, to: null };
}

function gate(request: Request, key: string): Response | null {
	const answer = admit(request.url, request.headers.get('cookie'), key);
	if (answer.pass) return null;
	if (answer.to !== null) {
		return new Response(null, {
			status: 303,
			headers: {
				location: answer.to,
				'set-cookie': `${COOKIE}=${key}; Path=/; HttpOnly; SameSite=Strict; Max-Age=86400`
			}
		});
	}
	return new Response('Open the address the sweep portal printed, with its key.', {
		status: 401,
		headers: { 'content-type': 'text/plain; charset=utf-8' }
	});
}

/** This machine's addresses on the networks it is on, for the phone to use. */
function addresses(): string[] {
	return Object.values(networkInterfaces())
		.flat()
		.filter((one) => one && one.family === 'IPv4' && !one.internal)
		.map((one) => one!.address);
}

function json(value: unknown, status = 200): Response {
	return new Response(JSON.stringify(value), { status, headers: { 'content-type': 'application/json' } });
}

async function handle(request: Request): Promise<Response> {
	const url = new URL(request.url);
	try {
		if (request.method === 'GET' && url.pathname === '/') {
			return new Response(Bun.file(PAGE), { headers: { 'content-type': 'text/html; charset=utf-8' } });
		}
		if (request.method === 'GET' && url.pathname === '/api/sweeps') return json(everything());
		if (request.method === 'GET' && url.pathname.startsWith('/evidence/')) return served(url.pathname.slice(1));
		if (request.method === 'POST' && url.pathname === '/api/result') {
			return json({ ticket: record((await request.json()) as Asked) });
		}
		if (request.method === 'POST' && url.pathname === '/api/evidence') {
			const file = url.searchParams.get('file') ?? '';
			const ticket = url.searchParams.get('id') ?? '';
			const name = url.searchParams.get('name') ?? 'attachment';
			const link = await keep(file, ticket, name, await request.arrayBuffer());
			return json({ link, evidence: evidenceOf(file, ticket) });
		}
		return new Response('not found', { status: 404 });
	} catch (error) {
		return json({ error: error instanceof Error ? error.message : String(error) }, 400);
	}
}

function opened(url: string): void {
	const [program, ...args] =
		process.platform === 'win32'
			? ['cmd', '/c', 'start', '', url]
			: process.platform === 'darwin'
				? ['open', url]
				: ['xdg-open', url];
	spawn(program, args, { stdio: 'ignore', detached: true }).unref();
}

if (import.meta.main) {
	const argv = process.argv.slice(2);
	const asked = argv.indexOf('--port');
	const port = asked >= 0 ? Number(argv[asked + 1]) : 5199;
	if (!existsSync(TESTING)) {
		console.error(`Run it from the repository: ${TESTING} is not here.`);
		process.exit(1);
	}
	writeFileSync(INDEX, current());
	const phone = argv.includes('--phone');
	const key = phone ? randomBytes(18).toString('base64url') : '';
	// This machine only, unless asked: what it writes is the working record.
	const server = Bun.serve({
		hostname: phone ? '0.0.0.0' : '127.0.0.1',
		port,
		fetch: phone ? (request) => gate(request, key) ?? handle(request) : handle,
		maxRequestBodySize: LARGEST
	});
	const url = `http://127.0.0.1:${server.port}/${phone ? `?key=${key}` : ''}`;
	console.log(`Sweep portal: ${url}  (Ctrl+C to stop)`);
	if (phone) {
		console.log('On a phone on the same network, open one of these:');
		for (const address of addresses()) console.log(`  http://${address}:${server.port}/?key=${key}`);
	}
	if (!argv.includes('--no-open')) opened(url);
}
