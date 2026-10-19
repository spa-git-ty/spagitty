// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Which cards Home shows, and in what order (FEAT-148).
 *
 * Kept as the rail's choice is (FEAT-134): the cards hidden and the order
 * chosen, by id, as strings. A file written by a newer build that names a card
 * this one has not got keeps the rest of its list, and a card this build adds
 * arrives where it was designed to be rather than at the end.
 */

export type HomeCardId =
	| 'needs'
	| 'farm'
	| 'working'
	| 'branches'
	| 'requests'
	| 'commits'
	| 'heat'
	| 'pipelines'
	| 'releases';

export interface HomeCard {
	id: HomeCardId;
	label: string;
	/** Two columns and two rows wide, where there are three columns. */
	wide?: boolean;
}

/** The order Home was designed in (2A). */
export const HOME_CARDS: HomeCard[] = [
	{ id: 'needs', label: 'Needs you', wide: true },
	{ id: 'farm', label: 'Farm' },
	{ id: 'working', label: 'Working copy' },
	{ id: 'branches', label: 'Branches' },
	{ id: 'requests', label: 'Pull requests' },
	{ id: 'commits', label: 'Recent commits' },
	{ id: 'heat', label: 'Commits, last 16 weeks' },
	{ id: 'pipelines', label: 'Pipelines' },
	{ id: 'releases', label: 'Releases' }
];

export interface HomeLayout {
	hidden: string[];
	order: string[];
}

/** Every card, in the designed order. */
export const HOME_DEFAULT: HomeLayout = { hidden: [], order: [] };

/** Every card in the order chosen, hidden ones included. */
export function inHomeOrder(order: string[], cards: HomeCard[] = HOME_CARDS): HomeCard[] {
	if (order.length === 0) return cards;
	const placed = cards
		.filter((card) => order.includes(card.id))
		.sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
	cards.forEach((card, index) => {
		if (placed.includes(card)) return;
		const before = cards
			.slice(0, index)
			.reverse()
			.find((other) => placed.includes(other));
		placed.splice(before ? placed.indexOf(before) + 1 : 0, 0, card);
	});
	return placed;
}

/** The cards Home draws, in order. */
export function shownCards(layout: HomeLayout): HomeCard[] {
	return inHomeOrder(layout.order).filter((card) => !layout.hidden.includes(card.id));
}

/** The order to store: nothing when it is the designed order. */
function orderOf(arranged: HomeCard[]): string[] {
	return arranged.every((card, index) => card.id === HOME_CARDS[index]?.id)
		? []
		: arranged.map((card) => card.id);
}

/** The order after moving `id` by `delta` places. */
export function moveCardBy(id: string, delta: number, order: string[]): string[] {
	const arranged = [...inHomeOrder(order)];
	const from = arranged.findIndex((card) => card.id === id);
	const to = from + delta;
	if (from === -1 || to < 0 || to >= arranged.length) return order;
	const [moving] = arranged.splice(from, 1);
	arranged.splice(to, 0, moving);
	return orderOf(arranged);
}

/** The order after dropping `id` on `target`'s row. */
export function moveCardOnto(id: string, target: string, order: string[]): string[] {
	const arranged = inHomeOrder(order);
	const from = arranged.findIndex((card) => card.id === id);
	const to = arranged.findIndex((card) => card.id === target);
	if (from === -1 || to === -1) return order;
	return moveCardBy(id, to - from, order);
}
