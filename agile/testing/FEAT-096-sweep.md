<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-096 — Manual sweep

**Item:** [`agile/items/FEAT-096-an-extension-host-anyone-can-build-for.md`](../items/FEAT-096-an-extension-host-anyone-can-build-for.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT096-01 | A release build; the fixture repository open | 1. `bun run ext create com.example.sweep sweep`<br>2. `bun run ext test sweep`<br>3. `bun run ext dev sweep`<br>4. Open Settings › Extensions | The new extension is listed as Development; turning it on asks for nothing beyond its required permissions | P1 | |
| SWEEP-FEAT096-02 | 01 done | 1. Open the command palette and type "hello"<br>2. Run it | The command runs; a notice says hello from the current branch | P1 | |
| SWEEP-FEAT096-03 | — | 1. `bun run ext pack examples/extensions/hello --build`<br>2. Settings › Extensions › Install from file… and choose the package | The dialog shows identity, publisher "as stated", the target, the checksum, the files and the trust sentence; Install puts it in the list as "Installed from a file" | P1 | |
| SWEEP-FEAT096-04 | 03 done, enabled | 1. Open Commit<br>2. Use the "Say hello" button and the summary panel | Both work; nothing else on the screen moved | P1 | |
| SWEEP-FEAT096-05 | 03 done | 1. Bump the example's version, add a capability, pack again<br>2. Update from file… | The dialog marks the new capability "new in this version"; after updating it is not granted until turned on | P1 | |
| SWEEP-FEAT096-06 | 05 done | 1. Roll back | The previous version is back; its grants are as they were | P2 | |
| SWEEP-FEAT096-07 | 03 done | 1. Turn it off for the repository<br>2. Open the palette | Its commands are gone from the palette and the Commit screen | P1 | |
| SWEEP-FEAT096-08 | 03 done | 1. Remove it, deleting history | It is gone from the list and from `%APPDATA%\dev.spagitty.app\extensions\packages` | P1 | |
| SWEEP-FEAT096-09 | A package whose name contains `../` or two names differing in case (build with the test writer) | 1. Install from file… | Refused with the reason; nothing written under `packages` | P1 | |
| SWEEP-FEAT096-10 | An extension whose worker exits on a command | 1. Run the command three times | Each run fails with "stopped unexpectedly"; the card says Stopped; after the third, running it says to restart; Restart makes it work again; the window never stops responding | P1 | |
| SWEEP-FEAT096-11 | — | 1. Light theme and dark theme, two text sizes<br>2. Tab through Settings › Extensions and a findings panel | Every control is reachable and visibly focused; long finding text wraps inside its card; the dialogs close with Escape | P2 | |
| SWEEP-FEAT096-12 | macOS and Linux machines | Repeat 01–04 | The same results; the program is found and runs | P2 | |
