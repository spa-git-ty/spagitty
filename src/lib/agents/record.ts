// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The record of one step, ready to draw (FEAT-118).
 *
 * The kind of each line is set where the stream is read, in Rust; this only
 * groups what belongs together. An agent's reply arrives a line at a time, and
 * a reply drawn as Markdown has to be drawn whole, so adjacent reply lines are
 * one block.
 */

import type { EventKind, Step, StepEvent } from './types';

export interface RecordBlock {
	kind: EventKind;
	text: string;
	/** The file it is about, for a read or an edit. */
	path?: string;
}

/** The step's record as blocks: adjacent reply lines become one reply. */
export function blocksOf(step: Pick<Step, 'record'>): RecordBlock[] {
	const blocks: RecordBlock[] = [];
	for (const event of step.record ?? []) {
		const last = blocks.at(-1);
		if (event.kind === 'reply' && last?.kind === 'reply') {
			last.text += '\n' + event.text;
			continue;
		}
		blocks.push({ kind: event.kind, text: event.text, path: event.path });
	}
	return blocks;
}

/** What each kind is called above its block. */
export const KIND_LABELS: Record<EventKind, string> = {
	sent: 'You',
	command: 'Command',
	tool: 'Tool',
	read: 'Read',
	reply: 'Agent',
	result: 'Result',
	error: 'Error'
};

/** A line of the step as the person would copy it. */
export function plainOf(event: StepEvent): string {
	return event.text;
}

/** How many lines a prompt is, for the label above it. */
export function lineCount(text: string): number {
	return text.length === 0 ? 0 : text.split('\n').length;
}
