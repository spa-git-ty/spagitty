// SPDX-License-Identifier: GPL-3.0-or-later

/** The portal on a network (`--phone`): the key, or nothing (TASK-073). */

import { describe, expect, it } from 'vitest';
import { admit } from './serve';

const KEY = 'k'.repeat(24);
const AT = 'http://10.0.0.2:5199';

describe('the portal with --phone', () => {
	it('lets in a request that holds the key in its cookie', () => {
		expect(admit(`${AT}/api/sweeps`, `other=1; sweep-key=${KEY}`, KEY)).toEqual({ pass: true });
	});

	it('takes the key out of the address, to be kept in a cookie', () => {
		expect(admit(`${AT}/?key=${KEY}`, null, KEY)).toEqual({ pass: false, to: '/' });
		expect(admit(`${AT}/?show=all&key=${KEY}`, null, KEY)).toEqual({ pass: false, to: '/?show=all' });
	});

	it('turns away a request without the key, or with a wrong one', () => {
		expect(admit(`${AT}/`, null, KEY)).toEqual({ pass: false, to: null });
		expect(admit(`${AT}/?key=wrong`, null, KEY)).toEqual({ pass: false, to: null });
		expect(admit(`${AT}/api/result`, 'sweep-key=wrong', KEY)).toEqual({ pass: false, to: null });
	});
});
