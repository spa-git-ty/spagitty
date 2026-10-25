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
}

export interface Done {
	sendId: string;
	callId: string;
	response: ResponseView | null;
	error: string | null;
	cancelled: boolean;
	sent: Shown;
	historyId: string | null;
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
