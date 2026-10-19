<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-011 — Manual sweep

Test tickets for the Settings screen (1K).

**What this screen is.** Five sections behind a chip index: You, Accounts,
Behaviour, Appearance, Advanced. The identity is git's own configuration and is
read and written through it; the behaviour toggles are Spagitty's own and are
stored in its config directory; About carries the GPL-3 obligations. Nothing on
this screen needs an open repository.

**Before you start.** Some tickets write to your real `~/.gitconfig`. Copy it
somewhere first — SWEEP-1K-04 and SWEEP-1K-05 are exactly the case the automated
tests refuse to cover, which is why they are here.

| Field | Meaning |
| --- | --- |
| Priority | P1 blocks the item, P2 should be fixed before release, P3 cosmetic |
| Result | left blank for the tester: pass / fail |

---

### SWEEP-1K-01 — The rail reaches a real screen

- **Priority:** P1
- **Steps:** Press **Settings** in the nav rail.
- **Expected:** A screen with a header carrying five chips — You, Accounts,
  Behaviour, Appearance, Advanced — not the dashed "Not built yet" card. The
  theme chip that used to sit in the stub's header is gone from it and lives in
  Appearance.
- **Result:**

### SWEEP-1K-02 — It works with no repository open

- **Priority:** P1
- **Preconditions:** Spagitty started with no repository, or the open one closed.
- **Steps:** Open Settings and visit all five sections.
- **Expected:** Every section renders. **You** shows the global identity and
  says "No repository is open, so only the global configuration is offered";
  the *this repository* chip does nothing. Nothing anywhere says "no repository
  open" as an error. Acceptance criterion 8.
- **Result:**

### SWEEP-1K-03 — The identity matches what git reports

- **Priority:** P1
- **Preconditions:** A repository open. In a terminal, run
  `git config user.name` and `git config user.email` inside it.
- **Steps:** Open **You** and compare.
- **Expected:** The "In effect" values match the terminal exactly, and each says
  where it came from — your global configuration, this repository, the system
  configuration, or the environment. Acceptance criterion 1.
- **Result:**

### SWEEP-1K-04 — Writing the global identity

- **Priority:** P1
- **Preconditions:** Back up `~/.gitconfig`. Note the current
  `git config --global user.name`.
- **Steps:** With **Editing: global** selected, change the Name field and press
  **Save**. In a terminal run `git config --global user.name` and
  `git config --local user.name` inside the open repository.
- **Expected:** The global value is what you typed. The local value is
  unchanged — including *still unset* if it was unset. Nothing was written to
  `.git/config`; check its modification time or diff it. Acceptance criterion 2.
- **Result:**

### SWEEP-1K-05 — Writing the repository identity

- **Priority:** P1
- **Steps:** Select **Editing: this repository**, set an Email, press **Save**.
  Then run `git config --local user.email` and `git config --global user.email`.
- **Expected:** The local value is what you typed; the global one is unchanged.
  The screen now says the effective email comes from this repository, and shows
  a warning under the global field that the repository overrides it. Acceptance
  criterion 2.
- **Result:**

### SWEEP-1K-06 — Clearing unsets rather than blanks

- **Priority:** P1
- **Preconditions:** SWEEP-1K-05 done, so `user.email` is set locally.
- **Steps:** With the repository scope selected, press **Clear** on Email, then
  **Save**. Then run `git config --local --get user.email` (expect exit code 5
  and no output) and open `.git/config` in an editor.
- **Expected:** The key is **absent** from `.git/config` — not present with an
  empty value. The screen falls back to reporting the global email as the
  effective one. Acceptance criterion 3.
- **Result:**

### SWEEP-1K-07 — Switching scope does not carry a typed value across

- **Priority:** P1
- **Steps:** With **global** selected, type a new Name but do **not** save.
  Press **this repository**.
- **Expected:** The field refills with whatever the repository holds — empty if
  it holds nothing. What you typed is discarded rather than becoming a
  repository identity on the next Save.
- **Result:**

### SWEEP-1K-08 — Toggles survive a restart

- **Priority:** P1
- **Steps:** In **Behaviour**, switch all three toggles. Quit Spagitty
  completely and start it again. Return to Behaviour.
- **Expected:** All three are as you left them. Acceptance criterion 4.
- **Result:**

### SWEEP-1K-09 — A toggle says what it does not do yet

- **Priority:** P1
- **Steps:** Read each of the three toggles' text.
- **Expected:** Each says what it will do, and each carries "Persisted, not yet
  honoured" with a work item — FEAT-019, FEAT-015, FEAT-020. None of them
  claims to be in effect. Acceptance criterion 4, narrowed to the truth.
- **Result:** Obsolete, 2026-10-19 — FEAT-015, FEAT-019 and FEAT-020 made the three toggles do what they say.

### SWEEP-1K-10 — A hand-edited settings file does not stop the application

- **Priority:** P2
- **Steps:** Quit Spagitty. Find `settings.json` in Spagitty's config directory
  (`~/.config/spagitty/` or the platform equivalent), replace its contents with
  `{` and save. Start Spagitty and open Behaviour.
- **Expected:** Spagitty starts. The toggles show their defaults — sign off, ask
  before rewriting **on**, show commands off. No error dialog, no crash.
- **Result:**

### SWEEP-1K-11 — The About commit is this build's commit

- **Priority:** P1
- **Preconditions:** You built this binary yourself and know the commit —
  `git rev-parse HEAD` in the source tree at build time.
- **Steps:** Open **Advanced** and read the Build line.
- **Expected:** The full SHA matches. A binary built from a tree with no git
  history shows `unknown` rather than a wrong SHA. Acceptance criterion 5.
- **Result:**

### SWEEP-1K-12 — The license list is real

- **Priority:** P1
- **Steps:** In **Advanced**, read the dependency list. Pick three Rust
  packages and check each against `cargo metadata` or the crate's page. Check
  the npm side against `package-lock.json`.
- **Expected:** Names, versions and licenses match. The npm side lists only what
  ships — `@tauri-apps/api` and its dialog plugin — and not vite, vitest or
  typescript. Acceptance criterion 6.
- **Result:**

### SWEEP-1K-13 — Filtering the license list

- **Priority:** P2
- **Steps:** Type `apache` into the filter, then `not declared`, then something
  that matches nothing.
- **Expected:** Both lists filter together, by package name and by license. The
  counts beside each heading follow. Nothing matching leaves two empty lists and
  the page intact.
- **Result:**

### SWEEP-1K-14 — The GPL-3 obligations are all still there

- **Priority:** P1
- **Steps:** Read the whole **Advanced** section.
- **Expected:** Version, the build commit, `GPL-3.0-or-later`, the sentence
  about obtaining corresponding source, and the Software Freedom Conservancy
  trademark notice — everything the old footer carried. Acceptance criterion 7.
- **Result:**

### SWEEP-1K-15 — Pull requests reaches Accounts

- **Priority:** P2
- **Steps:** Open **Pull requests** from the rail and press
  **Settings → Accounts**.
- **Expected:** Settings opens **on the Accounts section**, not on You. Going
  back and pressing it again does the same. The address ends `#accounts`.
- **Result:**

### SWEEP-1K-16 — Theme

- **Priority:** P2
- **Steps:** In **Appearance**, switch to Dark, then Light, then quit and
  restart.
- **Expected:** The whole application follows immediately, and the choice
  survives the restart. The chip for the theme in use is the marked one.
- **Result:**

### SWEEP-1K-17 — A repository with no identity at all

- **Priority:** P2
- **Preconditions:** A repository where `user.name` is unset everywhere —
  easiest inside a container or with `HOME` pointed at an empty directory.
- **Steps:** Open Settings → You.
- **Expected:** "Not set anywhere. Git refuses to commit without it." rather
  than an empty field with no explanation.
- **Result:**

### SWEEP-1K-18 — A write that cannot succeed

- **Priority:** P3
- **Preconditions:** Make `~/.gitconfig` read-only (`chmod 400`).
- **Steps:** Edit the global Name and press Save. Afterwards, put the
  permissions back.
- **Expected:** The footer shows git's own error. The field keeps what you
  typed rather than clearing it, and the screen does not claim the value was
  saved.
- **Result:**
