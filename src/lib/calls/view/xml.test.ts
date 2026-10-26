// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import type { Line } from './json';
import { formatXml, prefixesOf, unescapeXml } from './xml';

const text = (lines: Line[]) => lines.map((line) => line.map((piece) => piece.text).join(''));
const values = (lines: Line[]) =>
	lines.flatMap((line) => line.flatMap((piece) => (piece.path ? [[piece.path, piece.value]] : [])));

const SOAP =
	'<?xml version="1.0"?><soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:inv="urn:inv">' +
	'<soap:Body><inv:GetInvoiceResponse><inv:invoice id="7"><inv:total>12.50</inv:total>' +
	'<inv:line>a</inv:line><inv:line>b &amp; c</inv:line></inv:invoice></inv:GetInvoiceResponse></soap:Body></soap:Envelope>';

describe('an XML response', () => {
	it('is drawn with two spaces, a lone text on its element’s line', () => {
		expect(text(formatXml('<a><b>1</b><c/><d></d></a>')!.lines)).toEqual([
			'<a>',
			'  <b>1</b>',
			'  <c/>',
			'  <d></d>',
			'</a>'
		]);
	});

	it('starts a path inside a SOAP body with `//`, and numbers siblings that share a name', () => {
		const formatted = formatXml(SOAP)!;
		expect(values(formatted.lines)).toEqual([
			['//inv:GetInvoiceResponse/inv:invoice/@id', '7'],
			['//inv:GetInvoiceResponse/inv:invoice/inv:total', '12.50'],
			['//inv:GetInvoiceResponse/inv:invoice/inv:line[1]', 'a'],
			['//inv:GetInvoiceResponse/inv:invoice/inv:line[2]', 'b & c']
		]);
		expect(formatted.ns).toEqual({ soap: 'http://schemas.xmlsoap.org/soap/envelope/', inv: 'urn:inv' });
		expect(prefixesOf('//inv:GetInvoiceResponse/inv:invoice/inv:total', formatted.ns)).toEqual({ inv: 'urn:inv' });
	});

	it('uses an absolute path outside a SOAP body', () => {
		expect(values(formatXml('<r><x k="v">1</x><![CDATA[ignored]]></r>')!.lines)).toEqual([
			['/r/x/@k', 'v'],
			['/r/x', '1']
		]);
		expect(values(formatXml('<r><![CDATA[<b>]]></r>')!.lines)).toEqual([['/r', '<b>']]);
	});

	it('is null for text that is not well-formed', () => {
		for (const bad of ['', 'plain', '<a>', '<a></b>', '<a><!-- open</a>', '</a>']) {
			expect(formatXml(bad)).toBeNull();
		}
	});

	it('reads entities', () => {
		expect(unescapeXml('&lt;&#38;&#x26;&quot;&bogus;')).toBe('<&&"&bogus;');
	});
});
