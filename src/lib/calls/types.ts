// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Calls (FEAT-160): the types that cross from `spagitty_core::calls` and
 * `src-tauri/src/calls.rs`, mirrored by hand as `src/lib/types.ts` is.
 */

export type Kind = 'rest' | 'soap';
export type BodyType = 'json' | 'xml' | 'form' | 'multipart' | 'raw' | 'none';
export type AuthType = 'none' | 'bearer' | 'basic' | 'apiKey';
export type Colour = 'ok' | 'warn' | 'danger' | 'blue' | 'teal';

/** `[name, value]`, with a third `false` when switched off. */
export type Pair = [string, string] | [string, string, boolean];

export interface Field {
	name: string;
	value: string;
	file?: boolean;
	on?: boolean;
}

export interface Body {
	type: BodyType;
	text?: string;
	fields?: Field[];
	contentType?: string;
}

export interface Auth {
	type: AuthType;
	token?: string;
	user?: string;
	password?: string;
	key?: string;
	value?: string;
	in?: 'header' | 'query';
}

export interface WsSecurity {
	mode: 'none' | 'username' | 'signed';
	user?: string;
	password?: string;
	digest?: boolean;
}

export interface KeepRule {
	var: string;
	from: string;
	every?: boolean;
	ns?: Record<string, string>;
}

export interface Call {
	id: string;
	name: string;
	kind: Kind;
	method?: string;
	url: string;
	headers?: Pair[];
	body?: Body;
	auth?: Auth;
	service?: string;
	port?: string;
	operation?: string;
	version?: string;
	action?: string;
	envelope?: string;
	wsSecurity?: WsSecurity;
	keep?: KeepRule[];
	notes?: string;
}

export interface Folder {
	folder: string;
	items: Item[];
}

export type Item = Folder | Call;

export interface Variable {
	name: string;
	value?: string;
	secret?: boolean;
	kept?: boolean;
	frozen?: boolean;
}

export interface Collection {
	v: number;
	name: string;
	wsdl?: string;
	vars?: Variable[];
	items: Item[];
}

export interface Environment {
	v: number;
	name: string;
	color: Colour;
	askBeforeSending: boolean;
	checkTls?: boolean;
	vars: Variable[];
}

export interface Shared {
	v: number;
	vars: Variable[];
}

/** A file read from the calls folder, or why it could not be. */
export interface Stored<T> {
	stem: string;
	file: string;
	value: T | null;
	error: string | null;
}

export interface Opened {
	stem: string;
	id: string;
}

export interface PersonState {
	active: string | null;
	open: Opened[];
	current: Opened | null;
	collapsed: string[];
	filter: string;
}

export type Proxy = { kind: 'system' } | { kind: 'none' } | { kind: 'custom'; url: string };

export interface ClientCertFor {
	host: string;
	cert: string;
	key: string;
}

export interface CallsSettings {
	folder: string;
	openFound: boolean;
	lookFor: string[];
	scanOnOpen: 'ask' | 'always' | 'never';
	frameworks: string[];
	skipPaths: string;
	clickToKeep: boolean;
	keepEvery: boolean;
	suggestNames: boolean;
	timeoutSeconds: number;
	followRedirects: boolean;
	checkTls: boolean;
	proxy: Proxy;
	clientCerts: ClientCertFor[];
	beautify: boolean;
	everyHeader: string | null;
	soapVersion: string;
	envelopeFromWsdl: boolean;
	wsSecurity: string;
	faultAsError: boolean;
	history: number;
	shareAs: 'file' | 'zip' | 'commit';
}

export interface Loaded {
	folder: string;
	collections: Stored<Collection>[];
	environments: Stored<Environment>[];
	shared: Shared;
	sharedError: string | null;
	state: PersonState;
	settings: CallsSettings;
	overridden: string[];
	today: number;
}

export type Shape = 'json' | 'xml' | 'html' | 'events' | 'text' | 'binary';

export interface Shown {
	method: string;
	url: string;
	headers: [string, string][];
	body: string | null;
}

export interface ResponseView {
	status: number;
	reason: string;
	headers: [string, string][];
	millis: number;
	size: number;
	truncated: boolean;
	shape: Shape;
	mediaType: string;
	text: string | null;
	plain: boolean;
	url: string;
	/** A SOAP call's fault, when Settings says to show one as an error (FEAT-163). */
	fault?: Fault | null;
}

export interface Fault {
	code: string;
	reason: string;
	detail: string | null;
}

/** A WSDL's operations, for the operation picker (FEAT-163). */
export interface WsdlView {
	services: {
		name: string;
		ports: {
			name: string;
			version: '1.1' | '1.2';
			address: string;
			operations: { name: string; action: string; rpc: boolean }[];
		}[];
	}[];
}

export interface Done {
	sendId: string;
	callId: string;
	response: ResponseView | null;
	error: string | null;
	cancelled: boolean;
	sent: Shown;
	historyId: string | null;
	/** What the call's *every call* rules read; `value: null` found nothing (FEAT-161). */
	kept: Applied[];
	keptError: string | null;
}

export interface Applied {
	var: string;
	value: string | null;
	from: string;
}

/** Where a kept value came from (`calls::keep::Kept`). */
export interface Kept {
	at: number;
	callId: string;
	call: string;
	method: string;
	from: string;
	stale: boolean;
}

export interface KeptView extends Kept {
	name: string;
	value: string | null;
}

/** An environment's private side: kept values in the clear, and which secrets have a value. */
export interface Private {
	kept: KeptView[];
	secrets: string[];
}

export interface Environments {
	environments: Stored<Environment>[];
	shared: Shared;
	sharedError: string | null;
}

export interface Chunk {
	sendId: string;
	text: string;
}

export interface HistoryEntry {
	id: string;
	at: number;
	callId: string | null;
	collection: string | null;
	name: string;
	environment: string | null;
	request: Shown;
	status: number | null;
	reason: string;
	millis: number;
	size: number;
	headers: [string, string][];
	error: string | null;
	cut: boolean;
}

// --- Import (FEAT-162) ---

export type SourceKind =
	| 'spring'
	| 'jaxRs'
	| 'nest'
	| 'jaxWs'
	| 'springWs'
	| 'wsdl'
	| 'openApi'
	| 'postman'
	| 'insomnia'
	| 'bruno'
	| 'http'
	| 'har'
	| 'curl'
	| 'spagitty';

export interface Source {
	id: string;
	kind: SourceKind;
	label: string;
	file: string | null;
	count: number;
	error: string | null;
	/** A file in the repository, which can be opened in place. */
	inRepository: boolean;
}

export type EndpointStatus =
	| { kind: 'new' }
	| { kind: 'changed'; collection: string; callId: string; what: string }
	| { kind: 'in'; collection: string; callId: string };

export interface Endpoint {
	id: string;
	key: string;
	source: string;
	group: string;
	file: string | null;
	line: number | null;
	method: string;
	path: string;
	hint: string;
	call: Call;
	status: EndpointStatus;
}

export interface Found {
	sources: Source[];
	endpoints: Endpoint[];
	environments: Environment[];
	/** Calls whose Postman script lines went into their notes. */
	scripted: number;
	baseUrl: string | null;
	filesRead: number;
	capped: boolean;
}

export interface Choices {
	folders: 'perGroup' | 'byPath';
	bodies: boolean;
	pathVars: boolean;
}

export interface BroughtIn {
	stem: string;
	brought: { added: number; changed: number; unchanged: number };
	environments: number;
}

export interface ScanProgress {
	scanId: string;
	files: number;
}

export interface ScanDone {
	scanId: string;
	found: Found | null;
	error: string | null;
	cancelled: boolean;
}

// --- cURL (FEAT-166) ---

export type Quoting = 'posix' | 'cmd';

/** A pasted command: the call, and which flags were read and which were not. */
export interface Pasted {
	call: Call;
	read: string[];
	ignored: string[];
}

// --- Share (FEAT-164) ---

export type ShareFormat = 'spagitty' | 'postman' | 'zip' | 'commit';

export interface ShareAsk {
	stem: string;
	format: ShareFormat;
	environments: string[];
	examples: boolean;
	history: boolean;
	path: string | null;
}

export interface SharePlan {
	files: { path: string; size: number }[];
	/** What is written: the zip's size, or the files'. */
	total: number;
}
