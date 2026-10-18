// SPDX-License-Identifier: GPL-3.0-or-later
import { expect, it } from 'vitest';
import { parseLog } from './ansi';

const text = (line: { spans: { text: string }[] }) => line.spans.map((s) => s.text).join('');

it('a GitLab trace loses its control codes and keeps its colours as tones', () => {
	const trace = [
		'\u001b[0KRunning with gitlab-runner 16.11.4 (cf51e113)\u001b[0;m',
		'section_start:1792316273:prepare_executor\r\u001b[0K\u001b[0K\u001b[36;1mPreparing the "docker" executor\u001b[0;m\u001b[0;m',
		'\u001b[32;1mJob succeeded\u001b[0;m',
		'section_end:1792316280:prepare_executor\r\u001b[0K'
	].join('\n');
	const lines = parseLog(trace);
	expect(lines.map(text)).toEqual(['Running with gitlab-runner 16.11.4 (cf51e113)', 'Preparing the "docker" executor', 'Job succeeded']);
	expect(lines[1].heading).toBe(true);
	expect(lines[1].spans[0]).toMatchObject({ tone: 'cyan', bold: true });
	expect(lines[2].spans[0].tone).toBe('ok');
	expect(trace.includes('\u001b') && lines.every((l) => !text(l).includes('\u001b'))).toBe(true);
});

it('a section marker with no carriage return is still removed', () => {
	expect(parseLog('section_start:1:step_script[collapsed=true]Executing step').map(text)).toEqual(['Executing step']);
});

it('a redrawn progress line shows what was drawn last, and colour carries over lines', () => {
	const lines = parseLog('Downloading 10%\rDownloading 100%\n\u001b[31mfirst\nsecond\u001b[0m\nplain');
	expect(lines.map(text)).toEqual(['Downloading 100%', 'first', 'second', 'plain']);
	expect(lines[2].spans[0].tone).toBe('danger');
	expect(lines[3].spans[0].tone).toBeUndefined();
});

it('GitHub timestamps and markers become plain lines, headings and tones', () => {
	const lines = parseLog('2026-10-18T10:00:00.1234567Z ##[group]Run tests\n2026-10-18T10:00:01.0000000Z ok\n2026-10-18T10:00:02.0000000Z ##[endgroup]\n2026-10-18T10:00:03.0000000Z ##[error]Process completed with exit code 1.');
	expect(lines.map(text)).toEqual(['Run tests', 'ok', 'Process completed with exit code 1.']);
	expect(lines[0].heading).toBe(true);
	expect(lines[2].spans[0].tone).toBe('danger');
});

it('256-colour arguments are skipped rather than read as colours', () => {
	const [line] = parseLog('\u001b[38;5;31mx\u001b[0m');
	expect(line.spans[0].tone).toBeUndefined();
});
