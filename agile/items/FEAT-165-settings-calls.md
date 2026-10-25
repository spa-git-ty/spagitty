<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-165 — Settings › Calls

**Status:** Backlog — after FEAT-161.
**Screens:** 1K (Settings › Calls, a new section).
**Raised by:** the author, 2026-10-25: make it easy to configure; put whatever settings Calls can have in Settings.
**Proposal:** [`docs/proposals/calls.md`](../../docs/proposals/calls.md) §9. **Designs:** [`design_handoff_calls/`](../../design_handoff_calls/) — `SettingsCalls.dc.html`.

## Change

A *Calls* section in Settings holds every setting Calls reads, in six groups. A repository can override any of them in `.spagitty/calls/settings.json`.

## Acceptance criteria

- **A new chip, Calls**, after Behaviour, at `/settings#calls`. The section id is added to `SECTIONS` with its own branch in `routes/settings/+page.svelte`.
- **The groups and defaults** are the table in the proposal's §9:
  - Where calls live
  - Scanning code
  - Kept values
  - Sending
  - SOAP
  - Safety and sharing
- **Overrides**:
  - `.spagitty/calls/settings.json` overrides any setting for that repository.
  - The section's line says so, and a setting that is overridden in the open repository shows *Overridden in this repository*.
- ***Leave secrets out when sharing*** is shown as *Always*, locked. It is not a setting.
- **Frameworks not yet readable** (Micronaut, Quarkus, Express, FastAPI) are not shown until a reader exists.
- ***Reset*** returns this section to its defaults.
- **Tests**: a component test for the section, and a test that every setting is read by the code it names.
