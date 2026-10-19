// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A CI log as lines of styled text.
 *
 * GitLab traces and GitHub logs are terminal output: colour codes, erase-line
 * codes, carriage returns that redraw a progress line, and each host's own
 * section markers (`section_start:…` on GitLab, `##[group]` on GitHub). Shown
 * raw they are noise between every word. Colours map to the theme's tokens,
 * so a log reads the same in both themes; anything that moves a cursor is
 * dropped.
 */

/** A theme tone, named by the token it is drawn with. */
export type LogTone = 'danger' | 'ok' | 'warn' | 'blue' | 'violet' | 'cyan' | 'muted';

export interface LogSpan {
	text: string;
	tone?: LogTone;
	bold?: boolean;
}

export interface LogLine {
	index: number;
	spans: LogSpan[];
	/** A section's title: GitLab `section_start`, GitHub `##[group]`. */
	heading?: boolean;
}

const TONES: Record<number, LogTone | undefined> = {
	30: 'muted', 31: 'danger', 32: 'ok', 33: 'warn', 34: 'blue', 35: 'violet', 36: 'cyan', 37: undefined,
	90: 'muted', 91: 'danger', 92: 'ok', 93: 'warn', 94: 'blue', 95: 'violet', 96: 'cyan', 97: undefined
};

// Escape sequences, written with \u001b so the source holds no control byte.
const OSC = /\u001b\][^\u0007\u001b]*(?:\u0007|\u001b\\)/g;
const CSI = /\u001b\[([0-9;?]*)([A-Za-z])/g;
const SECTION = /section_(start|end):\d+:[A-Za-z0-9_.-]+(?:\[[^\]\r\n]*\])?\r?(?:\u001b\[0K)?/g;
const STAMP = /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z /;

interface Style {
	tone?: LogTone;
	bold?: boolean;
}

/** Apply one `m` sequence's parameters to the running style. */
function sgr(style: Style, params: string): Style {
	const codes = params === '' ? [0] : params.split(';').map((p) => Number(p) || 0);
	let next = { ...style };
	for (let i = 0; i < codes.length; i++) {
		const code = codes[i];
		if (code === 0) next = {};
		else if (code === 1) next.bold = true;
		else if (code === 22) next.bold = false;
		else if (code === 39) next.tone = undefined;
		else if (code in TONES) next.tone = TONES[code];
		// 256-colour and true-colour forms carry their own arguments; skip them.
		else if (code === 38 || code === 48) i += codes[i + 1] === 5 ? 2 : codes[i + 1] === 2 ? 4 : 0;
	}
	return next;
}

/** One raw line into spans, carrying the style across lines as a terminal does. */
function spansOf(raw: string, style: Style): { spans: LogSpan[]; style: Style } {
	const spans: LogSpan[] = [];
	let current = style, at = 0;
	const push = (text: string) => {
		if (!text) return;
		const last = spans.at(-1);
		if (last && last.tone === current.tone && last.bold === current.bold) last.text += text;
		else spans.push({ text, tone: current.tone, bold: current.bold });
	};
	for (const match of raw.matchAll(CSI)) {
		push(raw.slice(at, match.index));
		if (match[2] === 'm') current = sgr(current, match[1]);
		at = match.index! + match[0].length;
	}
	push(raw.slice(at));
	return { spans, style: current };
}

export function parseLog(text: string): LogLine[] {
	const lines: LogLine[] = [];
	let style: Style = {};
	for (let raw of text.split('\n')) {
		raw = raw.replace(/\r$/, '').replace(OSC, '');
		let heading = false;
		// GitLab sections: the start marker's line is the section's title, and
		// an end marker on a line of its own leaves nothing to show.
		if (raw.includes('section_')) {
			heading = /section_start:/.test(raw);
			const ended = /section_end:/.test(raw);
			raw = raw.replace(SECTION, '');
			if (ended && !raw.replace(CSI, '').trim()) continue;
		}
		// A carriage return redraws the line: only what was drawn last is seen.
		if (raw.includes('\r')) raw = raw.slice(raw.lastIndexOf('\r') + 1);
		// GitHub: a timestamp on every line, and its own markers.
		raw = raw.replace(STAMP, '');
		let tone: LogTone | undefined;
		const marker = /^##\[(group|endgroup|error|warning|notice|debug|command)\]/.exec(raw);
		if (marker) {
			if (marker[1] === 'endgroup') continue;
			raw = raw.slice(marker[0].length);
			heading = marker[1] === 'group';
			tone = marker[1] === 'error' ? 'danger' : marker[1] === 'warning' ? 'warn' : marker[1] === 'command' ? 'blue' : marker[1] === 'debug' ? 'muted' : undefined;
		}
		const parsed = spansOf(raw, style);
		style = parsed.style;
		const spans = tone ? parsed.spans.map((span) => ({ ...span, tone: span.tone ?? tone })) : parsed.spans;
		lines.push({ index: lines.length, spans, heading: heading || undefined });
	}
	while (lines.length && !lines.at(-1)!.spans.some((s) => s.text.trim())) lines.pop();
	return lines;
}
