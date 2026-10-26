// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import {
	addCall,
	callsIn,
	cookiesOf,
	filterItems,
	findCall,
	folderOf,
	formatSize,
	lookup,
	mapCall,
	missingIn,
	newId,
	paramsOf,
	removeCall,
	statusTone,
	suggestName,
	tabCounts,
	withParams
} from './model';
import type { Call, Collection, Environment, Item, Shared } from './types';

const call = (id: string, name: string, method = 'GET', url = ''): Call => ({ id, name, kind: 'rest', method, url });

const TREE: Item[] = [
	{ folder: 'Auth', items: [call('a', 'Log in', 'POST', '{{baseUrl}}/auth/login')] },
	{ folder: 'Users', items: [call('b', 'Get user', 'GET', '{{baseUrl}}/users/{{userId}}')] },
	call('c', 'Health')
];

describe('a collection’s tree', () => {
	it('finds, changes and removes a call in its folder', () => {
		expect(callsIn(TREE).map((found) => found.id)).toEqual(['a', 'b', 'c']);
		expect(findCall(TREE, 'b')?.name).toBe('Get user');
		expect(folderOf(TREE, 'b')).toBe('Users');
		expect(folderOf(TREE, 'c')).toBeNull();
		const renamed = mapCall(TREE, 'b', (found) => ({ ...found, name: 'Read user' }));
		expect(findCall(renamed, 'b')?.name).toBe('Read user');
		expect(findCall(TREE, 'b')?.name).toBe('Get user');
		expect(callsIn(removeCall(TREE, 'a')).map((found) => found.id)).toEqual(['b', 'c']);
	});

	it('adds a call to a folder, making the folder when there is none', () => {
		expect(callsIn(addCall(TREE, call('d', 'D'), 'Auth')).map((found) => found.id)).toEqual(['a', 'd', 'b', 'c']);
		const made = addCall(TREE, call('e', 'E'), 'Orders');
		expect(made[made.length - 1]).toEqual({ folder: 'Orders', items: [call('e', 'E')] });
		expect(addCall(TREE, call('f', 'F')).length).toBe(4);
	});

	it('filters by method, name or URL and drops empty folders', () => {
		expect(callsIn(filterItems(TREE, 'post')).map((found) => found.id)).toEqual(['a']);
		expect(filterItems(TREE, 'users')).toEqual([TREE[1]]);
		expect(filterItems(TREE, '  ')).toBe(TREE);
	});

	it('makes ids in the backend’s shape, ordered by time', () => {
		const first = newId(1_700_000_000_000, () => 0);
		const later = newId(1_700_000_000_001, () => 0);
		expect(first).toMatch(/^c_[0-9a-hjkmnp-tv-z]{12}$/);
		expect(first < later).toBe(true);
	});
});

describe('the query as rows', () => {
	it('reads the query as written, variables and all', () => {
		expect(paramsOf('{{baseUrl}}/x?page=2&q={{term}}&flag#top')).toEqual([
			['page', '2'],
			['q', '{{term}}'],
			['flag', '']
		]);
		expect(paramsOf('http://h/x')).toEqual([]);
	});

	it('writes rows back, leaving out a row switched off and keeping the fragment', () => {
		expect(withParams('http://h/x?old=1#top', [['a', '1'], ['b', '2', false], ['c', '']])).toBe('http://h/x?a=1&c#top');
		expect(withParams('http://h/x?old=1', [])).toBe('http://h/x');
	});
});

describe('what the tabs count', () => {
	it('counts what each tab holds', () => {
		expect(
			tabCounts({
				...call('a', 'A', 'POST', 'h?x=1'),
				headers: [['Accept', 'json'], ['X-Off', '1', false]],
				body: { type: 'json', text: '{}' },
				auth: { type: 'bearer', token: '{{token}}' },
				keep: [{ var: 'token', from: '$.token' }]
			})
		).toEqual({ params: 1, headers: 1, body: 1, auth: 1, keep: 1 });
		expect(tabCounts(call('a', 'A'))).toEqual({ params: 0, headers: 0, body: 0, auth: 0, keep: 0 });
	});
});

describe('variables', () => {
	const env: Environment = {
		v: 1,
		name: 'staging',
		color: 'warn',
		askBeforeSending: false,
		vars: [{ name: 'baseUrl', value: 'https://s' }]
	};
	const shared: Shared = { v: 1, vars: [{ name: 'baseUrl', value: 'https://shared' }, { name: 'tenant', value: 't' }] };
	const collection: Collection = { v: 1, name: 'c', items: [], vars: [{ name: 'userId', value: '1' }] };

	it('finds a name in the environment, then shared, then the collection', () => {
		expect(lookup('baseUrl', env, shared, collection)?.from).toBe('environment');
		expect(lookup('tenant', env, shared, collection)?.from).toBe('shared');
		expect(lookup('userId', env, shared, collection)?.from).toBe('collection');
		expect(lookup('$uuid', null, null, null)?.from).toBe('builtin');
		expect(lookup('nope', env, shared, collection)).toBeNull();
	});

	it('names what a call uses that nothing defines', () => {
		const used = { ...call('a', 'A', 'GET', '{{baseUrl}}/{{id}}'), headers: [['X', '{{off}}', false]] as Call['headers'] };
		expect(missingIn(used, env, shared, collection)).toEqual(['id']);
		expect(missingIn(used, null, null, null)).toEqual(['baseUrl', 'id']);
	});
});

describe('the name a kept value is offered', () => {
	it('is the last key, as the proposal says', () => {
		expect(suggestName('$.data.accessToken')).toBe('token');
		expect(suggestName('$.data.user.id')).toBe('userId');
		expect(suggestName('$.data.refreshToken')).toBe('refreshToken');
		expect(suggestName('$.data.id')).toBe('id');
		expect(suggestName("$['order-number']")).toBe('orderNumber');
		expect(suggestName('$.items[0].sku')).toBe('sku');
		expect(suggestName('$.access_token')).toBe('token');
		expect(suggestName('$[0]')).toBe('value');
	});

	it('is the local name of an XPath’s last step', () => {
		expect(suggestName('//inv:GetInvoiceResponse/inv:invoice/inv:total')).toBe('total');
		expect(suggestName('//inv:invoice/@id')).toBe('invoiceId');
		expect(suggestName('/r/line[2]')).toBe('line');
	});
});

describe('a response', () => {
	it('reads cookies, sizes and tones', () => {
		expect(cookiesOf([['Set-Cookie', 'sid=abc; Path=/; HttpOnly'], ['set-cookie', 'flag'], ['Server', 'x']])).toEqual([
			{ name: 'sid', value: 'abc', attributes: 'Path=/; HttpOnly' },
			{ name: 'flag', value: '', attributes: '' }
		]);
		expect(formatSize(900)).toBe('900 B');
		expect(formatSize(1434)).toBe('1.4 KB');
		expect(statusTone(201)).toBe('ok');
		expect(statusTone(404)).toBe('warn');
		expect(statusTone(503)).toBe('danger');
	});
});
