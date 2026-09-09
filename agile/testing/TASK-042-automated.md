<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-042 — Automated tests

**Item:** [`agile/items/TASK-042-one-control-scale-and-a-platform-policy.md`](../items/TASK-042-one-control-scale-and-a-platform-policy.md)

## What was tested

Two new contracts in `src/lib/ui/flat.test.ts`, four assertions about the macOS
window policy in `src/lib/chrome/window.test.ts`, and the existing window-state
assertions rewritten for the new platform split. 2,797 → 2,816.

| Test | Asserts |
| --- | --- |
| **nothing names its own type size** | No component stylesheet contains `font-size: Npx`. `em` and `calc(var(--fs-…) * n)` pass, because both follow the scale. |
| **controls stay where they are** (7 cases) | No `:hover` rule in `Btn`, `Chip`, `RefChip`, `Menu`, `NavRail`, `Toolbar` or `TitleBar` sets a `transform`. |
| still answers a press | The other half, so this is a contract rather than a deletion: the feedback moved to the event that earns it, it did not vanish. |
| leaves the base configuration undecorated | Linux and Windows still get the undecorated, transparent window. |
| gives macOS the real frame back | `decorations: true`, `transparent: false`. |
| keeps Spagitty's own bar by overlaying the system controls | `titleBarStyle: "Overlay"` and `hiddenTitle`. Plain decorations would give two title bars. |
| changes nothing but the decoration | Title, size and resizability are identical to the base config. A window that opened at a different size on one platform would be a second set of numbers to keep in step. |
| draws its own card on the host that decorates nothing | Windows alone now. |
| draws no card at all on Linux / macOS | Both, for the same reason. |
| reads the platform off the agent string, and only whole words | Including that an unrecognised host still gets the card. |

## Why the two new contracts are the substance

Eight type sizes and nineteen hover rules is an afternoon's work. Both were
*already* an afternoon's work once — TASK-039 fixed 83 sizes in eleven dialogs
and eight more had appeared elsewhere by the time this task started. A pass with
no assertion behind it has to be run again, which is exactly what `flat.test.ts`
learned about its own `KNOWN` list.

The hover contract is deliberately **scoped**, and the scope is argued in the
test rather than left implicit: the shared kit and the chrome, because that is
where targets repeat in a row and a moving target costs something. A repository
card in a grid still lifts.

## Run against the broken state first

`Btn.svelte`'s hover lift restored:

```
 × src/lib/ui/Btn.svelte moves nothing on hover
   Tests  1 failed | 19 passed (20)
```

The type-size assertion was written before the last eight were fixed and failed
on exactly those eight, which is how the list in the item was produced.

## What is not covered, and why

- **Everything about macOS that needs a Mac.** Whether the platform config
  merges, whether the traffic lights land where 78px expects, whether the drag
  region still moves the window, what full screen does to the reservation, and
  whether shortcuts and focus behave. `SWEEP-005` to `SWEEP-009`, all unrun.
  The tests here can only say the file is right and the frontend agrees with it.
- **Whether stillness reads better than the lift.** It is a taste judgement and
  the review made it; `SWEEP-004` is where somebody looks.
- **Reflow from the eight grown sizes.** `SWEEP-002`, `SWEEP-003`.

## Coverage

All changes are stylesheets, markup guarded by an existing tested predicate
(`isMac`, covered by `platform.test.ts` from BUG-030), and one JSON file. No
statement or branch was added to a module in the coverage scope.
