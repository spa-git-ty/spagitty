<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-115 — Agents in Review and Merger

**Status:** Done — ships in 2.0.0. The native sweep with installed agents and a real key is still to run.
**Branch:** `feature/FEAT-115-agents-in-review-and-merge`.
**Screens:** 1K (Settings › Agents, a new section), 1R (Review), 1S (Merger), chrome (the rail's dots, the palette), 1Q (Farm › Setup reads the same agents).
**Raised by:** the author, 2026-10-08: attach agents in Settings — a local agent CLI, or a remote agent through an API key — then assign them in Review and in Merger to do the review or the merge; show their work as it happens; let the person choose how much is automated; and do not redesign the screens people use to review and merge by hand.
**Proposal:** [`docs/proposals/agents-in-review-and-merge.md`](../../docs/proposals/agents-in-review-and-merge.md). **Designs:** [`design_handoff_agents/`](../../design_handoff_agents/).

## Change

A person can hand a review or a merge to an agent and stay in charge of it.

- **Settings › Agents** attaches agents once, for the machine: the command-line agents detection finds (Claude Code, Codex, Cursor, Oh My Pi) or any CLI added by hand, and models reached over Anthropic's, OpenAI's, Google's or any OpenAI-compatible API with the person's own key. Each says what it may do — Review, Merge, Farm — and the open repository has its own rules: the highest level, the agents allowed, consent to send code to a provider, the branches never landed unattended, whether an agent may give a verdict, and whether its comments say so on the host.
- **Review and Merger** gain one way in each — *Assign an agent…* beside *Start review*, *Check out branch* and *Resolve N conflicts* — and an Agent card while an assignment exists. Everything an agent makes is the material a person would have made: a finding is a pending comment with an author, a resolution is one of the resolver's own choices with who chose it.
- **Four levels** — Suggest, Step by step, Sign off, Unattended — say where the agent stops. Unsure, failing checks, a reached limit and a moved head always stop. What no level lifts is enforced by Spagitty around the agent.

## Acceptance criteria

- **No agent, no trace.** With no agent set up, Review and Merger render as in 1.3: no button, no tab, no mark, no legend entry. Tested by mounting both screens with an empty agent list.
- Settings › Agents lists the built-in providers with what detection found — a version and a path, *Doesn't run* with its reason, or *Not installed* — and the two *Add* rows. *Test* runs a local agent once on a tiny prompt in a temporary directory, and sends one small request to a remote one, and says what answered and how long it took, or the provider's own sentence.
- A remote agent's key goes to the OS keychain beside the forge tokens and nowhere else. The section shows that a key is stored and its last four characters. The webview never holds a key.
- Model requests go through one module, `crates/spagitty-core/src/models/http.rs`, beside `forge/http.rs`; the boundary test names the two. `https` only, except to an endpoint on this machine; no redirects; the key is cut out of every error.
- An assignment refuses up front: an agent the repository does not allow, a remote agent in a repository that has not agreed to send code to its provider (asked once, kept in Settings), an agent that wrote commits in the pull request (by `Co-authored-by` trailer), and a second agent on the same job.
- An agent runs in a scratch worktree under the git directory — at the pull request's head, or holding the merge with its markers — never in the person's working copy. A local agent runs in its provider's read-only mode where it has one. After every step the worktree is compared with how it was, and anything the agent wrote is listed in the timeline and put back.
- The levels table is the artboard *Levels and gates*: Suggest proposes everything and applies nothing; Step by step stops at every gate; Sign off applies its own sure proposals and stops before Send or Land; Unattended does the last act, only where the repository allows it, as Comment unless verdicts are allowed, and for a merge only with checks configured and passing, every conflict sure, neither branch moved, and the receiving branch not on the never-unattended list.
- Raising the level takes effect from the next gate and never applies what already waits; lowering takes effect at once. The level is capped at the repository's highest. Every change is recorded with who made it.
- Review: findings sit where pending comments go, in the agent's colour, with its name, severity and *unsure*, and *Accept*, *Edit*, *Dismiss*, *Why?*. Only decided ones go out with Finish review. Comments an agent drafted carry *Drafted with …* on the host and the review body counts them, unless the repository turns that off. A finding is only kept on a line of the diff.
- Merge: Spagitty names each resolution — A, B, both either way, picked lines, or Edit — by comparing the agent's text with the sides. Text the agent wrote is the fourth origin, with a hexagon in the agent's colour. The files list's dots gain a ring while the agent works on a conflict and a half-filled dot once it has proposed. The commit dialog says who chose each conflict, and the message gains a `Co-authored-by` trailer naming the agent.
- The Agent card: the agent, the level as a menu, Pause and Stop; one sentence that is always true; the meter, and tokens against the budget for a remote agent; the timeline with a gate as the one tinted card; *Tell the agent…*; *Raw output*. After three minutes with no output the sentence says so. Reduced motion stops the working mark.
- The rail's dot shows on Review or Merger while an assignment there waits for the person. Notifications for *waiting*, *finished* and *stopped*, only while the window is not focused, each switchable.
- Every control is also a palette command, greyed with its reason when no agent is working.
- An assignment's record — the agent and its version or model, every level change, each step, each proposal and who decided it, and the raw transcript — is kept in application data and never sent anywhere. After a restart an unfinished one reads *Stopped when Spagitty closed*, with *Resume*: a new run from the last finished step, told what was already done.

## Decisions on the proposal's open questions

Taken with the author on 2026-10-08 (§18):

1. Agents sits after You in the chip index, as the design draws it.
2. No committed rule against remote agents in 2.0: the choice is per machine and per repository, in Settings.
3. *Drafted with <agent>*, on by default, switchable per repository.
4. An unattended agent may give a verdict only where the repository allows it; otherwise it sends Comment.
5. No reading ahead while a gate waits.
6. Two vocabularies: the farm's autonomy and the assignment levels answer different questions and stay apart.
7. Hosted agent services are not a third kind in 2.0.
8. A change outside the conflicts is always refused and reported, never kept.
9. Step by step is the first-time default for both jobs.
10. Tokens only; no prices.

## Not in this item

- **The agent's band** beside the focus ruler (spec §8.4). A command-line agent does not report which lines it is reading, so the room marks the file the agent is on and the files it has read, and moves with it when *Follow the agent* is on.
- **A finding that repeats an open thread becoming a proposed reply in it** (§8.3). The agent is told the open threads on each file and asked not to repeat them; a repeat arrives as a finding.
- **Restart on the new head keeping the unchanged findings** (§8.7). A moved head pauses the assignment with *Carry on*, which marks every later finding as made against the old head, or *Stop*; a new assignment starts on the new head.
- **Undo the merge** after an unattended landing (§9.6). The landing is refused unless the checks passed on the result, every conflict was sure, neither branch moved and the receiving branch is not on the never-unattended list; moving it back is the person's, by hand.
- **Rebase stops taken by an agent** (§9.7). *Assign an agent…* is not offered while the strategy is *Rebase then fast-forward*, nor at a rebase's stop.
- Per-assignment notifications sounds and per-agent cost rates.
