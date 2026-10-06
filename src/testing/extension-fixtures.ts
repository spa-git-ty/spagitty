// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Extension listings for the frontend tests (FEAT-096). Scaffolding, outside
 * `src/lib`, like the other fixtures here.
 */

import type { ExtensionView, Listing, Manifest, ReviewRecord } from '$lib/extensions/types';
import record from '../../schemas/extensions/fixtures/review-record.json';

export function manifest(overrides: Partial<Manifest> = {}): Manifest {
	return {
		manifestVersion: 1,
		id: 'com.example.hello',
		name: 'Hello',
		version: '1.0.0',
		publisher: 'Example',
		engines: { spagitty: '>=0.9.0', extensionApi: '^1.0.0' },
		runtime: { kind: 'native-process', entrypoints: { 'x86_64-pc-windows-msvc': 'bin/hello.exe' } },
		capabilities: { required: ['repository.read', 'review.provide'], optional: ['forge.pullRequest.read'] },
		contributes: {
			commands: [
				{ id: 'hello', title: 'Say hello', context: 'workingCopy' },
				{ id: 'review', title: 'Review changes', context: 'workingCopy', reviewProvider: 'review' },
				{ id: 'task', title: 'Review task', context: 'farmTask', reviewProvider: 'review' },
				{ id: 'hidden', title: 'Palette only', menu: false },
				{ id: 'quiet', title: 'Menu only', context: 'workingCopy', palette: false, when: ['hasWorkingChanges'] }
			],
			reviewProviders: [{ id: 'review', targets: ['workingCopy', 'farmTask'], sendsCodeTo: 'Example Cloud' }],
			panels: [
				{ id: 'findings', title: 'Findings', renderer: 'reviewFindings' },
				{ id: 'about', title: 'About', renderer: 'summary', location: 'workingCopy' },
				{ id: 'pr', title: 'Hello on PRs', renderer: 'reviewStatus' }
			],
			settings: [
				{ key: 'region', type: 'enum', values: ['us', 'eu'], default: 'us' },
				{ key: 'mode', type: 'enum', values: ['off', 'required'], scope: 'repository' }
			]
		},
		...overrides
	};
}

export function extension(overrides: Partial<ExtensionView> = {}): ExtensionView {
	const m = overrides.manifest ?? manifest();
	return {
		id: m.id,
		name: m.name,
		version: m.version,
		publisher: m.publisher,
		description: 'Says hello.',
		license: 'MIT',
		homepage: null,
		provenance: 'local',
		official: false,
		state: 'installed',
		stateReason: null,
		compatibility: { compatible: true, reasons: [] },
		enabled: true,
		consented: true,
		sendsCodeTo: ['Example Cloud'],
		capabilities: [
			{ capability: 'repository.read', required: true, granted: true, description: 'Read this repository' },
			{ capability: 'review.provide', required: true, granted: true, description: 'Show review findings' },
			{ capability: 'forge.pullRequest.read', required: false, granted: false, description: 'Read pull requests' }
		],
		manifest: m,
		settings: [
			{ key: 'region', type: 'enum', values: ['us', 'eu'], default: 'us', value: 'us' },
			{ key: 'mode', type: 'enum', values: ['off', 'required'], scope: 'repository', value: 'off' }
		],
		tools: [],
		unavailable: [],
		previousVersion: null,
		diagnostics: { stderr: '', logs: [], toolRuns: [], error: null, program: null },
		running: 0,
		...overrides
	};
}

export function listing(extensions: ExtensionView[] = [extension()]): Listing {
	return { extensions, problems: [], trust: 'Extensions run with your permissions.', target: 'x86_64-pc-windows-msvc' };
}

/** The shared contract fixture, as the host writes it. */
export function reviewRecord(): ReviewRecord {
	return structuredClone(record) as ReviewRecord;
}
