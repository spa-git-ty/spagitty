// SPDX-License-Identifier: GPL-3.0-or-later

import { readFileSync, readdirSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { beforeEach, describe, expect, it } from 'vitest';
import { click, render } from '../../testing/mount';
import type { FileDiff, PullRequest } from '$lib/types';

import PRDiffPane from './PRDiffPane.svelte';
import RequestDetail from './RequestDetail.svelte';
import RequestRow from './RequestRow.svelte';
import { CHECK_LABELS, REVIEW_LABELS, requests } from './store.svelte';

function request(overrides: Partial<PullRequest> = {}): PullRequest {
	return {
		id: 'pr-1',
		number: 412,
		title: 'Split the diff view',
		body: 'Description text',
		authorName: 'Ada Lovelace',
		updated: Math.floor(Date.now() / 1000) - 3600,
		sourceBranch: 'feature/split-view',
		targetBranch: 'main',
		draft: false,
		review: 'awaitingReview',
		checks: 'passing',
		needsYou: true,
		needsYouBecause: 'Your review was requested.',
		changedFiles: 4,
		added: 120,
		removed: 8,
		mergeable: true,

		headSha: 'a1b2c3d4e5f6',

		reviewRequested: false,

		openThreads: 0,

		resolvedThreads: 0,

		repliesToYou: 0,

		repository: null,
		...overrides
	};
}

beforeEach(() => {
	requests.clear();
});

describe('grouping', () => {
	it('leads with what is waiting on you', () => {
		requests.present([
			request({ id: 'a', needsYou: false }),
			request({ id: 'b', needsYou: true })
		]);

		expect(requests.needingYou.map((r) => r.id)).toEqual(['b']);
		expect(requests.waitingOnOthers.map((r) => r.id)).toEqual(['a']);
	});

	it('puts every request in exactly one group', () => {
		requests.present([
			request({ id: 'a', needsYou: true }),
			request({ id: 'b', needsYou: false }),
			request({ id: 'c', needsYou: true })
		]);

		expect(requests.needingYou.length + requests.waitingOnOthers.length).toBe(
			requests.all.length
		);
	});

	it('opens the first request so the panel is never blank beside a list', () => {
		requests.present([request({ id: 'a' }), request({ id: 'b' })]);

		expect(requests.openId).toBe('a');
		expect(requests.open?.id).toBe('a');
	});

	it('keeps the open request across a refresh when it is still there', () => {
		requests.present([request({ id: 'a' }), request({ id: 'b' })]);
		requests.select('b');

		requests.present([request({ id: 'a' }), request({ id: 'b' })]);

		expect(requests.openId).toBe('b');
	});

	it('falls back to the first when the open one was closed elsewhere', () => {
		requests.present([request({ id: 'a' }), request({ id: 'b' })]);
		requests.select('b');

		requests.present([request({ id: 'a' })]);

		expect(requests.openId).toBe('a');
	});
});

describe('the state before a host exists', () => {
	it('starts with no account connected, which is the only state in this pass', () => {
		expect(requests.connected).toBe(false);
		expect(requests.all).toEqual([]);
	});

	it('records a failure to reach a host and shows nothing', () => {
		requests.present([request()]);

		requests.fail('could not reach the host');

		expect(requests.error).toContain('could not reach');
		expect(requests.all).toEqual([]);
		expect(requests.openId).toBeNull();
	});

	it('clearing forgets the list and the account', () => {
		requests.present([request()]);

		requests.clear();

		expect(requests.connected).toBe(false);
		expect(requests.all).toEqual([]);
		expect(requests.openId).toBeNull();
	});
});

describe('RequestRow', () => {
	it('names the request, its number, its author and when it moved', () => {
		requests.present([request()]);
		const view = render(RequestRow, { request: request() });

		expect(view.text()).toContain('#412');
		expect(view.text()).toContain('Split the diff view');
		expect(view.text()).toContain('Ada Lovelace');
		view.destroy();
	});

	it('says why it needs you, when it does', () => {
		const view = render(RequestRow, { request: request() });

		expect(view.text()).toContain('Your review was requested');
		view.destroy();
	});

	it('says nothing about why when it is somebody else\'s move', () => {
		const view = render(RequestRow, {
			request: request({ needsYou: false, needsYouBecause: null }),
			waiting: true
		});

		expect(view.all('.because')).toHaveLength(0);
		view.destroy();
	});

	it('renders a waiting row dashed', () => {
		const view = render(RequestRow, {
			request: request({ needsYou: false }),
			waiting: true
		});

		expect(view.all('.row.waiting')).toHaveLength(1);
		view.destroy();
	});

	it('shows review and check state, and a draft as a draft', () => {
		const view = render(RequestRow, {
			request: request({ draft: true, review: 'changesRequested', checks: 'failing' })
		});

		expect(view.text()).toContain('draft');
		expect(view.text()).toContain('changes requested');
		expect(view.text()).toContain('checks failing');
		view.destroy();
	});

	it('marks a pull request that conflicts with its base (BUG-063)', () => {
		const view = render(RequestRow, { request: request({ mergeable: false }) });
		expect(view.text()).toContain('conflicts');
		view.destroy();

		const unknown = render(RequestRow, { request: request({ mergeable: null }) });
		expect(unknown.text()).not.toContain('conflicts');
		unknown.destroy();
	});

	it('says nothing about checks when the host runs none', () => {
		const view = render(RequestRow, { request: request({ checks: null }) });

		expect(view.text()).not.toContain('checks');
		view.destroy();
	});

	it('opens a request from its row', () => {
		requests.present([request({ id: 'a' }), request({ id: 'b' })]);
		const view = render(RequestRow, { request: request({ id: 'b' }) });

		click(view.get('.body'));

		expect(requests.openId).toBe('b');
		view.destroy();
	});

	it('marks the open row', () => {
		requests.present([request({ id: 'a' })]);
		const view = render(RequestRow, { request: request({ id: 'a' }) });

		expect(view.all('.row.open')).toHaveLength(1);
		view.destroy();
	});
});

describe('RequestDetail', () => {
	it('says what it is for before a request is opened', () => {
		const view = render(RequestDetail, {});

		expect(view.text()).toContain('Open a pull request');
		view.destroy();
	});

	it('shows the branches, the counts and the state', () => {
		requests.present([request()]);
		const view = render(RequestDetail, {});

		expect(view.text()).toContain('feature/split-view');
		expect(view.text()).toContain('main');
		expect(view.text()).toContain('4 files');
		expect(view.text()).toContain('+120');
		expect(view.text()).toContain('awaiting review');
		view.destroy();
	});

	it('says when the host reports it cannot merge', () => {
		requests.present([request({ mergeable: false })]);
		const view = render(RequestDetail, {});

		expect(view.text()).toContain('conflicts');
		view.destroy();
	});

	it('offers Review, and still says plainly that merging is not built', () => {
		// This assertion used to be "every action is disabled", which was true
		// while Spagitty talked to no host about one pull request. FEAT-058
		// built the review, so the rule it was protecting is the one that
		// survives: a control that looks live and does nothing is worse than one
		// that explains itself.
		requests.present([request()]);
		const view = render(RequestDetail, {});

		const buttons = view.all('button') as HTMLButtonElement[];
		const enabled = buttons.filter((button) => !button.disabled);
		const disabled = buttons.filter((button) => button.disabled);

		// Something is live now, and it is the review.
		expect(enabled.length).toBeGreaterThan(0);
		expect(view.text()).toContain('Review');

		// Whatever is still dead says why, in words rather than an identifier
		// only this project's own record can resolve.
		for (const button of disabled) {
			expect(button.title).not.toBe('');
			expect(button.title).not.toMatch(/FEAT-\d/);
		}
		view.destroy();
	});
});

describe('PRDiffPane', () => {
	const file: FileDiff = {
		path: 'src/Playback.kt',
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 2,
		removed: 1,
		hunks: [
			{
				oldStart: 1,
				oldLines: 2,
				newStart: 1,
				newLines: 3,
				header: '@@ -1,2 +1,3 @@',
				lines: [
					{ origin: 'context', old: 1, new: 1, text: '/* a comment' },
					{ origin: 'removed', old: 2, new: null, text: '   that ends here */' },
					{ origin: 'added', old: null, new: 2, text: '   that ends */' },
					{ origin: 'added', old: null, new: 3, text: 'val code = "x"' }
				]
			}
		]
	};

	for (const view of ['unified', 'split'] as const) {
		it(`colours the code in the ${view} view`, () => {
			const pane = render(PRDiffPane, { file, path: file.path, error: null, loading: false, view });

			const coloured = (kind: string) => pane.all(`.tok-${kind}`).map((span) => span.textContent);
			expect(coloured('keyword')).toContain('val');
			expect(coloured('string')).toContain('"x"');
			// A comment opened on one line stays a comment on the next.
			expect(coloured('comment')).toContain('   that ends */');
			pane.destroy();
		});
	}
});

describe('the promises this screen makes', () => {
	/** Every source file the screen is built from. */
	function sources(): string[] {
		const here = join(process.cwd(), 'src/lib/requests');
		const files = readdirSync(here)
			.filter((name) => !name.endsWith('.test.ts'))
			.map((name) => readFileSync(join(here, name), 'utf8'));
		files.push(readFileSync(join(process.cwd(), 'src/routes/requests/+page.svelte'), 'utf8'));
		return files;
	}

	it('uses no host\'s name anywhere in the screen', () => {
		// This is the kind of thing that rots the moment somebody adds "Open on
		// <host>" without thinking, so it is asserted rather than intended.
		const brands = ['GitHub', 'GitLab', 'Bitbucket', 'Gitea', 'Forgejo', 'Azure DevOps'];

		for (const source of sources()) {
			for (const brand of brands) {
				expect(source).not.toContain(brand);
			}
		}
	});

	it('makes no network call from the webview, in any form', () => {
		// The claim narrowed when FEAT-017 landed, and it is worth being exact
		// about how. The application does talk to a host now — but not from
		// here. The webview asks the backend, the backend makes one request in
		// one file, and the token never crosses into JavaScript at all.
		//
		// A `fetch` in a screen would put a bearer token in the devtools network
		// tab of anybody who opened it.
		for (const source of sources()) {
			expect(source).not.toMatch(/\bfetch\s*\(/);
			expect(source).not.toContain('XMLHttpRequest');
			expect(source).not.toContain('WebSocket');
			expect(source).not.toMatch(/https?:\/\//);
		}
	});

	it('holds no token, anywhere in the screen', () => {
		// The store, the rows, the detail panel and the route. A token reaches
		// the OS keychain through the backend and is never read back out, so
		// nothing here should so much as name one.
		for (const source of sources()) {
			expect(source).not.toMatch(/\btoken\b/i);
			expect(source).not.toMatch(/\bAuthorization\b/);
			expect(source).not.toMatch(/\bBearer\b/);
		}
	});

	it('links no HTTP client into the webview', () => {
		// Unchanged by FEAT-017, and the half of the old promise that still
		// holds absolutely. The frontend has no way to reach a network.
		const npm = JSON.parse(readFileSync(join(process.cwd(), 'package.json'), 'utf8'));
		const declared = Object.keys({
			...(npm.dependencies ?? {}),
			...(npm.devDependencies ?? {})
		});

		for (const name of ['axios', 'node-fetch', 'got', 'undici', 'ky', 'superagent']) {
			expect(declared).not.toContain(name);
		}
	});

	it('links exactly one HTTP client into the application, and only in the core', () => {
		// What the old test became (FEAT-017). It used to forbid every HTTP
		// client in both languages; forbidding them outright stopped being
		// possible the moment pull requests were read, so it now pins the thing
		// that actually matters — that there is *one*, that it is the one that
		// was chosen, and that the Tauri layer did not quietly grow a second.
		const permitted = 'ureq';
		const clients = ['reqwest', 'ureq', 'hyper', 'isahc', 'attohttpc', 'curl', 'surf'];

		const core = readFileSync(join(process.cwd(), 'crates/spagitty-core/Cargo.toml'), 'utf8');
		const declaredInCore = clients.filter((crate) =>
			new RegExp(`^\\s*${crate}\\s*=`, 'm').test(core)
		);
		expect(declaredInCore).toEqual([permitted]);

		for (const manifest of ['Cargo.toml', 'src-tauri/Cargo.toml']) {
			const text = readFileSync(join(process.cwd(), manifest), 'utf8');
			for (const crate of clients) {
				expect(text, `${manifest} should link no HTTP client`).not.toMatch(
					new RegExp(`^\\s*${crate}\\s*=`, 'm')
				);
			}
		}
	});

	it('makes its requests from exactly one file', () => {
		// The same argument `shell.rs` makes about spawning a process: one call
		// site means "what does this send, and where" has one answer somebody
		// can read in an afternoon. A second would have to be added here.
		const core = join(process.cwd(), 'crates/spagitty-core/src');
		const rust: string[] = [];
		const walk = (dir: string) => {
			for (const entry of readdirSync(dir, { withFileTypes: true })) {
				const path = join(dir, entry.name);
				if (entry.isDirectory()) walk(path);
				else if (entry.name.endsWith('.rs')) rust.push(path);
			}
		};
		walk(core);

		const touching = rust.filter((path) => /\bureq\b/.test(readFileSync(path, 'utf8')));

		expect(touching.map((path) => relative(core, path).split(sep).join('/'))).toEqual([
			'forge/http.rs'
		]);
	});

	it('names every state in words a user of any host would recognise', () => {
		expect(Object.values(REVIEW_LABELS)).toEqual([
			'awaiting review',
			'changes requested',
			'approved',
			'no reviewers'
		]);
		expect(Object.values(CHECK_LABELS)).toEqual([
			'checks passing',
			'checks failing',
			'checks running'
		]);
	});
});
