// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A SOAP call (FEAT-163): the version, the operation picker, the shown
 * header, WS-Security, *Rebuild from WSDL*, and a fault in the response.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render, type Mounted } from '../../testing/mount';
import type { Call } from './types';

const { notice } = vi.hoisted(() => ({ notice: { ok: vi.fn(), failed: vi.fn() } }));

vi.mock('$lib/ui/notice.svelte', () => ({ notice, describe: (error: unknown) => String(error) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn(async () => null) }));
vi.mock('./api', () => ({
	wsdl: vi.fn(async () => ({
		services: [
			{
				name: 'BillingService',
				ports: [
					{
						name: 'Invoices11Port',
						version: '1.1',
						address: 'https://billing.example.com/ws/invoices',
						operations: [
							{ name: 'GetInvoice', action: 'urn:GetInvoice', rpc: false },
							{ name: 'ListInvoices', action: 'urn:ListInvoices', rpc: false }
						]
					}
				]
			}
		]
	})),
	rebuild: vi.fn(async () => ['<soapenv:Envelope>rebuilt</soapenv:Envelope>', 'urn:ListInvoices'])
}));

import * as api from './api';
import ResponseView from './ResponseView.svelte';
import SoapEditor from './SoapEditor.svelte';

const CALL: Call = {
	id: 'c_1',
	name: 'GetInvoice',
	kind: 'soap',
	url: '{{baseUrl}}/ws/invoices',
	service: 'BillingService',
	port: 'Invoices11Port',
	operation: 'GetInvoice',
	version: '1.1',
	action: 'urn:GetInvoice',
	envelope: '<soapenv:Envelope><inv:invoiceId>{{invoiceId}}</inv:invoiceId></soapenv:Envelope>',
	wsSecurity: { mode: 'username', user: '{{user}}', password: '{{password}}' }
};

let view: Mounted | null = null;
let call: Call = CALL;

async function settle(): Promise<void> {
	for (let i = 0; i < 4; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	flushSync();
}

function show(start: Call = CALL, wsdl: string | null = 'wsdl/invoices.wsdl') {
	call = start;
	view = render(SoapEditor, {
		call: start,
		wsdl,
		onchange: (change: (c: Call) => Call) => {
			call = change(call);
		},
		find: () => null,
		names: [],
		sending: false,
		refusal: null,
		onsend: vi.fn(),
		oncancel: vi.fn()
	});
	return view;
}

const button = (text: string) =>
	[...document.querySelectorAll('button')].find((b) => b.textContent?.replace(/\s+/g, ' ').trim().startsWith(text)) as HTMLButtonElement;

beforeEach(() => {
	vi.clearAllMocks();
});

afterEach(() => {
	view?.destroy();
	view = null;
	document.body.innerHTML = '';
});

describe('a SOAP call', () => {
	it('shows its version, operation and tabs, and the header the version needs', () => {
		show();
		expect(view!.get('.version').textContent?.replace(/\s+/g, ' ').trim()).toBe('SOAP 1.1 ▾');
		expect(view!.get('.operation').textContent?.replace(/\s+/g, ' ').trim()).toBe('BillingService › GetInvoice ▾');
		expect(view!.all('[role="tab"]').map((tab) => tab.textContent?.trim())).toEqual(['Envelope', 'Headers', 'WS-Security', 'Keep']);
		expect((view!.get('textarea[aria-label="Envelope"]') as HTMLTextAreaElement).value).toContain('{{invoiceId}}');
		click(view!.all('[role="tab"]')[1]);
		expect(view!.get('.shown').textContent).toBe('SOAPAction: "urn:GetInvoice"');
	});

	it('says how 1.2 carries the action', () => {
		show({ ...CALL, version: '1.2' });
		click(view!.all('[role="tab"]')[1]);
		expect(view!.get('.shown').textContent).toBe('Content-Type: application/soap+xml; charset=utf-8; action="urn:GetInvoice"');
	});

	it('picks another operation from the WSDL, and rebuilds its envelope', async () => {
		show();
		click(view!.get('.operation'));
		await settle();
		expect(api.wsdl).toHaveBeenCalledWith('wsdl/invoices.wsdl');
		click(button('ListInvoices'));
		await settle();
		expect(api.rebuild).toHaveBeenCalledWith({
			wsdl: 'wsdl/invoices.wsdl',
			service: 'BillingService',
			port: 'Invoices11Port',
			operation: 'ListInvoices',
			version: '1.1',
			envelope: null
		});
		expect(call).toMatchObject({
			operation: 'ListInvoices',
			name: 'ListInvoices',
			action: 'urn:ListInvoices',
			envelope: '<soapenv:Envelope>rebuilt</soapenv:Envelope>'
		});
	});

	it('rebuilds the same operation keeping what was typed', async () => {
		show();
		click(button('Rebuild from WSDL'));
		await settle();
		expect(vi.mocked(api.rebuild).mock.calls[0][0]).toMatchObject({ operation: 'GetInvoice', envelope: CALL.envelope });
	});

	it('offers None and Username token, with Signed drawn and not offered', () => {
		show();
		click(view!.all('[role="tab"]')[2]);
		expect(button('Signed').disabled).toBe(true);
		expect((view!.get('input[aria-label="WS-Security user"]') as HTMLInputElement).value).toBe('{{user}}');
		click(button('Digest'));
		expect(call.wsSecurity).toMatchObject({ mode: 'username', digest: true });
		click(button('None'));
		expect(call.wsSecurity?.mode).toBe('none');
	});
});

describe('a SOAP response', () => {
	const response = (fault: boolean) => ({
		status: 200,
		reason: 'OK',
		headers: [['Content-Type', 'text/xml']] as [string, string][],
		millis: 30,
		size: 200,
		truncated: false,
		shape: 'xml' as const,
		mediaType: 'text/xml',
		text: '<soapenv:Envelope xmlns:soapenv="x"><soapenv:Body><ok/></soapenv:Body></soapenv:Envelope>',
		plain: false,
		url: 'u',
		fault: fault ? { code: 'soapenv:Client', reason: 'Invoice INV-9 not found', detail: '<inv:id>INV-9</inv:id>' } : null
	});

	function answer(fault: boolean) {
		view = render(ResponseView, {
			outcome: { sendId: 's', response: response(fault), error: null, cancelled: false, sent: null, streamed: '', raw: false },
			sending: false,
			beautify: true,
			name: 'GetInvoice',
			ontoggleraw: vi.fn(),
			onsave: vi.fn(),
			soap: true
		});
	}

	it('shows a fault as an error even on 200', () => {
		answer(true);
		expect(view!.get('.fault-tag').textContent).toBe('Fault · soapenv:Client');
		expect(view!.get('.fault').textContent?.replace(/\s+/g, ' ').trim()).toBe('soapenv:Client Invoice INV-9 not found <inv:id>INV-9</inv:id>');
	});

	it('says no fault otherwise', () => {
		answer(false);
		expect(view!.find('.fault')).toBeNull();
		expect(view!.text()).toContain('no fault');
	});
});
