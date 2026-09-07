// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The window's manifest, and the one Linux override on it (BUG-029).
 *
 * Tauri merges `tauri.linux.conf.json` over `tauri.conf.json` when it builds
 * for Linux, and the merge replaces arrays wholesale rather than element by
 * element — so the Linux override cannot say "the same window, but opaque".
 * It has to repeat the whole window object, and a repeated object is an object
 * that drifts: a width changed in one file and not the other is a Linux build
 * that opens at a size nobody chose, found by somebody on Linux, months later.
 *
 * So the duplication is allowed and pinned. This file is the only place that
 * says *why* it exists at all, because JSON cannot hold the comment.
 *
 * # Why Linux is opaque
 *
 * `transparent: true` exists so the window can draw its own rounded corner and
 * let the desktop show through outside it. On Linux nothing wants that: the
 * compositor draws the corner, the border and the shadow itself, so Spagitty's
 * card sat *inside* the compositor's, and the transparent margin between them
 * was filled by Hyprland's blur — a band of smeared desktop around the app, in
 * the packaged AppImage on Omarchy.
 *
 * `src/lib/chrome/window.ts` stops drawing the inner card, which is what makes
 * the picture right. This makes it right *cheaply*: an opaque window is one
 * the compositor need not blur behind or blend every frame, and on Linux every
 * frame is already rasterized on the CPU (see `platform.rs`). It also closes
 * the door on the WebKitGTK failure mode recorded there, where an accelerated
 * transparent window loses its buffer and goes invisible.
 */

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

interface WindowConfig {
	title: string;
	width: number;
	height: number;
	resizable: boolean;
	fullscreen: boolean;
	decorations: boolean;
	transparent: boolean;
}

const read = (path: string) =>
	JSON.parse(readFileSync(path, 'utf8')) as { app: { windows: WindowConfig[] } };

const base = read('src-tauri/tauri.conf.json').app.windows[0];
const linux = read('src-tauri/tauri.linux.conf.json').app.windows[0];

describe('the Linux window override', () => {
	it('changes transparency and nothing else', () => {
		expect({ ...linux, transparent: base.transparent }).toEqual(base);
	});

	it('is opaque, so the compositor has no margin to blur', () => {
		expect(linux.transparent).toBe(false);
		expect(base.transparent).toBe(true);
	});

	it('still draws its own title bar, which is not what BUG-029 was about', () => {
		// The traffic lights and the drag region are Spagitty's on every host.
		// Only the *edge* moved.
		expect(linux.decorations).toBe(false);
		expect(base.decorations).toBe(false);
	});
});
