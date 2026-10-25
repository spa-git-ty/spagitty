// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * FEAT-165: every setting in Settings › Calls is read by the code it names.
 *
 * A switch nothing reads is worse than no switch (BehaviourSection's rule),
 * so each field of `CallsSettings` is listed here with the file that reads
 * it and what that file says when it does. A field whose reader is a later
 * item names that item instead, and the test fails once the item is Done
 * and the field is still not read.
 */

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = process.cwd();
const read = (path: string) => readFileSync(join(ROOT, path), 'utf8');

type Reader = { file: string; reads: RegExp } | { pending: string };

const CORE = 'crates/spagitty-core/src/calls/settings.rs';
const APP = 'src-tauri/src/calls.rs';
const SCREEN = 'src/lib/calls/CallsScreen.svelte';

const READERS: Record<string, Reader> = {
	folder: { file: APP, reads: /taken\.folder/ },
	openFound: { pending: 'FEAT-162' },
	lookFor: { pending: 'FEAT-162' },
	scanOnOpen: { pending: 'FEAT-162' },
	frameworks: { pending: 'FEAT-162' },
	skipPaths: { pending: 'FEAT-162' },
	clickToKeep: { file: SCREEN, reads: /settings\?\.clickToKeep/ },
	keepEvery: { file: SCREEN, reads: /settings\?\.keepEvery/ },
	suggestNames: { file: SCREEN, reads: /settings\?\.suggestNames/ },
	timeoutSeconds: { file: CORE, reads: /self\.timeout_seconds/ },
	followRedirects: { file: CORE, reads: /self\.follow_redirects/ },
	checkTls: { file: CORE, reads: /self\.check_tls/ },
	proxy: { file: CORE, reads: /self\.proxy/ },
	clientCerts: { file: CORE, reads: /self\s*\.client_certs/ },
	beautify: { file: SCREEN, reads: /settings\?\.beautify/ },
	everyHeader: { file: CORE, reads: /\.every_header\b/ },
	soapVersion: { pending: 'FEAT-163' },
	envelopeFromWsdl: { pending: 'FEAT-163' },
	wsSecurity: { pending: 'FEAT-163' },
	faultAsError: { pending: 'FEAT-163' },
	history: { file: APP, reads: /settings\.history/ },
	shareAs: { pending: 'FEAT-164' }
};

/** The fields of `CallsSettings`, from the Rust struct, in camelCase. */
function fields(): string[] {
	const text = read(CORE);
	const body = /pub struct CallsSettings \{([\s\S]*?)\n\}/.exec(text)?.[1] ?? '';
	return [...body.matchAll(/^\s*pub (\w+):/gm)].map((match) =>
		match[1].replace(/_(\w)/g, (_, letter: string) => letter.toUpperCase())
	);
}

function status(item: string): string {
	const file = readdirSync(join(ROOT, 'agile/items')).find((name) => name.startsWith(`${item}-`));
	const text = file ? read(`agile/items/${file}`) : '';
	return /\*\*Status:\*\* (\w+)/.exec(text)?.[1] ?? 'missing';
}

describe('Settings › Calls', () => {
	it('names a reader for every setting', () => {
		const all = fields();
		expect(all.length).toBeGreaterThan(20);
		expect(Object.keys(READERS).sort()).toEqual([...all].sort());
	});

	it('has every setting read where it says', () => {
		for (const [key, reader] of Object.entries(READERS)) {
			if ('pending' in reader) continue;
			expect(reader.reads.test(read(reader.file)), `${key} in ${reader.file}`).toBe(true);
		}
	});

	it('leaves a setting unread only while its item is not done', () => {
		for (const [key, reader] of Object.entries(READERS)) {
			if (!('pending' in reader)) continue;
			expect(['Backlog', 'Open', 'Partial'], `${key} waits on ${reader.pending}`).toContain(status(reader.pending));
		}
	});
});
