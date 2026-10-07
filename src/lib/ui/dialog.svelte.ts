// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Asking the user something, from anywhere.
 *
 * Every destructive git operation has to be confirmed and several need a name
 * typed. Both are the same shape — put a question on screen, wait, resolve with
 * the answer — so they are one store with two entry points rather than a modal
 * component threaded through every screen that might need one.
 *
 * The store holds a promise's `resolve`, which is what lets an action read like
 * the thing it is:
 *
 * ```ts
 * if (!(await dialog.confirm({ … }))) return;
 * await api.reset(id, 'hard');
 * ```
 *
 * There is deliberately no queue. A second question while one is open replaces
 * the first, answering it as cancelled: two stacked modals is not a state any
 * of this is worth supporting, and silently dropping the second would leave its
 * caller awaiting forever.
 */

export interface Question {
	kind: 'confirm' | 'prompt' | 'choice';
	title: string;
	/** The sentence under the title. Say what will happen, in plain words. */
	body: string;
	/** The affirmative button's text: "Delete", "Reset", never "OK". */
	confirmLabel: string;
	/** Paints the affirmative button as destructive and requires a second look. */
	danger: boolean;
	/** Prompt only: the field's label. */
	label?: string;
	/** Prompt only: what the field starts with. */
	value?: string;
	placeholder?: string;
	/** Choice only: the middle way's button, between Cancel and the affirmative. */
	alternativeLabel?: string;
}

/** What a three-way question answers (FEAT-107). */
export type Choice = 'confirm' | 'alternative' | 'cancel';

type Answer = boolean | string | null;

let question = $state<Question | null>(null);
let draft = $state('');
let resolver: ((answer: Answer) => void) | null = null;

/**
 * What a question answers with when it does not get a real answer.
 *
 * A confirmation says no; a prompt says nothing was typed. One definition,
 * because a replaced question and a dismissed one are the same event from the
 * caller's side and must not be able to disagree about it.
 */
function cancelValue(open: Question): Answer {
	if (open.kind === 'choice') return 'cancel';
	return open.kind === 'prompt' ? null : false;
}

function ask(next: Question): Promise<Answer> {
	// Whatever was open loses; its caller is told so rather than left hanging.
	//
	// BUG-007: it is told with *its own* cancel value, not the incoming
	// question's. Every prompt caller guards on `null` exactly — `if (name ===
	// null) return` — so a prompt replaced by a confirmation used to resolve
	// `false`, walk straight past that guard, and reach `api.createBranch` with
	// a boolean where a branch name belongs.
	if (resolver && question) {
		const resolve = resolver;
		const answer = cancelValue(question);
		resolver = null;
		resolve(answer);
	}

	question = next;
	draft = next.value ?? '';

	return new Promise<Answer>((resolve) => {
		resolver = resolve;
	});
}

function settle(answer: Answer): void {
	const resolve = resolver;
	resolver = null;
	question = null;
	draft = '';
	resolve?.(answer);
}

export const dialog = {
	get question(): Question | null {
		return question;
	},
	/** The prompt field's current contents. */
	get draft(): string {
		return draft;
	},
	/** True when the affirmative button would do nothing — an empty required name. */
	get blocked(): boolean {
		return question?.kind === 'prompt' && draft.trim() === '';
	},

	setDraft(next: string): void {
		draft = next;
	},

	/** Ask a yes/no question. Resolves false when dismissed. */
	confirm(options: {
		title: string;
		body: string;
		confirmLabel: string;
		danger?: boolean;
	}): Promise<boolean> {
		return ask({
			kind: 'confirm',
			title: options.title,
			body: options.body,
			confirmLabel: options.confirmLabel,
			danger: options.danger ?? false
		}) as Promise<boolean>;
	},

	/** Ask for a line of text. Resolves null when dismissed. */
	prompt(options: {
		title: string;
		body: string;
		label: string;
		confirmLabel: string;
		value?: string;
		placeholder?: string;
		danger?: boolean;
	}): Promise<string | null> {
		return ask({
			kind: 'prompt',
			title: options.title,
			body: options.body,
			confirmLabel: options.confirmLabel,
			danger: options.danger ?? false,
			label: options.label,
			value: options.value ?? '',
			placeholder: options.placeholder
		}) as Promise<string | null>;
	},

	/**
	 * Ask a question with a middle way (FEAT-107): Cancel, the alternative, or
	 * the affirmative. Resolves 'cancel' when dismissed.
	 */
	choose(options: {
		title: string;
		body: string;
		confirmLabel: string;
		alternativeLabel: string;
		danger?: boolean;
	}): Promise<Choice> {
		return ask({
			kind: 'choice',
			title: options.title,
			body: options.body,
			confirmLabel: options.confirmLabel,
			alternativeLabel: options.alternativeLabel,
			danger: options.danger ?? false
		}) as Promise<Choice>;
	},

	/** Take the middle way of a choice. */
	alternative(): void {
		if (question?.kind === 'choice') settle('alternative');
	},

	/** Answer affirmatively: true, the typed text, or 'confirm' for a choice. */
	accept(): void {
		if (!question) return;
		if (question.kind === 'choice') {
			settle('confirm');
		} else if (question.kind === 'prompt') {
			const text = draft.trim();
			if (text === '') return;
			settle(text);
		} else {
			settle(true);
		}
	},

	/** Dismiss. A prompt answers null, a confirmation answers false. */
	dismiss(): void {
		if (!question) return;
		settle(cancelValue(question));
	}
};
