<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-031 — Manual sweep

**Item:** [`agile/items/BUG-031-the-window-opens-in-the-wrong-theme.md`](../items/BUG-031-the-window-opens-in-the-wrong-theme.md)

**Run this on a packaged build, not `tauri dev`.** The defect is a startup
frame, and a development build serves its modules over HTTP from a Vite server —
a different startup entirely from the one a user has. On Linux it also matters
that the release build takes WebKitGTK's software renderer (FEAT-055), which is
what turns one wrong frame into several.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | A packaged build; theme set to Catppuccin **Mocha** and the app closed | Record the screen at 60fps or higher. Launch Spagitty. Step through the recording frame by frame | **No white frame.** The window's first painted frame is dark. Before this change the first frames were Latte. If any frame is light, note how many and on which renderer. | High | |
| SWEEP-002 | The same recording | Look for a *second* change — a moment where colours shift after the window is already up | There is none. The boot script and `theme.init()` set the same values, so the handover must be invisible. A visible second change means the cache and the palette table disagree. | High | |
| SWEEP-003 | A packaged build, warm and cold | Time from launch to the first painted frame, five runs each | Record the numbers. There is no target here to pass or fail against — the review asked for a measurement and this is it. What matters is that the boot script adds no measurable cost: it is one small same-origin file read synchronously. | Medium | |
| SWEEP-004 | A machine with no Spagitty storage (fresh user account), desktop set to **dark** | Launch Spagitty | It opens dark. Before this change a first run always painted light and then sampled the preference. Settings → Appearance shows **Follow system** active, with "now dark" beside it. | High | |
| SWEEP-005 | The same, still following | Change the desktop from dark to light **with Spagitty open** — on Omarchy, `omarchy-theme-set` or the system light/dark toggle | Spagitty follows, immediately, without touching the family. This is the behaviour that did not exist at all. | High | |
| SWEEP-006 | Following the system | Press **Dark** in Appearance. Change the desktop to light | Spagitty stays dark. Restart it: still dark, still `manual`. Press **Follow system** again and it resumes. | High | |
| SWEEP-007 | A development checkout | `bun run dev`, open the page, check the network panel | `/theme-boot.js` is served with `text/javascript` and no CSP error in the console. Edit the file and reload: the change is served. | Medium | |
| SWEEP-008 | A packaged build | Open the developer console and look for content security policy violations at startup | None. If the script were inline, this is where it would be blocked — silently as far as the user is concerned, since the only symptom is the flash returning. | High | |
| SWEEP-009 | A packaged build | In the console, set `localStorage['spagitty.theme.palette'] = '{"mode":"dark","tokens":{"--bg":"url(x)"}}'` and restart | The window opens with the stylesheet's own colours, not broken. Then set the key to `'garbage'` and restart: the same. A corrupt cache must cost a theme, never a window. | Medium | |
| SWEEP-010 | An install from **0.7.0** with an explicitly chosen dark theme | Upgrade and launch | Still dark, still that family. Appearance shows Dark active rather than Follow system — the migration treats a mode stored before the source existed as a decision. | High | |
| SWEEP-011 | Reduced motion enabled | Repeat SWEEP-001 | Unchanged. Nothing here animates, and the ticket exists to confirm that a startup change did not introduce a transition on the root element. | Low | |

## Negative paths this sweep deliberately covers

- **SWEEP-002** is the failure this fix can *introduce*: two code paths now set
  the same palette, and if they disagree the flash comes back in a new form.
- **SWEEP-009** is the corrupt-input case. The tests cover the parsing; this
  covers what a real window does with it.
- **SWEEP-010** is the one an upgrade can get wrong in a way nobody notices
  until they complain their theme changed by itself.

## What cannot be checked here

Whether the *first* frame is the right one on a machine faster or slower than
the reviewer's. The failure this fixes is timing-dependent by nature and was
never reproducible on demand — which is why the fix is structural (paint before
the layout mounts) rather than an attempt to make the layout mount sooner.
