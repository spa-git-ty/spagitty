// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Settings screen's state.
 *
 * Three unrelated things live behind one screen, and they are kept apart here
 * because they are stored in three different places: the identity is git's own
 * configuration, the toggles are Spagitty's preferences file, and the build
 * identity and license list are compiled in. A failure in one must not blank the
 * others — About in particular, since the license and the commit are an
 * obligation rather than a convenience.
 *
 * Nothing here needs an open repository. Without one the local scope is neither
 * offered nor written, and every other section is unaffected.
 */

import * as api from '../api';
import type {
	About,
	Identity,
	IdentityKey,
	IdentityScope,
	Licenses,
	ForgeAccount,
	Settings,
	Signing,
	Update
} from '../types';

export type Section =
	| 'you'
	| 'remotes'
	| 'tools'
	| 'behaviour'
	| 'personality'
	| 'godmode'
	| 'appearance'
	| 'license';

/** The settings that are a yes or a no, and so can be flipped. */
export type BooleanSetting = {
	[K in keyof Settings]: Settings[K] extends boolean ? K : never;
}[keyof Settings];

/** The settings that are one of several named values. */
export type ChoiceSetting = Exclude<keyof Settings, BooleanSetting>;

/** The chip index, in the order it is shown. */
export const SECTIONS: { id: Section; label: string }[] = [
	{ id: 'you', label: 'You' },
	{ id: 'remotes', label: 'Remotes' },
	{ id: 'tools', label: 'External Tools' },
	{ id: 'behaviour', label: 'Behaviour' },
	{ id: 'personality', label: 'Personality' },
	{ id: 'godmode', label: 'God mode' },
	{ id: 'appearance', label: 'Appearance' },
	{ id: 'license', label: 'License' }
];

/**
 * What the toggles are before the first read resolves, and in a plain browser.
 *
 * These mirror `Default for Settings` in `src-tauri/src/settings.rs`, which is
 * the authority: every value the screen shows after `load()` came from there.
 * They exist so the first frame is not blank, not as a second definition.
 */
const DEFAULTS: Settings = {
	checkForUpdates: true,
	confirmHistoryRewrite: true,
	showGitCommands: false,
	pruneOnFetch: false,
	fetchAvatars: true,
	personality: 'balanced',
	sound: 'off'
};

function isSection(value: string): value is Section {
	return SECTIONS.some((section) => section.id === value);
}

let section = $state<Section>('you');
let identity = $state<Identity | null>(null);
/**
 * Commit signing, read from git rather than from the preferences file.
 *
 * Beside the identity because it is the same kind of thing and shares the same
 * scope chip: both are git configuration with a global value and a repository
 * override, and choosing the scope once for both is the honest arrangement.
 */
let signing = $state<Signing | null>(null);
/** Connected hosting accounts (FEAT-017). Hosts and logins; never tokens. */
let accounts = $state<ForgeAccount[]>([]);
/** What the last update check found, and what it failed with. */
let update = $state<Update | null>(null);
let updateError = $state<string | null>(null);
let checking = $state(false);
let scope = $state<IdentityScope>('global');
let drafts = $state<Record<IdentityKey, string>>({ name: '', email: '' });
let stored = $state<Settings>(DEFAULTS);
let licenses = $state<Licenses | null>(null);
let about = $state<About | null>(null);
let loaded = $state(false);
let busy = $state(false);
/** What the last read failed with. */
let error = $state<string | null>(null);
/**
 * What the last write failed with, kept apart from `error`.
 *
 * Every write is followed by a re-read, and a successful re-read clears
 * `error` — so a failure recorded there would be wiped by the very reload that
 * was meant to report it.
 */
let writeError = $state<string | null>(null);

/** The value the chosen scope holds for `key`, which is what the field edits. */
function storedValue(key: IdentityKey): string {
	const value = identity?.[key];
	if (!value) return '';
	return (scope === 'local' ? value.local : value.global) ?? '';
}

/** Put both fields back to what the chosen scope holds. */
function resetDrafts(): void {
	drafts = { name: storedValue('name'), email: storedValue('email') };
}

export const settings = {
	get section(): Section {
		return section;
	},
	get identity(): Identity | null {
		return identity;
	},
	get signing(): Signing | null {
		return signing;
	},
	get accounts(): ForgeAccount[] {
		return accounts;
	},
	get update(): Update | null {
		return update;
	},
	get updateError(): string | null {
		return updateError;
	},
	get checking(): boolean {
		return checking;
	},
	get scope(): IdentityScope {
		return scope;
	},
	get settings(): Settings {
		return stored;
	},
	get licenses(): Licenses | null {
		return licenses;
	},
	get about(): About | null {
		return about;
	},
	get loaded(): boolean {
		return loaded;
	},
	get busy(): boolean {
		return busy;
	},
	get error(): string | null {
		return error;
	},
	get writeError(): string | null {
		return writeError;
	},

	/** False when no repository is open, so the local scope cannot be chosen. */
	get canEditLocally(): boolean {
		return identity?.repository ?? false;
	},

	draft(key: IdentityKey): string {
		return drafts[key];
	},

	/** True when the field differs from what the chosen scope holds. */
	isDirty(key: IdentityKey): boolean {
		return drafts[key].trim() !== storedValue(key).trim();
	},

	show(next: Section): void {
		section = next;
	},

	/** Select a section from a URL fragment, ignoring anything unrecognised. */
	showFromHash(hash: string): void {
		const name = hash.replace(/^#/, '');
		// `advanced` was this section's name until it was renamed to `license`,
		// which is what it had always actually held. A link written before the
		// rename still lands somewhere sane rather than silently doing nothing.
		if (name === 'advanced') {
			section = 'license';
			return;
		}
		// `accounts` was a chip of its own that never had a branch to render —
		// pressing it landed on License, while the accounts themselves were
		// drawn under You the whole time. The chip is gone and the fragment
		// resolves to where the section actually lives, so the two links the
		// Pull requests screen carries still arrive at the accounts.
		if (name === 'accounts') {
			section = 'you';
			return;
		}
		if (isSection(name)) section = name;
	},

	/**
	 * Choose which file the fields edit.
	 *
	 * The drafts follow, because a name typed against the global scope must not
	 * be saved into a repository by flipping a chip.
	 */
	setScope(next: IdentityScope): void {
		if (next === 'local' && !this.canEditLocally) return;
		scope = next;
		resetDrafts();
	},

	setDraft(key: IdentityKey, value: string): void {
		drafts = { ...drafts, [key]: value };
	},

	/** Everything the screen shows, read in parallel so one failure is one section. */
	async load(): Promise<void> {
		if (!api.inTauri()) {
			loaded = true;
			return;
		}
		busy = true;
		try {
			const [read, signs, connected, toggles, list, build] = await Promise.allSettled([
				api.identity(),
				api.signing(),
				api.forgeAccounts(),
				api.settings(),
				api.licenses(),
				api.about()
			]);

			if (read.status === 'fulfilled') {
				identity = read.value;
				if (!identity.repository && scope === 'local') scope = 'global';
				resetDrafts();
				error = null;
			} else {
				error = String(read.reason);
			}
			if (signs.status === 'fulfilled') signing = signs.value;
			if (connected.status === 'fulfilled') accounts = connected.value;
			if (toggles.status === 'fulfilled') stored = toggles.value;
			if (list.status === 'fulfilled') licenses = list.value;
			if (build.status === 'fulfilled') about = build.value;

			loaded = true;
		} finally {
			busy = false;
		}
	},

	/**
	 * Write one field to the chosen scope.
	 *
	 * An empty field unsets the key rather than storing an empty string: an
	 * empty `user.email` is a configured empty email, which git will commit
	 * with, while an unset one falls back to the next scope.
	 */
	async save(key: IdentityKey): Promise<void> {
		if (busy) return;
		busy = true;
		writeError = null;
		try {
			identity = await api.setIdentity(scope, key, drafts[key]);
			resetDrafts();
		} catch (e) {
			writeError = String(e);
		} finally {
			busy = false;
		}
	},

	/** Empty a field. Nothing is written until it is saved. */
	clear(key: IdentityKey): void {
		this.setDraft(key, '');
	},

	/**
	 * Turn commit signing on or off in the chosen scope.
	 *
	 * Not optimistic, unlike the preference toggles: this writes to git's own
	 * configuration and the answer carries more than the flag — which file it
	 * landed in, and whether the signer it names can actually run. Showing that
	 * before it is known would be showing a guess.
	 */
	async setSigning(on: boolean): Promise<void> {
		if (busy) return;
		busy = true;
		writeError = null;
		try {
			signing = await api.setSigning(scope, on);
		} catch (e) {
			writeError = String(e);
		} finally {
			busy = false;
		}
	},

	/**
	 * Connect a hosting account (FEAT-017).
	 *
	 * The token is passed straight through to the backend and is never held in
	 * this store. Resolves to whether it worked, so the field can be cleared
	 * either way and the host reset only on success.
	 */
	async connectAccount(host: string, token: string): Promise<boolean> {
		if (busy) return false;
		busy = true;
		writeError = null;
		try {
			// One host is supported; the kind is not a question to ask a person
			// who already typed the hostname.
			accounts = await api.forgeConnect('gitHub', host, token);
			return true;
		} catch (e) {
			writeError = String(e);
			return false;
		} finally {
			busy = false;
		}
	},

	/** Disconnect an account, which also deletes its token from the keychain. */
	async disconnectAccount(host: string, user: string): Promise<void> {
		if (busy) return;
		busy = true;
		writeError = null;
		try {
			accounts = await api.forgeDisconnect(host, user);
		} catch (e) {
			writeError = String(e);
		} finally {
			busy = false;
		}
	},

	/**
	 * Ask whether there is a newer Spagitty.
	 *
	 * Not gated on `checkForUpdates` — that preference governs the automatic
	 * check at startup. Pressing the button is somebody asking, and a button
	 * that silently did nothing because of a setting elsewhere would be worse
	 * than not having one.
	 *
	 * Kept out of `busy`, which gates the writes: a failed update check must
	 * not leave the identity fields disabled.
	 */
	async checkForUpdate(): Promise<void> {
		if (checking || !api.inTauri()) return;
		checking = true;
		updateError = null;
		try {
			update = await api.checkUpdate();
		} catch (e) {
			updateError = String(e);
		} finally {
			checking = false;
		}
	},

	/** Remove `commit.gpgsign` from the chosen scope, letting the next one decide. */
	async clearSigning(): Promise<void> {
		if (busy) return;
		busy = true;
		writeError = null;
		try {
			signing = await api.clearSigning(scope);
		} catch (e) {
			writeError = String(e);
		} finally {
			busy = false;
		}
	},

	/**
	 * Flip a toggle and store it.
	 *
	 * Optimistic, and put back if the write fails: a toggle that shows one state
	 * and stored another is worse than one that visibly refuses.
	 */
	async toggle(key: BooleanSetting): Promise<void> {
		if (busy) return;
		await this.write({ ...stored, [key]: !stored[key] });
	},

	/**
	 * Choose one of a setting's several values, and store it.
	 *
	 * The personality and the sound level are not toggles — three states each —
	 * and giving them their own setter rather than widening `toggle` keeps
	 * `toggle` unable to write a non-boolean, which is what stops a mistyped
	 * key silently storing `false` over somebody's choice.
	 */
	async choose<K extends ChoiceSetting>(key: K, value: Settings[K]): Promise<void> {
		if (busy || stored[key] === value) return;
		await this.write({ ...stored, [key]: value });
	},

	/** Store a whole settings object, optimistically, putting it back on failure. */
	async write(next: Settings): Promise<void> {
		const previous = stored;
		stored = next;
		busy = true;
		writeError = null;
		try {
			await api.setSettings(stored);
		} catch (e) {
			stored = previous;
			writeError = String(e);
		} finally {
			busy = false;
		}
	},

	clearState(): void {
		section = 'you';
		identity = null;
		signing = null;
		accounts = [];
		update = null;
		updateError = null;
		checking = false;
		scope = 'global';
		drafts = { name: '', email: '' };
		stored = DEFAULTS;
		licenses = null;
		about = null;
		loaded = false;
		busy = false;
		error = null;
		writeError = null;
	}
};
