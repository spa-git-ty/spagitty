// SPDX-License-Identifier: GPL-3.0-or-later

import pkg from '../../package.json';

/**
 * Build identity, shown in the title bar and (in full) in Settings -> About.
 *
 * GPL-3 asks that a user can get the source corresponding to the exact build
 * they are running, so the commit is stamped in at compile time by
 * `src-tauri/build.rs` and read back through the `about` command. The values
 * here are the fallbacks used before that call resolves, and in a plain browser.
 *
 * The number is the package's own, read at build time: a typed one fell
 * behind the release and the status strip said 0.1.0 under 1.0.1.
 */
export const version = {
	number: pkg.version,
	/** The SPDX identifier. The authoritative one. */
	license: 'GPL-3.0-or-later',
	/** Abbreviated for the title bar, where there is room for eight characters. */
	licenseShort: 'GPL-3.0',
	/** Replaced by the real commit once `about()` resolves. */
	commit: 'unknown'
};
