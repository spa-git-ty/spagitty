// SPDX-License-Identifier: GPL-3.0-or-later

import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ErrorCode, isMessage } from './protocol';
import { parseManifest, validate } from './manifest';
import { inspect, pack } from './package';
import { FakeHost } from './testing/index';
import { satisfies } from './version';
import { defineExtension, HostError, Peer } from './worker';
import { crc32, readZip, writeZip } from './zip';

const fixtures = join(process.cwd(), 'schemas/extensions/fixtures');

describe('the manifest rules agree with the host', () => {
	it('accepts every valid fixture', () => {
		for (const name of readdirSync(join(fixtures, 'manifests/valid'))) {
			const text = readFileSync(join(fixtures, 'manifests/valid', name), 'utf8');
			expect(() => parseManifest(text), name).not.toThrow();
		}
	});

	it('refuses every invalid fixture at the field the host names', () => {
		const names = readdirSync(join(fixtures, 'manifests/invalid'));
		expect(names.length).toBeGreaterThanOrEqual(10);
		for (const name of names) {
			const { expect: path, manifest } = JSON.parse(readFileSync(join(fixtures, 'manifests/invalid', name), 'utf8'));
			const errors = validate(manifest);
			expect(errors.some((e) => e.startsWith(path)), `${name}: ${errors.join(' | ')}`).toBe(true);
		}
	});

	it('reports every broken rule at once', () => {
		const errors = validate({ manifestVersion: 1, id: 'Bad', name: '', version: 'x', publisher: 'p', colour: 1 });
		expect(errors.some((e) => e.startsWith('id:'))).toBe(true);
		expect(errors.some((e) => e.startsWith('name:'))).toBe(true);
		expect(errors.some((e) => e.startsWith('version:'))).toBe(true);
		expect(errors.some((e) => e.startsWith('colour:'))).toBe(true);
		expect(errors.some((e) => e.startsWith('engines:'))).toBe(true);
	});
});

describe('version requirements, as the host reads them', () => {
	it('treats the Cargo and npm spellings alike', () => {
		for (const range of ['>=0.9.0, <1.0.0', '>=0.9.0 <1.0.0', '>= 0.9.0 , < 1.0.0']) {
			expect(satisfies(range, '0.9.0'), range).toBe(true);
			expect(satisfies(range, '0.9.7'), range).toBe(true);
			expect(satisfies(range, '1.0.0'), range).toBe(false);
			expect(satisfies(range, '0.8.1'), range).toBe(false);
		}
	});

	it('follows caret and tilde', () => {
		expect(satisfies('^1.0.0', '1.4.2')).toBe(true);
		expect(satisfies('^1.0.0', '2.0.0')).toBe(false);
		expect(satisfies('1.0', '1.9.0')).toBe(true);
		expect(satisfies('~0.9', '0.9.5')).toBe(true);
		expect(satisfies('~0.9', '0.10.0')).toBe(false);
		expect(satisfies('^0.2.3', '0.2.9')).toBe(true);
		expect(satisfies('^0.2.3', '0.3.0')).toBe(false);
	});

	it('keeps pre-releases out unless a comparator names one', () => {
		expect(satisfies('>=0.9.0', '0.9.0-alpha.1')).toBe(false);
		expect(satisfies('>=0.9.0-alpha.1', '0.9.0-alpha.2')).toBe(true);
		expect(satisfies('whenever', '1.0.0')).toBe(false);
		expect(satisfies('>=1.0.0', 'one')).toBe(false);
	});
});

describe('a worker written with the SDK', () => {
	it('checks local readiness through callbacks and refuses unsupported providers', async () => {
		const host = FakeHost.inMemory(defineExtension({ checkReview: async (ctx) => ({ ready: (await ctx.host.detectTool('tool')).found, providerVersion: '1' }) }), { services: { 'tools.detect': () => ({ found: true }) } });
		await host.start();
		expect(await host.checkReview('review')).toEqual({ ready: true, providerVersion: '1' });
		const unsupported = FakeHost.inMemory(defineExtension({}));
		await unsupported.start();
		await expect(unsupported.checkReview('review')).rejects.toMatchObject({ code: ErrorCode.MethodNotFound });
	});
	const extension = defineExtension({
		activate: () => ({ unavailable: [{ id: 'later', reason: 'not yet' }] }),
		commands: {
			async hello(ctx) {
				ctx.progress('Saying hello');
				await ctx.host.notify(`Hello ${String(ctx.settings.name ?? 'there')}`);
				await ctx.host.storageSet('greeted', true);
				return { message: `stored ${String(await ctx.host.storageGet('greeted'))}` };
			},
			async boom() {
				throw new Error('it broke');
			},
			async wait(ctx) {
				await new Promise<void>((resolve) => ctx.signal.addEventListener('abort', () => resolve()));
				return { message: 'stopped' };
			},
			async refused(ctx) {
				try {
					await ctx.host.describeRepository('repo:404');
					return { message: 'no error' };
				} catch (error) {
					return { message: `refused ${(error as HostError).code}` };
				}
			}
		},
		reviewProviders: {
			review(ctx) {
				ctx.findings([{ id: 'a', severity: 'low', title: 'T', message: 'M' }]);
				return { status: 'completed', completeness: 'complete', summary: `scope ${ctx.snapshot.scope}`, providerVersion: '1' };
			}
		},
		panels: {
			about: () => ({ title: 'About', rows: [{ label: 'x', value: 1 }] })
		}
	});

	it('answers the handshake and reports what cannot work', async () => {
		const host = FakeHost.inMemory(extension);
		const activated = await host.start();
		expect(activated).toEqual({ unavailable: [{ id: 'later', reason: 'not yet' }] });
		expect(host.violations).toEqual([]);
	});

	it('completes a command exactly once, with its callbacks answered', async () => {
		const host = FakeHost.inMemory(extension, { settings: { name: 'Ada' } });
		await host.start();
		const outcome = await host.command('hello');
		expect(outcome).toMatchObject({ status: 'completed', message: 'stored true', progress: ['Saying hello'] });
		expect(host.notices).toEqual([{ level: 'info', message: 'Hello Ada' }]);
		expect(host.violations).toEqual([]);
	});

	it('turns a thrown error into a failure and a cancellation into cancelled', async () => {
		const host = FakeHost.inMemory(extension);
		await host.start();
		expect((await host.command('boom')).status).toBe('failed');
		const { operationId, done } = await host.startCommand('wait');
		host.cancel(operationId);
		expect((await done).status).toBe('cancelled');
		expect(host.violations).toEqual([]);
	});

	it('surfaces a host refusal with its code', async () => {
		const host = FakeHost.inMemory(extension);
		await host.start();
		expect((await host.command('refused')).message).toBe(`refused ${ErrorCode.NotGranted}`);
	});

	it('runs a review and resolves a panel', async () => {
		const host = FakeHost.inMemory(extension);
		await host.start();
		const outcome = await host.review('review', { scope: 'committed' });
		expect(outcome.findings).toHaveLength(1);
		expect(outcome.review).toMatchObject({ status: 'completed', summary: 'scope committed' });
		expect(await host.panel('about')).toEqual({ title: 'About', rows: [{ label: 'x', value: 1 }] });
		expect(host.violations).toEqual([]);
	});

	it('refuses an unknown command with method-not-found', async () => {
		const host = FakeHost.inMemory(extension);
		await host.start();
		await expect(host.command('nope')).rejects.toMatchObject({ code: ErrorCode.MethodNotFound });
	});
});

describe('messages', () => {
	it('knows a JSON-RPC message from something else', () => {
		expect(isMessage({ jsonrpc: '2.0', method: 'x' })).toBe(true);
		expect(isMessage({ jsonrpc: '2.0', id: 1, result: {} })).toBe(true);
		expect(isMessage({ jsonrpc: '2.0', id: 1, result: {}, error: {} })).toBe(false);
		expect(isMessage({ id: 1, result: {} })).toBe(false);
		expect(isMessage([])).toBe(false);
	});
});

describe('packages', () => {
	let dir = '';
	afterEach(() => {
		if (dir) rmSync(dir, { recursive: true, force: true });
		dir = '';
	});

	function project(entrypoints: Record<string, string>, files: Record<string, Uint8Array | string>) {
		dir = mkdtempSync(join(tmpdir(), 'ext-'));
		writeFileSync(
			join(dir, 'extension.json'),
			JSON.stringify({
				manifestVersion: 1,
				id: 'com.example.packed',
				name: 'Packed',
				version: '1.0.0',
				publisher: 'Example',
				license: 'MIT',
				engines: { spagitty: '>=0.9.0', extensionApi: '^1.0.0' },
				runtime: { kind: 'native-process', entrypoints }
			})
		);
		for (const [name, data] of Object.entries(files)) {
			mkdirSync(join(dir, ...name.split('/').slice(0, -1)), { recursive: true });
			writeFileSync(join(dir, ...name.split('/')), data);
		}
		return dir;
	}

	it('round-trips a ZIP with checked CRCs', () => {
		const data = new TextEncoder().encode('hello');
		expect(crc32(data)).toBe(0x3610a686);
		const [entry] = readZip(writeZip([{ name: 'a.txt', data, mode: 0o100644 }]));
		expect(new TextDecoder().decode(entry.data)).toBe('hello');
		expect(entry.mode).toBe(0o100644);
	});

	it('packs only the targets that were built, and inspects clean', () => {
		const elf = new Uint8Array([0x7f, 0x45, 0x4c, 0x46, 1, 2, 3]);
		project(
			{ 'x86_64-unknown-linux-gnu': 'bin/linux-x64/packed', 'aarch64-apple-darwin': 'bin/macos-arm64/packed' },
			{ 'bin/linux-x64/packed': elf, 'README.md': '# hi', 'src/main.ts': 'source is not shipped' }
		);
		const packed = pack(dir);
		expect(packed.targets).toEqual(['x86_64-unknown-linux-gnu']);
		expect(packed.dropped).toEqual(['aarch64-apple-darwin']);
		expect(packed.files.map((f) => f.path)).toEqual(['README.md', 'bin/linux-x64/packed', 'extension.json']);
		const report = inspect(packed.bytes);
		expect(report.problems).toEqual([]);
		expect(Object.keys(report.manifest.runtime.entrypoints)).toEqual(['x86_64-unknown-linux-gnu']);
	});

	it('refuses an undeclared program and a package with nothing built', () => {
		project({ 'x86_64-unknown-linux-gnu': 'bin/x' }, { 'bin/x': 'x', 'helper.exe': 'MZ' });
		expect(() => pack(dir)).toThrow(/does not declare/);
		rmSync(dir, { recursive: true, force: true });
		project({ 'x86_64-unknown-linux-gnu': 'bin/missing' }, {});
		expect(() => pack(dir)).toThrow(/no program is built/);
	});

	it('finds tampering on inspection', () => {
		project({ 'x86_64-unknown-linux-gnu': 'bin/x' }, { 'bin/x': new Uint8Array([0x7f, 0x45, 0x4c, 0x46]) });
		const packed = pack(dir);
		const entries = readZip(packed.bytes).map((e) => (e.name === 'bin/x' ? { ...e, data: new Uint8Array([0x7f, 0x45, 0x4c, 0x46, 9]) } : e));
		expect(inspect(writeZip(entries)).problems).toContain('bin/x does not match integrity.json');
	});
});

describe('public SDK callbacks and transport boundaries', () => {
 it('preserves uncertain comment receipts, operation ownership and settings changes', async () => {
  let sent: unknown;const changed=vi.fn();
  const h=FakeHost.inMemory(defineExtension({settingsChanged:changed,commands:{request:async ctx=>{
   ctx.heartbeat();ctx.log('requesting');
   const snapshot=await ctx.host.pullRequestSnapshot('repo:1',7);
   const receipt=await ctx.host.commentOnPullRequest('repo:1',snapshot.number,'@coderabbitai review',ctx.operationId);
   return {message:receipt.status+' '+ctx.settings.region};
  }}}),{settings:{region:'us'},services:{'forge.pullRequest.snapshot':()=>({number:7}),'forge.pullRequest.comment':p=>{sent=p;return{status:'uncertain',headSha:'head',message:'Refresh before sending again'};}}});
  await h.start();h.changeSettings({region:'eu'});
  expect((await h.command('request')).message).toBe('uncertain eu');
  expect(sent).toMatchObject({repository:'repo:1',number:7,operationId:expect.stringMatching(/^op-/)});
  expect(changed).toHaveBeenCalledWith({region:'eu'});expect(h.logs).toContain('[info] requesting');
  await h.stop();
 });
 it('a missing tool fails the operation instead of reporting a successful run', async () => {
  const h=FakeHost.inMemory(defineExtension({commands:{check:async ctx=>{await ctx.host.runTool({operationId:ctx.operationId,tool:'cli',profile:'inspect'},()=>{});}}}));
  await h.start();expect((await h.command('check')).status).toBe('failed');expect(h.toolCalls).toHaveLength(1);
  await expect(h.panel('missing')).rejects.toMatchObject({code:ErrorCode.MethodNotFound});
  await expect(h.review('missing')).rejects.toMatchObject({code:ErrorCode.MethodNotFound});
  await h.stop();
 });
 it('a closed channel rejects waiting callbacks, malformed and late answers cannot resolve them', async () => {
  let receive!: (line:string)=>void;let close!: ()=>void;const writes:string[]=[];
  const peer=new Peer({onLine:h=>{receive=h;},onClose:h=>{close=h;},write:l=>writes.push(l)},'w',async()=>({}),()=>{});
  const waiting=peer.call('repository.describe',{});
  receive('not json');receive('');receive(JSON.stringify({jsonrpc:'2.0',id:'foreign',result:{}}));
  close();await expect(waiting).rejects.toMatchObject({code:ErrorCode.NotActive});
  receive(JSON.stringify({jsonrpc:'2.0',id:JSON.parse(writes[0]).id,result:{}}));
  expect(()=>peer.notify('log',{message:'😀'.repeat(300_000)})).toThrow('message too large');
 });
});
