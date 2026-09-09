// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Structural metrics — the single source of truth.
 *
 * Everything that depends on the commit-row pitch reads ROW_PITCH from here:
 * the virtualized row list, the refs gutter, the lane canvas geometry, and the
 * stylesheet (via `applyMetrics`). There is no second `30` anywhere in the
 * frontend, and no `height: 30px` in any component.
 *
 * The Rust side mirrors ROW_PITCH in `crates/spagitty-core/src/graph.rs` because
 * lane elbows are described in row units there; that mirror is asserted against
 * this value by `row_pitch_matches_the_frontend`, a test in that module which
 * reads this file and fails if the two ever drift.
 */

/** Height of one commit row, in CSS pixels. The graph's fundamental unit. */
export const ROW_PITCH = 30;

/**
 * Horizontal distance between two lanes.
 *
 * This number is set by the node, not the other way round: a lane closer than
 * a node is wide draws lines through faces. FEAT-023 put an author's portrait
 * on the node, so `2 × NODE_R + LANE_STROKE` is 24.5 and the pitch has to clear
 * it — 26 leaves a pixel and a half of background between two adjacent heads at
 * 100%.
 *
 * It was 15 while nodes were 11px initials discs, itself retuned down from 24
 * after measuring Spagitty against GitKraken on the same repository, then 22
 * at the first portrait size. Going back up costs width, and the trade is
 * deliberate: a graph whose nodes say *who* earns the pixels, and the message
 * column is still the wider of the two at five lanes.
 */
export const LANE_PITCH = 26;

/**
 * x of lane 0. Lanes therefore sit at 16, 42, 68, 94, 120.
 *
 * At least `NODE_R`, or the first lane's portrait is clipped by the column's
 * own left edge.
 */
export const LANE_X0 = 16;

/**
 * Radius of a commit node — the author's portrait.
 *
 * Twenty-two pixels across, plus a 2px ring in the column's own colour: large
 * enough that a generated face reads as a face at a glance rather than as a
 * coloured dot, while still leaving daylight between two stacked heads at the
 * 30px row pitch. A pixel larger and the column reads as a solid stripe of
 * faces.
 */
export const NODE_R = 11;

/**
 * Radius of a merge node.
 *
 * A merge is not a person's work in the way a commit is — it is the moment two
 * lines join — so it is drawn as a plain dot in the lane colour rather than as
 * a face, which is what the reference does and what makes a merge findable by
 * scanning. Well under half the portrait's radius, so it reads as a smaller
 * kind of event; it did not grow with the portrait, because a merge dot large
 * enough to match would start competing with the faces around it.
 */
export const MERGE_R = 4.5;

/**
 * Stroke width of a lane line.
 *
 * Widened with the nodes: a 2px line arriving at an 18px head looks like a
 * thread tied to a stone. 2.5 keeps the lane readable against the graph
 * column's own fill without turning two adjacent lanes into one band.
 */
export const LANE_STROKE = 2.5;

/**
 * Lane columns the design specifies, and the width the column is sized for by
 * default. Ordinary repositories never exceed this.
 */
export const LANE_COLUMNS_MIN = 5;

/**
 * Hard cap on lane columns.
 *
 * Real histories go well past the design's five. Measured on `cli/cli` (12,896
 * commits, 3,189 merges — an ordinary PR-merge workflow), by how many 40-row
 * viewports draw every lane they contain without clamping:
 *
 *     cap   lane col   message col @1280   viewports fully drawn
 *      5      149px          489px                  1.6%
 *      8      227px          411px                 15.8%
 *     10      279px          359px                 37.3%
 *     12      331px          307px                 61.8%   <- chosen
 *     14      383px          255px                 76.7%
 *     16      435px          203px                 87.3%
 *     20      539px           99px                 94.4%
 *
 * Twelve is the knee: it buys most of the improvement, and the last column
 * before the curve flattens. The viewport percentages are a property of the
 * history, so they held when FEAT-029 enlarged the portraits; the widths are
 * not, and they moved. The cap was originally also the widest column that still
 * left the message column the wider of the two — at the enlarged node that
 * crossover sits at eleven, so twelve now spends 24px more on lanes than on
 * messages. Kept at twelve deliberately: losing a whole lane column costs more
 * than those 24px buy back.
 *
 * Some histories defeat any cap. `git/git` needs a mean lane depth of 187 and
 * peaks at 382, because hundreds of topic branches interleave in date order;
 * git's own graph reaches 190 columns there.
 *
 * **This is a cap on width, not on lanes** (FEAT-035). Past it the column stops
 * growing and the *pitch* gives instead, so a thirteenth lane is drawn slightly
 * closer to its neighbour rather than on top of it. See [`lanePitch`].
 */
export const LANE_COLUMNS_MAX = 12;

/**
 * How far the lanes are laid out across, from lane 0 to the rightmost one.
 *
 * The one number compression works within: whatever the lane count, the last
 * lane lands here, so the column's width never depends on how busy the history
 * is. Derived rather than written down, so it cannot disagree with the cap.
 */
export const LANE_SPAN = (LANE_COLUMNS_MAX - 1) * LANE_PITCH;

/**
 * Tightest the lanes may be squeezed before compression stops helping.
 *
 * **This was 6, and 6 was wrong.** The reasoning was that a
 * [`LANE_STROKE`]-wide line at a 6px pitch keeps 3.5px of daylight — "thin, but
 * two lines rather than a band" — and that 48 tellable-apart lanes beat the
 * twelve the old clamp gave.
 *
 * A screenshot of `git/git` disproved it. At a mean lane depth of 187 the span
 * fills with 48 lines 3.5px apart and the column is not a graph, it is a
 * picket fence: no lane can be followed from one row to the next, and the
 * 3.5px of daylight that made the argument work on paper is not daylight on a
 * real display at a real size. The arithmetic was right and the premise was
 * not.
 *
 * 14 is half the design pitch, which leaves 11.5px between two strokes — a gap
 * you can see rather than one you can calculate. The span then holds 21 lanes
 * instead of 48.
 *
 * Twenty-one is fewer, and that is the trade taken deliberately: a history deep
 * enough to need a twenty-second lane is a history where the extra lane was
 * never going to be readable, and folding it is more honest than drawing it
 * indistinguishably. `git/git` still overflows — 382 lanes defeat any width,
 * and every client folds there, including the one this was compared against.
 * The difference is that the twenty-one which *are* drawn can now be told
 * apart.
 */
export const LANE_PITCH_MIN = 14;

/** Highest lane index the span can still draw at a distinct x. */
export const LANE_INDEX_MAX = Math.floor(LANE_SPAN / LANE_PITCH_MIN);

/**
 * How much room the graph column asks for (TASK-041).
 *
 * Three numbers, and they are the three the column's width is made of: the
 * distance between lanes, the size of a node, and how many lanes the column is
 * sized for before any history is looked at. Everything else in this file is
 * derived from them.
 *
 * They are a **set** rather than three settings, because they are not
 * independent. The whole argument for [`LANE_PITCH`] is that a lane closer than
 * a node is wide draws lines through faces — so a smaller pitch that kept the
 * portrait would be a column of overlapping heads, and a smaller portrait that
 * kept the pitch would waste the space it saved. One choice, three consequences.
 *
 * `comfortable` is what every release before this one drew, unchanged.
 *
 * `compact` is for the case the design's numbers are worst at: a history that
 * is two or three lanes deep, on a 1280 window, where five lanes' worth of
 * column is reserved for lanes that are not there and the commit subject — the
 * thing the screen exists to show — gets what is left. Three columns rather
 * than five, a 16px pitch and a 6px node: the node stops being a portrait and
 * becomes a mark, which is the trade being offered rather than a bug. The
 * portrait is still shown wherever there is room for a face to be a face — the
 * author column, the commit detail — which is FEAT-079's own argument for
 * where a picture belongs.
 *
 * 16 is above [`LANE_PITCH_MIN`] on purpose: compression is still allowed to
 * happen underneath a compact column, and a resting pitch at or below the floor
 * would mean a deep history in compact mode had nothing left to give.
 */
export interface Density {
	/** Distance between two lanes at rest, before any compression. */
	pitch: number;
	/** Radius of a commit's node at rest. */
	node: number;
	/** Lane columns the column is sized for when the history needs fewer. */
	columnsMin: number;
}

export const COMFORTABLE: Density = {
	pitch: LANE_PITCH,
	node: NODE_R,
	columnsMin: LANE_COLUMNS_MIN
};

export const COMPACT: Density = { pitch: 16, node: 6, columnsMin: 3 };

/**
 * The span a density lays its lanes out across at rest.
 *
 * [`LANE_SPAN`] is this for the comfortable density and is kept as its own
 * constant because half the graph's call sites default to it. A compact column
 * is narrower by construction, so its resting span is narrower too — and the
 * distinction matters in exactly one place: the node's size is decided by how
 * deep the *history* is, deliberately and not by how wide the column has been
 * dragged (FEAT-039). That decision needs a reference span that belongs to the
 * density rather than to the drag, and `LANE_SPAN` would be the wrong one for a
 * compact column by 110 pixels.
 */
export function laneSpanOf(density: Density = COMFORTABLE): number {
	return (LANE_COLUMNS_MAX - 1) * density.pitch;
}

/**
 * Horizontal distance between two lanes, once `needed` of them must fit.
 *
 * At or under the cap this is the design pitch and nothing moves. Past it the
 * lanes share out [`LANE_SPAN`] between them, down to [`LANE_PITCH_MIN`].
 *
 * The alternative — the behaviour this replaced — was to clamp the lane *index*,
 * which drew lanes 13, 14 and 15 at exactly the twelfth lane's x. They did not
 * overflow the column; they were folded onto each other, so a node on lane 15
 * sat precisely where a node on lane 12 did and the graph stopped being a graph
 * at the point a busy history most needs one.
 */
export function lanePitch(
	needed: number,
	span: number = LANE_SPAN,
	density: Density = COMFORTABLE
): number {
	if (needed <= 1) return density.pitch;
	return Math.max(LANE_PITCH_MIN, Math.min(density.pitch, span / (needed - 1)));
}

/**
 * The horizontal room the lanes actually have, inside a column of `width`.
 *
 * The inverse of [`laneColumnWidth`], and the reason the graph column can be
 * dragged at all (FEAT-039). Given a width someone chose, this is what is left
 * for lanes once the first lane's offset, the node's own radius and the tail
 * before the message column are taken out.
 */
export function laneSpanFor(width: number, zoom = 1, density: Density = COMFORTABLE): number {
	const usable = width / Math.max(zoom, 0.01) - LANE_X0 - density.node - LANE_TAIL;
	return Math.max(0, usable);
}

/**
 * Radius of a commit's node once the lanes are compressed.
 *
 * The node is what set [`LANE_PITCH`] in the first place — "a lane closer than a
 * node is wide draws lines through faces" — so a compressed pitch has to bring
 * the node down with it or the thing compression was for is undone by the
 * portraits sitting on top of it.
 *
 * It never shrinks below [`MERGE_R`], which is already this graph's smallest
 * meaningful mark, and that floor is where the guarantee ends: **up to 32 lanes
 * a node fits inside its own pitch, and past that it starts covering its
 * neighbour's.** The alternative is a node that keeps shrinking until it cannot
 * be seen, which loses more than the overlap costs — by then the column is
 * dense enough that the node is the only thing locating a commit at all.
 *
 * Rounded **down**, for two reasons: rounding up could hand back a node wider
 * than the pitch it was derived from, and the radius picks the portrait tile
 * size, so a fractional one would mint a cache entry per scroll.
 */
export function laneNodeRadius(
	needed: number,
	span: number = LANE_SPAN,
	density: Density = COMFORTABLE
): number {
	const pitch = lanePitch(needed, span, density);
	if (pitch >= density.pitch) return density.node;
	return Math.max(MERGE_R, Math.min(density.node, Math.floor((pitch - LANE_STROKE) / 2)));
}

/**
 * Slack between the rightmost node and the message column.
 *
 * The design's 28px was slack at a 24px pitch; kept while the pitch was retuned
 * down to 15 it would have been nearly two whole lanes of empty column, which
 * is the widest single contributor to a graph that looks wider than its
 * history. Eighteen still keeps a node clear of the divider and of the first
 * character of a commit message at today's 26px pitch.
 */
const LANE_TAIL = 18;

/** Clamp a lane count into the range the column can render. */
export function laneColumns(needed: number, density: Density = COMFORTABLE): number {
	return Math.min(Math.max(needed, density.columnsMin), LANE_COLUMNS_MAX);
}

/**
 * Width of the lane column for a given number of lanes, at a given zoom.
 *
 * Rounded to whole pixels: the node radius is fractional, and a column whose
 * CSS width lands on a half pixel puts the canvas and the row cells on
 * different device-pixel boundaries, which shows up as a lane line that is one
 * pixel off the node it is drawn through.
 */
export function laneColumnWidth(
	needed: number,
	zoom = 1,
	density: Density = COMFORTABLE
): number {
	return Math.round(
		(LANE_X0 +
			(laneColumns(needed, density) - 1) * density.pitch +
			density.node +
			LANE_TAIL) *
			zoom
	);
}

/** Number of lane colors in the cycle; a lane keeps its color for its lifetime. */
export const LANE_COLOR_COUNT = 5;

// --- Chrome ---------------------------------------------------------------

export const TITLEBAR_H = 30;

/**
 * The action bar under the tabs.
 *
 * **Fifty, and now forty (TASK-041).** The application frame was 110px before
 * the screen's own header — a 30px title bar, a 30px tab strip and a 50px
 * toolbar — on a window whose default height is 800. That is an eighth of the
 * window spent on chrome before the work starts, and the toolbar was the part
 * with slack in it: its controls are an icon over a label, and the two together
 * measure 15px + 16px at the application's smallest type, plus 3px of padding
 * top and bottom. Thirty-eight. The other twelve pixels were not doing
 * anything.
 *
 * Forty rather than thirty-eight, so the row still has a pixel of air at each
 * end at 100% and does not become the tightest thing on screen. It scales with
 * zoom like every other metric here, so a person who has zoomed to 150% gets a
 * 60px bar rather than a 40px one with 24px controls in it.
 *
 * Going further means taking the labels off, and that is a different decision
 * with a real cost — a row of eight unlabelled glyphs is a row nobody can aim
 * at without hovering. The narrow layout already drops them below 900px, which
 * is the case where the trade is worth making.
 */
export const TOOLBAR_H = 40;
/** The repository tab row, below the title bar (FEAT-044). */
export const TABS_H = 30;
/**
 * The status strip along the bottom of the window (FEAT-043).
 *
 * Shorter than the title bar, because it carries one line of secondary
 * type and nothing clickable. It is chrome the eye should be able to
 * ignore, and a strip as tall as a bar reads as a place where something
 * ought to be happening.
 */
export const STRIP_H = 22;
export const RAIL_W = 186;
export const DETAIL_W = 270;

// --- Graph screen columns -------------------------------------------------

export const REFS_GUTTER_W = 186;

/** The design's lane column width: what five lanes need. */
export const LANE_COL_W = laneColumnWidth(LANE_COLUMNS_MIN);

// --- Diff screen columns --------------------------------------------------

/** The file list beside a commit's diff. */
export const DIFF_FILES_W = 210;

/**
 * Width of a line-number gutter, sized for five digits. Files longer than
 * 99,999 lines push the column wider rather than truncating the number.
 */
export const DIFF_GUTTER_W = 44;

// --- Working copy columns -------------------------------------------------

/** The staged/unstaged column on the Commit screen. */
export const CHANGES_FILES_W = 250;

/** A card on the All repositories screen. The design's 300px grid cell. */
export const REPO_CARD_W = 300;

/** The lookup and blame column on the Log search screen. */
export const SEARCH_SIDE_W = 280;

/** The detail panel on the Pull requests screen. */
export const REQUESTS_DETAIL_W = 300;

// --- Stash screen columns -------------------------------------------------

/**
 * The entries column on the Stash screen (FEAT-034).
 *
 * Wider than a plain list because each entry draws its own two-row lane and
 * then its message beside it. It became a fixed column when the diff pane
 * arrived: two flexible columns beside each other have no divider that means
 * anything.
 */
export const STASH_ENTRIES_W = 280;

/**
 * The Farm's activity drawer (FEAT-074).
 *
 * A hundred and eighty: eight or nine lines of log at the application's
 * monospace size. Enough to read a run's last moves without turning the screen
 * into a terminal, and the drawer is draggable for the times it should be one.
 */
export const FARM_LOG_H = 180;

// --- Lane elbow shape -----------------------------------------------------

/**
 * How tightly a lane turns when it changes column.
 *
 * A branch or merge transition is drawn as a **rounded right angle**: down its
 * own lane, one quarter-turn, straight across, one quarter-turn, down the new
 * lane. This is the radius of those two turns.
 *
 * It used to be a cubic spanning the whole row — a long S that left one lane
 * vertically and arrived at the other vertically, spending the entire row on
 * the crossing. That reads well with a handful of lanes and badly with many:
 * every transition becomes a diagonal sweep, dozens of them overlap in the same
 * band, and there is no straight run left for the eye to follow. The reference
 * client turns square and keeps the runs straight, and the difference is most
 * of why its graph is easier to read at depth.
 *
 * 6px is small enough to read as a corner rather than a curve, and large enough
 * not to look like a mitre joint at the stroke width the lanes are drawn at. It
 * is clamped in [`lanes`] against half the crossing and half the row, so a
 * transition between adjacent lanes at a compressed pitch degrades to a smaller
 * corner rather than to a bulge.
 */
export const ELBOW_RADIUS = 6;

// --- Derived --------------------------------------------------------------

/**
 * Center y of row `i` within the scrolled content.
 *
 * `pitch` is the row height actually in effect — `scale.pitch`, not the design
 * constant — so that zooming moves rows and lanes by the same arithmetic. It
 * defaults to the constant, which is what every test and every unzoomed frame
 * wants.
 */
export function rowCenterY(index: number, pitch: number = ROW_PITCH): number {
	return index * pitch + pitch / 2;
}

/**
 * Center x of a lane, at the pitch `columns` lanes have to share (FEAT-035).
 *
 * `columns` is the **true** number of lanes in view, not a clamped one: that is
 * what decides the pitch, and clamping it before it arrives here is what used to
 * fold the overflow onto the last column.
 *
 * One clamp remains: above [`LANE_INDEX_MAX`] the pitch has hit its floor and
 * there is no room left, so the deepest lanes do stack — the old behaviour, now
 * reached at 48 lanes instead of 12.
 *
 * There used to be a second, flooring the count at [`LANE_COLUMNS_MIN`] so that
 * three lanes would not spread across the whole column. At the design span it
 * did nothing — [`lanePitch`] caps at [`LANE_PITCH`], so two lanes and five sit
 * at identical x. Against a *dragged* span it compressed a two-lane repository
 * as though five lanes had to fit, and the squeeze began long before any two
 * lanes were touching (FEAT-046). The floor belongs to the column's width,
 * where [`laneColumns`] still applies it, and not to where the lanes go.
 *
 * `zoom` scales the horizontal geometry the same way `applyMetrics` scales the
 * CSS widths, so the canvas and the reserved column keep agreeing.
 */
export function laneX(
	lane: number,
	columns: number = LANE_COLUMNS_MIN,
	zoom = 1,
	span: number = LANE_SPAN,
	density: Density = COMFORTABLE
): number {
	const pitch = lanePitch(columns, span, density);

	// How many lanes land on a distinct x. Above the floor the pitch was chosen
	// so that all of them do, by construction; at the floor it is the span that
	// decides. Stated as the two cases rather than re-derived from the pitch,
	// because `floor(286 / 9.2258)` is 30 rather than 31 and the last lane would
	// silently fall a whole column short.
	const drawable =
		pitch > LANE_PITCH_MIN ? columns : Math.floor(span / LANE_PITCH_MIN) + 1;

	const index = Math.min(lane, Math.min(columns, drawable) - 1);
	return (LANE_X0 + Math.max(0, index) * pitch) * zoom;
}

/** CSS variable name of a lane's color. Lane colors cycle. */
export function laneColorVar(colorIndex: number): string {
	return `--lane-${(colorIndex % LANE_COLOR_COUNT) + 1}`;
}

/**
 * Radii, in CSS pixels, keyed by their token name without the `--`.
 *
 * `r-pill` is deliberately absent: a pill is `999px` at every zoom, because a
 * radius larger than half the box is already clamped by the browser and scaling
 * it would be arithmetic with no effect on any pixel.
 *
 * These are a copy of `app.css`'s `--r-*` declarations, which is what the first
 * paint uses before any of this runs. The two must agree, and `metrics.test.ts`
 * reads the stylesheet rather than trusting them to — the same arrangement
 * `TYPE_BASE` has in `scale.svelte.ts` (FEAT-042).
 */
const RADII: Record<string, number> = {
	'r-field': 8,
	'r-button': 11,
	'r-row': 8,
	'r-panel': 14,
	'r-floating': 18
};

/**
 * Publish the metrics to CSS so stylesheets can size things without
 * hard-coding a number that would drift from the value above.
 *
 * `zoom` scales structure — widths, radii, lane geometry. `pitchScale` scales
 * the commit-row pitch alone, and is `zoom × textScale`, because the row is the
 * box the commit message sits in and has to grow when the message does. Both
 * default to 1, so a caller that does not care about scaling gets the design's
 * own numbers.
 */
export function applyMetrics(
	root: HTMLElement = document.documentElement,
	zoom = 1,
	pitchScale = zoom
): void {
	const px: Record<string, number> = {
		'lane-pitch': LANE_PITCH,
		'titlebar-h': TITLEBAR_H,
		'tabs-h': TABS_H,
		'toolbar-h': TOOLBAR_H,
		'strip-h': STRIP_H,
		'rail-w': RAIL_W,
		'detail-w': DETAIL_W,
		'refs-gutter-w': REFS_GUTTER_W,
		'lane-col-w': LANE_COL_W,
		'diff-files-w': DIFF_FILES_W,
		'diff-gutter-w': DIFF_GUTTER_W,
		'changes-files-w': CHANGES_FILES_W,
		'repo-card-w': REPO_CARD_W,
		'search-side-w': SEARCH_SIDE_W,
		'requests-detail-w': REQUESTS_DETAIL_W,
		'stash-entries-w': STASH_ENTRIES_W,
		'farm-log-h': FARM_LOG_H,
		...RADII
	};
	for (const [name, value] of Object.entries(px)) {
		root.style.setProperty(`--${name}`, `${Math.round(value * zoom)}px`);
	}

	// The pitch is its own line because it takes the other factor, and it is
	// clamped to at least one pixel: a zero-height row would divide by zero in
	// the virtualization arithmetic.
	root.style.setProperty(
		'--row-pitch',
		`${Math.max(1, Math.round(ROW_PITCH * pitchScale))}px`
	);
}
