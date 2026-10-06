// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * `extension.json`: its types, and every rule the host enforces.
 *
 * `schemas/extensions/manifest.v1.schema.json` is the authority. This is the
 * same reading the host does in `crates/spagitty-extensions/src/manifest.rs`,
 * so `ext validate` refuses what Spagitty would refuse — with the same field
 * paths — before a package is ever imported. The shared fixtures in
 * `schemas/extensions/fixtures/manifests/` are run against both.
 */

import { parseRange, parseVersion } from './version';
import type { Capability, ContextKind, ReviewScope, ReviewTarget } from './protocol';

export const TARGETS = [
	'x86_64-pc-windows-msvc',
	'aarch64-pc-windows-msvc',
	'x86_64-unknown-linux-gnu',
	'aarch64-unknown-linux-gnu',
	'x86_64-apple-darwin',
	'aarch64-apple-darwin'
] as const;
export type Target = (typeof TARGETS)[number];

export const CAPABILITIES: Capability[] = [
	'repository.read',
	'review.provide',
	'tools.execute',
	'forge.pullRequest.read',
	'forge.pullRequest.comment'
];

const CONTEXTS = ['global', 'workingCopy', 'farmTask', 'pullRequest'];
const PREDICATES = ['repositoryOpen', 'hasWorkingChanges', 'taskSelected', 'taskHasCommit', 'pullRequestSelected', 'forgeConnected'];

export type ToolOption =
	| { type: 'enum'; values: Record<string, string[]>; required?: boolean }
	| { type: 'revision' | 'commit'; flag: string; required?: boolean };

export interface Manifest {
	$schema?: string;
	manifestVersion: 1;
	id: string;
	name: string;
	version: string;
	publisher: string;
	description?: string;
	license?: string;
	homepage?: string;
	engines: { spagitty: string; extensionApi: string };
	runtime: { kind: 'native-process'; entrypoints: Partial<Record<Target, string>> };
	activation?: ('onCommand' | 'onReviewProvider' | 'onPanel')[];
	capabilities?: { required?: Capability[]; optional?: Capability[] };
	externalTools?: {
		id: string;
		name?: string;
		executableNames: string[];
		versionArgs?: string[];
		minimumVersion?: string;
		installUrl?: string;
		profiles: {
			id: string;
			args: string[];
			options?: Record<string, ToolOption>;
			workdir?: 'operation' | 'none';
			timeoutMs?: number;
		}[];
	}[];
	contributes?: {
		commands?: {
			id: string;
			title: string;
			context?: ContextKind;
			when?: string[];
			menu?: boolean;
			palette?: boolean;
			reviewProvider?: string;
		}[];
		reviewProviders?: {
			id: string;
			title?: string;
			targets: ReviewTarget[];
			scopes?: ReviewScope[];
			configurationFiles?: string[];
			sendsCodeTo?: string;
		}[];
		panels?: {
			id: string;
			title: string;
			renderer: 'reviewFindings' | 'reviewStatus' | 'summary';
			location?: ContextKind;
			provider?: string;
		}[];
		settings?: {
			key: string;
			title?: string;
			description?: string;
			type: 'boolean' | 'enum' | 'text' | 'number' | 'executable';
			scope?: 'user' | 'repository';
			default?: unknown;
			values?: string[];
			labels?: Record<string, string>;
			min?: number;
			max?: number;
			maxLength?: number;
			tool?: string;
		}[];
	};
}

export function isExtensionId(id: string): boolean {
	if (id.length > 100) return false;
	const labels = id.split('.');
	return (
		labels.length >= 2 &&
		labels.length <= 5 &&
		labels.every((l) => /^[a-z0-9]+(-[a-z0-9]+)*$/.test(l))
	);
}

export function isLocalId(id: string): boolean {
	return /^[A-Za-z][A-Za-z0-9_-]{0,63}$/.test(id);
}

export function isPackagePath(path: string): boolean {
	return (
		path.length > 0 &&
		path.length <= 200 &&
		path.split('/').every((s) => s.length > 0 && s !== '.' && s !== '..' && /^[A-Za-z0-9._-]+$/.test(s))
	);
}

type Json = unknown;
type Obj = Record<string, Json>;
const isObj = (v: Json): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);
const join = (path: string, key: string) => (path ? `${path}.${key}` : key);
const length = (s: string) => [...s].length;

class Validator {
	errors: string[] = [];

	error(path: string, message: string) {
		this.errors.push(`${path || 'manifest'}: ${message}`);
	}

	object(path: string, value: Json, allowed: string[], required: string[]): Obj | null {
		if (!isObj(value)) {
			this.error(path, 'must be an object');
			return null;
		}
		for (const key of Object.keys(value)) {
			if (!allowed.includes(key)) this.error(join(path, key), 'is not a field this manifest version knows');
		}
		for (const key of required) {
			if (!(key in value)) this.error(join(path, key), 'is required');
		}
		return value;
	}

	string(path: string, value: Json, min: number, max: number): string | null {
		if (value === undefined) return null;
		if (typeof value !== 'string') {
			this.error(path, 'must be a string');
			return null;
		}
		const n = length(value);
		if (n < min) this.error(path, min === 1 ? 'must not be empty' : `must be at least ${min} characters`);
		else if (n > max) this.error(path, `must be at most ${max} characters`);
		return value;
	}

	array(path: string, value: Json, max: number): Json[] {
		if (value === undefined) return [];
		if (!Array.isArray(value)) {
			this.error(path, 'must be a list');
			return [];
		}
		if (value.length > max) this.error(path, `may have at most ${max} entries`);
		return value;
	}

	bool(path: string, value: Json) {
		if (value !== undefined && typeof value !== 'boolean') this.error(path, 'must be true or false');
	}

	oneOf(path: string, value: Json, allowed: string[]): string | null {
		const text = this.string(path, value, 1, 64);
		if (text === null) return null;
		if (allowed.includes(text)) return text;
		this.error(path, `must be one of ${allowed.join(', ')}`);
		return null;
	}

	localId(path: string, value: Json): string | null {
		const text = this.string(path, value, 1, 64);
		if (text === null) return null;
		if (isLocalId(text)) return text;
		this.error(path, "must start with a letter and use only letters, digits, '-' and '_'");
		return null;
	}

	https(path: string, value: Json) {
		const text = this.string(path, value, 1, 300);
		if (text !== null && (!text.startsWith('https://') || /\s/.test(text))) this.error(path, 'must be an https:// address');
	}

	unique(path: string, what: string, ids: string[]) {
		const seen = new Set<string>();
		for (const id of ids) {
			if (seen.has(id)) this.error(path, `declares the ${what} '${id}' more than once`);
			seen.add(id);
		}
	}

	literal(path: string, value: Json) {
		const text = this.string(path, value, 1, 128);
		if (text !== null && /[\s\0]/.test(text)) this.error(path, 'must be one argument with no spaces');
	}

	manifest(value: Json) {
		const root = this.object(
			'',
			value,
			['$schema', 'manifestVersion', 'id', 'name', 'version', 'publisher', 'description', 'license', 'homepage', 'engines', 'runtime', 'activation', 'capabilities', 'externalTools', 'contributes'],
			['manifestVersion', 'id', 'name', 'version', 'publisher', 'engines', 'runtime']
		);
		if (!root) return;
		if ('manifestVersion' in root && root.manifestVersion !== 1) {
			this.error('manifestVersion', `is ${JSON.stringify(root.manifestVersion)}; this Spagitty reads manifest version 1`);
		}
		const id = this.string('id', root.id, 3, 100);
		if (id !== null && !isExtensionId(id)) {
			this.error('id', 'must be two to five lowercase dot-separated labels, such as com.example.hello');
		}
		this.string('name', root.name, 1, 60);
		this.string('publisher', root.publisher, 1, 60);
		this.string('description', root.description, 0, 300);
		this.string('license', root.license, 1, 64);
		if ('homepage' in root) this.https('homepage', root.homepage);
		const version = this.string('version', root.version, 5, 64);
		if (version !== null && !parseVersion(version)) this.error('version', 'must be a semantic version such as 1.0.0');

		if ('engines' in root) {
			const engines = this.object('engines', root.engines, ['spagitty', 'extensionApi'], ['spagitty', 'extensionApi']);
			if (engines) {
				for (const key of ['spagitty', 'extensionApi']) {
					const range = this.string(`engines.${key}`, engines[key], 1, 100);
					if (range !== null && !parseRange(range)) {
						this.error(`engines.${key}`, 'is not a version requirement, such as >=0.9.0, <1.0.0');
					}
				}
			}
		}
		if ('runtime' in root) this.runtime(root.runtime);

		const activation: string[] = [];
		this.array('activation', root.activation, 3).forEach((entry, i) => {
			const kind = this.oneOf(`activation[${i}]`, entry, ['onCommand', 'onReviewProvider', 'onPanel']);
			if (kind) activation.push(kind);
		});
		this.unique('activation', 'activation', activation);

		const capabilities = 'capabilities' in root ? this.capabilities(root.capabilities) : [];
		const tools: string[] = [];
		this.array('externalTools', root.externalTools, 8).forEach((tool, i) => {
			const toolId = this.tool(`externalTools[${i}]`, tool);
			if (toolId) tools.push(toolId);
		});
		this.unique('externalTools', 'tool', tools);
		if (tools.length && !capabilities.includes('tools.execute')) {
			this.error('externalTools', 'declares tools, so capabilities must ask for tools.execute');
		}
		if ('contributes' in root) this.contributes(root.contributes, capabilities, tools);
	}

	runtime(value: Json) {
		const runtime = this.object('runtime', value, ['kind', 'entrypoints'], ['kind', 'entrypoints']);
		if (!runtime) return;
		if (runtime.kind !== undefined && runtime.kind !== 'native-process') {
			this.error('runtime.kind', `is '${String(runtime.kind)}'; only native-process is supported`);
		}
		if (runtime.entrypoints === undefined) return;
		if (!isObj(runtime.entrypoints)) {
			this.error('runtime.entrypoints', 'must be an object of target to path');
			return;
		}
		if (!Object.keys(runtime.entrypoints).length) this.error('runtime.entrypoints', 'must name at least one target');
		for (const [target, path] of Object.entries(runtime.entrypoints)) {
			const at = `runtime.entrypoints.${target}`;
			if (!(TARGETS as readonly string[]).includes(target)) this.error(at, `is not a target; use one of ${TARGETS.join(', ')}`);
			const text = this.string(at, path, 1, 200);
			if (text !== null && !isPackagePath(text)) this.error(at, 'must be a relative path inside the package, with forward slashes');
		}
	}

	capabilities(value: Json): Capability[] {
		const object = this.object('capabilities', value, ['required', 'optional'], []);
		if (!object) return [];
		const all: Capability[] = [];
		for (const key of ['required', 'optional']) {
			const path = `capabilities.${key}`;
			this.array(path, object[key], 5).forEach((entry, i) => {
				const at = `${path}[${i}]`;
				if (typeof entry !== 'string') return this.error(at, 'must be a string');
				if (!CAPABILITIES.includes(entry as Capability)) return this.error(at, `'${entry}' is not a capability this Spagitty knows`);
				if (all.includes(entry as Capability)) this.error(at, `${entry} is listed more than once`);
				all.push(entry as Capability);
			});
		}
		return all;
	}

	tool(path: string, value: Json): string | null {
		const tool = this.object(path, value, ['id', 'name', 'executableNames', 'versionArgs', 'minimumVersion', 'installUrl', 'profiles'], ['id', 'executableNames', 'profiles']);
		if (!tool) return null;
		const id = this.localId(join(path, 'id'), tool.id);
		this.string(join(path, 'name'), tool.name, 1, 60);
		const names = this.array(join(path, 'executableNames'), tool.executableNames, 4);
		if ('executableNames' in tool && !names.length) this.error(join(path, 'executableNames'), 'must name at least one executable');
		names.forEach((name, i) => {
			const at = `${path}.executableNames[${i}]`;
			const text = this.string(at, name, 1, 64);
			if (text !== null && !/^[A-Za-z0-9._-]+$/.test(text)) this.error(at, 'must be a bare file name, never a path');
		});
		this.array(join(path, 'versionArgs'), tool.versionArgs, 4).forEach((arg, i) => this.literal(`${path}.versionArgs[${i}]`, arg));
		const minimum = this.string(join(path, 'minimumVersion'), tool.minimumVersion, 5, 64);
		if (minimum !== null && !parseVersion(minimum)) this.error(join(path, 'minimumVersion'), 'must be a semantic version');
		if ('installUrl' in tool) this.https(join(path, 'installUrl'), tool.installUrl);
		const profiles = this.array(join(path, 'profiles'), tool.profiles, 16);
		if ('profiles' in tool && !profiles.length) this.error(join(path, 'profiles'), 'must declare at least one profile');
		const ids: string[] = [];
		profiles.forEach((profile, i) => {
			const pid = this.profile(`${path}.profiles[${i}]`, profile);
			if (pid) ids.push(pid);
		});
		this.unique(join(path, 'profiles'), 'profile', ids);
		return id;
	}

	profile(path: string, value: Json): string | null {
		const profile = this.object(path, value, ['id', 'args', 'options', 'workdir', 'timeoutMs'], ['id', 'args']);
		if (!profile) return null;
		const id = this.localId(join(path, 'id'), profile.id);
		this.array(join(path, 'args'), profile.args, 16).forEach((arg, i) => this.literal(`${path}.args[${i}]`, arg));
		if ('workdir' in profile) this.oneOf(join(path, 'workdir'), profile.workdir, ['operation', 'none']);
		if ('timeoutMs' in profile) {
			const ms = profile.timeoutMs;
			if (typeof ms !== 'number' || !Number.isInteger(ms) || ms < 1000 || ms > 7_200_000) {
				this.error(join(path, 'timeoutMs'), 'must be between 1000 and 7200000');
			}
		}
		if ('options' in profile) {
			const at = join(path, 'options');
			if (!isObj(profile.options)) this.error(at, 'must be an object');
			else {
				const entries = Object.entries(profile.options);
				if (entries.length > 8) this.error(at, 'may declare at most 8 options');
				for (const [name, option] of entries) {
					const here = join(at, name);
					if (!isLocalId(name)) this.error(here, 'is not a valid option name');
					this.option(here, option);
				}
			}
		}
		return id;
	}

	option(path: string, value: Json) {
		const kind = isObj(value) ? value.type : undefined;
		if (kind === 'enum') {
			const option = this.object(path, value, ['type', 'values', 'required'], ['type', 'values']);
			if (!option) return;
			this.bool(join(path, 'required'), option.required);
			if (isObj(option.values)) {
				const values = Object.entries(option.values);
				if (!values.length || values.length > 16) return this.error(join(path, 'values'), 'must have between 1 and 16 values');
				for (const [name, args] of values) {
					const at = join(join(path, 'values'), name);
					if (!isLocalId(name)) this.error(at, 'is not a valid value name');
					this.array(at, args, 4).forEach((arg, i) => this.literal(`${at}[${i}]`, arg));
				}
			} else if ('values' in option) {
				this.error(join(path, 'values'), 'must be an object of value to arguments');
			}
		} else if (kind === 'revision' || kind === 'commit') {
			const option = this.object(path, value, ['type', 'flag', 'required'], ['type', 'flag']);
			if (!option) return;
			this.bool(join(path, 'required'), option.required);
			const flag = this.string(join(path, 'flag'), option.flag, 2, 43);
			if (flag !== null && !/^--?[A-Za-z0-9][A-Za-z0-9-]{0,40}$/.test(flag)) this.error(join(path, 'flag'), 'must be a flag such as --base');
		} else {
			this.error(join(path, 'type'), 'must be enum, revision or commit');
		}
	}

	contributes(value: Json, capabilities: Capability[], tools: string[]) {
		const contributes = this.object('contributes', value, ['commands', 'reviewProviders', 'panels', 'settings'], []);
		if (!contributes) return;

		const providers: string[] = [];
		this.array('contributes.reviewProviders', contributes.reviewProviders, 4).forEach((provider, index) => {
			const path = `contributes.reviewProviders[${index}]`;
			const object = this.object(path, provider, ['id', 'title', 'targets', 'scopes', 'configurationFiles', 'sendsCodeTo'], ['id', 'targets']);
			if (!object) return;
			const id = this.localId(join(path, 'id'), object.id);
			if (id) providers.push(id);
			this.string(join(path, 'title'), object.title, 1, 60);
			this.string(join(path, 'sendsCodeTo'), object.sendsCodeTo, 1, 120);
			const targets = this.array(join(path, 'targets'), object.targets, 3);
			if ('targets' in object && !targets.length) this.error(join(path, 'targets'), 'must name at least one target');
			const seenTargets: string[] = [];
			targets.forEach((t, i) => {
				const v = this.oneOf(`${path}.targets[${i}]`, t, ['workingCopy', 'farmTask', 'pullRequest']);
				if (v) seenTargets.push(v);
			});
			this.unique(join(path, 'targets'), 'target', seenTargets);
			const seenScopes: string[] = [];
			this.array(join(path, 'scopes'), object.scopes, 4).forEach((s, i) => {
				const v = this.oneOf(`${path}.scopes[${i}]`, s, ['committed', 'uncommitted', 'tracked', 'includeUntracked']);
				if (v) seenScopes.push(v);
			});
			this.unique(join(path, 'scopes'), 'scope', seenScopes);
			this.array(join(path, 'configurationFiles'), object.configurationFiles, 16).forEach((file, i) => {
				const at = `${path}.configurationFiles[${i}]`;
				const text = this.string(at, file, 1, 200);
				if (text !== null && !isPackagePath(text)) this.error(at, 'must be a relative path inside the repository');
			});
		});
		this.unique('contributes.reviewProviders', 'review provider', providers);
		if (providers.length && !capabilities.includes('review.provide')) {
			this.error('contributes.reviewProviders', 'needs the review.provide capability');
		}

		const commands: string[] = [];
		this.array('contributes.commands', contributes.commands, 50).forEach((command, index) => {
			const path = `contributes.commands[${index}]`;
			const object = this.object(path, command, ['id', 'title', 'context', 'when', 'menu', 'palette', 'reviewProvider'], ['id', 'title']);
			if (!object) return;
			const id = this.localId(join(path, 'id'), object.id);
			if (id) commands.push(id);
			this.string(join(path, 'title'), object.title, 1, 80);
			if ('context' in object) this.oneOf(join(path, 'context'), object.context, CONTEXTS);
			const when: string[] = [];
			this.array(join(path, 'when'), object.when, 6).forEach((w, i) => {
				const v = this.oneOf(`${path}.when[${i}]`, w, PREDICATES);
				if (v) when.push(v);
			});
			this.unique(join(path, 'when'), 'condition', when);
			this.bool(join(path, 'menu'), object.menu);
			this.bool(join(path, 'palette'), object.palette);
			if ('reviewProvider' in object) {
				const provider = this.localId(join(path, 'reviewProvider'), object.reviewProvider);
				if (provider && !providers.includes(provider)) {
					this.error(join(path, 'reviewProvider'), `names '${provider}', which is not declared`);
				}
			}
		});
		this.unique('contributes.commands', 'command', commands);

		const panels: string[] = [];
		this.array('contributes.panels', contributes.panels, 10).forEach((panel, index) => {
			const path = `contributes.panels[${index}]`;
			const object = this.object(path, panel, ['id', 'title', 'renderer', 'location', 'provider'], ['id', 'title', 'renderer']);
			if (!object) return;
			const id = this.localId(join(path, 'id'), object.id);
			if (id) panels.push(id);
			this.string(join(path, 'title'), object.title, 1, 60);
			const renderer = this.oneOf(join(path, 'renderer'), object.renderer, ['reviewFindings', 'reviewStatus', 'summary']);
			if ('location' in object) this.oneOf(join(path, 'location'), object.location, CONTEXTS);
			const provider = 'provider' in object ? this.localId(join(path, 'provider'), object.provider) : null;
			if (provider && !providers.includes(provider)) this.error(join(path, 'provider'), `names '${provider}', which is not declared`);
			if (renderer === 'reviewFindings' && !('provider' in object) && providers.length !== 1) {
				this.error(join(path, 'provider'), "is required: a findings panel shows one review provider's results");
			}
		});
		this.unique('contributes.panels', 'panel', panels);

		const keys: string[] = [];
		this.array('contributes.settings', contributes.settings, 50).forEach((setting, index) => {
			const key = this.setting(`contributes.settings[${index}]`, setting, tools);
			if (key) keys.push(key);
		});
		this.unique('contributes.settings', 'setting', keys);
	}

	setting(path: string, value: Json, tools: string[]): string | null {
		const s = this.object(path, value, ['key', 'title', 'description', 'type', 'scope', 'default', 'values', 'labels', 'min', 'max', 'maxLength', 'tool'], ['key', 'type']);
		if (!s) return null;
		const key = this.localId(join(path, 'key'), s.key);
		this.string(join(path, 'title'), s.title, 1, 60);
		this.string(join(path, 'description'), s.description, 0, 200);
		if ('scope' in s) this.oneOf(join(path, 'scope'), s.scope, ['user', 'repository']);
		const kind = this.oneOf(join(path, 'type'), s.type, ['boolean', 'enum', 'text', 'number', 'executable']);
		const values: string[] = [];
		this.array(join(path, 'values'), s.values, 20).forEach((entry, i) => {
			const at = `${path}.values[${i}]`;
			const text = this.string(at, entry, 1, 40);
			if (text === null) return;
			if (!/^[A-Za-z0-9_-]+$/.test(text)) this.error(at, "must use only letters, digits, '-' and '_'");
			values.push(text);
		});
		this.unique(join(path, 'values'), 'value', values);
		if ('labels' in s) {
			if (!isObj(s.labels)) this.error(join(path, 'labels'), 'must be an object');
			else
				for (const [name, label] of Object.entries(s.labels)) {
					const at = join(join(path, 'labels'), name);
					if (!values.includes(name)) this.error(at, 'labels a value that is not in values');
					this.string(at, label, 1, 60);
				}
		}
		for (const bound of ['min', 'max']) {
			if (bound in s && typeof s[bound] !== 'number') this.error(join(path, bound), 'must be a number');
		}
		if ('maxLength' in s) {
			const n = s.maxLength;
			if (typeof n !== 'number' || !Number.isInteger(n) || n < 1 || n > 1000) this.error(join(path, 'maxLength'), 'must be between 1 and 1000');
		}
		const d = s.default;
		const has = 'default' in s;
		const min = typeof s.min === 'number' ? s.min : null;
		const max = typeof s.max === 'number' ? s.max : null;
		switch (kind) {
			case 'boolean':
				if (has && typeof d !== 'boolean') this.error(join(path, 'default'), 'must be true or false');
				break;
			case 'enum':
				if (!values.length) this.error(join(path, 'values'), 'is required for an enum setting');
				if (has && !(typeof d === 'string' && values.includes(d))) this.error(join(path, 'default'), 'must be one of values');
				break;
			case 'text':
				if (has) {
					if (typeof d !== 'string') this.error(join(path, 'default'), 'must be text');
					else if (length(d) > (typeof s.maxLength === 'number' ? s.maxLength : 1000)) this.error(join(path, 'default'), 'is longer than maxLength');
				}
				break;
			case 'number':
				if (min !== null && max !== null && min > max) this.error(join(path, 'min'), 'is greater than max');
				if (has) {
					if (typeof d !== 'number') this.error(join(path, 'default'), 'must be a number');
					else if ((min !== null && d < min) || (max !== null && d > max)) this.error(join(path, 'default'), 'is outside min and max');
				}
				break;
			case 'executable':
				if (has) this.error(join(path, 'default'), 'is not allowed: an executable is always chosen by the user');
				if (typeof s.tool === 'string') {
					if (!tools.includes(s.tool)) this.error(join(path, 'tool'), `names '${s.tool}', which is not declared in externalTools`);
				} else this.error(join(path, 'tool'), 'is required for an executable setting');
				break;
		}
		if ('tool' in s && kind !== 'executable') this.error(join(path, 'tool'), 'only applies to an executable setting');
		return key;
	}
}

/** Every rule `value` breaks, each prefixed with the field's path. */
export function validate(value: unknown): string[] {
	const v = new Validator();
	v.manifest(value);
	return v.errors;
}

/** Read and validate a manifest, throwing every error at once. */
export function parseManifest(text: string): Manifest {
	let value: unknown;
	try {
		value = JSON.parse(text);
	} catch (error) {
		throw new ManifestError([`extension.json is not JSON: ${(error as Error).message}`]);
	}
	const errors = validate(value);
	if (errors.length) throw new ManifestError(errors);
	return value as Manifest;
}

export class ManifestError extends Error {
	constructor(readonly errors: string[]) {
		super(`The extension's manifest is not valid:\n${errors.join('\n')}`);
		this.name = 'ManifestError';
	}
}
