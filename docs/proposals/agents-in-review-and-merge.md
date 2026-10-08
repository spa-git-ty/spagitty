<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Proposal: agents that review and merge — Spagitty 2.0.0

**Prepared:** 2026-10-08  
**Target:** 2.0.0  
**Status:** Proposed. A description of the feature only: no implementation, no work items, no identifiers assigned yet.  
**Raised by:** the author, 2026-10-08: attach agents in Settings — a local agent CLI, or a remote agent through an API key — then assign them in Review and in Merger to do the review or the merge; show their work as it happens, in a decent way; let the person choose how much is automated, from fully to an approval on every step; and do not redesign the screens people use to review and merge by hand.  
**Screens:** 1K (Settings › Agents, a new section), 1R (Review), 1S (Merger), chrome (the rail's dots, notifications). 1Q (Farm) reads the same agents.  
**Designs:** the Claude Design canvas *Spagitty 2.0 — Agents in Review and Merger*, listed in [section 13](#13-designs). To be exported to `design_handoff_agents/` the way `design_handoff_review/` and `design_handoff_merger/` were.

## 1. Outcome

Spagitty 2.0 lets a person hand a review or a merge to an agent and stay in charge of it.

- **Settings › Agents** is where agents are attached, once, for the whole machine. A **local agent** is a command-line agent installed here — Claude Code, Codex, Cursor, Oh My Pi, or any CLI added by hand. A **remote agent** is a model reached over its provider's API with the person's own key.
- **Review (1R)** and **Merger (1S)** can **assign** one of them to a pull request or to a merge. The agent reads, proposes and — as far as the person allows — acts.
- **What the agent is doing is visible while it does it**: one sentence that is always true, its presence on the files and lines it is working on, a timeline of steps, and the raw output one click away.
- **How far it goes alone is a level** the person picks per assignment: from *Suggest*, where nothing it produces is applied until accepted, to *Unattended*, where it finishes the job inside limits no level lifts.
- **The human path is unchanged.** With no agent set up, Review and Merger are the 1.3 screens. With agents set up, each screen gains one way in, and everything an agent produces is made of what a person would have made by hand — pending comments, resolver choices — so the person can take over at any moment and finish with the tools they already know.

## 2. The rule that shapes the rest: agents are a layer, not a mode

The author asked that the current screens not be redesigned for the non-agentic review and merge. That request is a design rule here, and six more follow from it.

1. **No agent, no trace.** With no agent set up, nothing in Review or Merger mentions agents: no empty panel, no disabled button, no "set up an agent" banner. Settings › Agents is the one place that says what agents are. A test renders both screens with an empty agent list and asserts that no agent element exists.
2. **One way in per screen.** With agents set up, the always-visible addition is *Assign an agent…* beside the action that starts the same work by hand — beside *Start review*, and beside *Resolve N conflicts*. Everything else appears only while an assignment exists.
3. **The live view goes where a card already goes.** Review already has an inset card on the right, the Conversation; the agent becomes a second tab of it. Merger's resolving view gains an inset card of the same kind, only while an agent is assigned, and it can be put away like the Graph's detail panel (FEAT-056).
4. **Agent output is the same material as human output.** A review finding is a pending comment (FEAT-093) with an author. A merge proposal is one of the resolver's own choices — Take A, Take B, Both, Pick lines, Edit (FEAT-102) — with an author. Nothing is kept in a form only the agent's view can show.
5. **Taking over is free.** *Stop* or *Take over*, at any point, leaves every proposal where the person would look for it, and the screen is then exactly the hand-review or hand-merge screen with some drafts already in it.
6. **The reader is never moved.** An agent working on file 9 does not scroll the person away from file 3. Following the agent is a toggle, off by default.

## 3. Words used here

| Word | Means |
| --- | --- |
| **Agent** | Something Spagitty can hand a job to. Copy names it — "Claude Code", "Codex", or the model — and never says "AI": nothing in Spagitty is a model (`docs/branding.md`). |
| **Local agent** | A command-line agent installed on this machine, run by Spagitty as the person, in a worktree of its own. What the farm runs today. |
| **Remote agent** | A model reached over its provider's HTTP API with a key the person supplies. Spagitty runs the loop and gives it a small, typed set of tools; it never gets a shell. |
| **Assignment** | One agent, one job: *review pull request #214*, or *merge feature/auth into main*. It has a level, a timeline and a record. |
| **Step** | A unit of the agent's work the person can see finish: read the pull request, plan, one file, one conflict, the checks. |
| **Gate** | A point where the agent may have to stop and wait for the person. Which gates stop depends on the level. |
| **Proposal** | Something the agent wants to happen: a comment, a verdict, a resolution. Accepted, edited or dismissed by the person — or applied by the agent itself at the higher levels. |
| **Level** | How far the agent goes alone. Four of them ([section 6](#6-levels-how-far-the-agent-goes-alone)). |

## 4. Settings › Agents

### 4.1 Where it sits

A new chip in Settings' index, **Agents**, at `/settings#agents`, built like the other sections — labelled rows and chip rows, no explanation it does not need (TASK-044). Farm › Setup links to it rather than carrying its own copy of the agent configuration.

The section has four parts, top to bottom: **On this machine** (local agents), **Through an API** (remote agents), **Defaults**, and **In this repository** (the open repository's rules). The design is the artboard *Settings › Agents*.

### 4.2 Local agents

- **Found, not typed.** The list is the farm's detection (`crates/spagitty-farm/src/agent/detector.rs`), re-read when the section opens. Each built-in provider is shown whether or not it is installed, with what was found: a version and a path, *found but does not run* with its reason, or *not installed*. None of this is new machinery; it moves from Farm › Setup to Settings.
- **Any CLI by hand**: a name, the command and the argument template, as the farm's Custom adapter takes them, and whether the prompt goes as an argument or on standard input. *Test* runs it once on a tiny prompt in a temporary directory and shows what came back.
- **What it may do**, per agent: *Review*, *Merge*, *Farm*. Off means it is never offered for that job. These are switches about trust, not capability chips; the capability chips stay with the farm's routing.
- **Permissions per job.** For a review the agent runs in its provider's read-only mode, where the provider has one. For a merge it may write, but only in the worktree it is given ([9.4](#94-proposals-in-the-resolvers-own-vocabulary)). A custom agent can promise neither, so Spagitty checks after every step rather than trusting it ([8.4](#84-in-the-room), [9.4](#94-proposals-in-the-resolvers-own-vocabulary)).

### 4.3 Remote agents

- **Provider, key, model.** The first providers proposed: Anthropic, OpenAI, Google, and any endpoint that speaks the OpenAI-compatible chat API — which covers routers such as OpenRouter and local servers such as Ollama and LM Studio — with a base URL field for that last kind. The model is picked from the provider's own list when it offers one, or typed. The design is the artboard *Add an API agent*.
- **The key goes into the OS keychain**, beside the forge tokens (`crates/spagitty-core/src/forge/keychain.rs`), and nowhere else: not the settings file, not the webview, not a log line. The section shows that a key is stored and its last four characters, never the key.
- **Test** sends one small request and says what answered: the provider, the model and how long it took, or the provider's own error sentence — the treatment forge errors already get.
- **Limits**: a token budget per assignment and per day, and a time limit per assignment. Reaching one stops the agent at the end of its current step and keeps everything it made. It never stops silently.
- **What leaves the machine** is said here once, and again at the first assignment in each repository ([11.3](#113-what-leaves-the-machine)). An endpoint on `localhost` is said to keep everything on this machine.

### 4.4 Defaults and the repository's rules

- **Defaults**: which agent reviews and which merges, and at which level. The *Assign* popover starts from these.
- **In this repository**, with the open repository named:
  - which agents may be used here — all, or a list;
  - whether remote agents may receive this repository's code — asked once ([11.3](#113-what-leaves-the-machine)) and changeable here;
  - the **highest level allowed** here — a work repository can be capped at *Sign off*, so nobody, including the person on a tired evening, lands something unattended;
  - branches an agent may **never land into unattended**;
  - whether an agent may give a **verdict** on the host — Approve or Request changes — rather than only comment. Off by default;
  - whether comments an agent drafted say so on the host ([8.6](#86-what-the-author-of-the-pull-request-sees)). On by default.

### 4.5 Machine, repository, farm

The farm keeps its agent registry per repository, deliberately (`crates/spagitty-farm/src/persistence/store.rs`): which agents exist is a fact about the machine; which of them a project uses, with which capabilities and which flags, is a fact about the project. That reasoning holds, so 2.0 adds a layer rather than replacing one:

1. **Machine** — Settings › Agents: every agent this person has, local or remote, with its secrets in the keychain.
2. **Repository** — `.spagitty/farm/agents.json` keeps overriding: capabilities, extra arguments, which agents are switched on here. Review and Merger read the same overrides the farm does.
3. **Farm** — the crew is chosen from the two above, as Setup does today.

On the first start of 2.0, agents found in the registries of repositories Spagitty knows are offered for the machine list, once. Nothing is moved without asking, and the repository files stay as they are.

### 4.6 With nothing set up

The section lists the built-in providers with what detection found, and the two *Add* rows. It does not tell the person they need an agent. Review and Merger say nothing ([section 2](#2-the-rule-that-shapes-the-rest-agents-are-a-layer-not-a-mode)).

## 5. Assignments

### 5.1 One agent, one job

An assignment is made from Review or Merger and belongs either to one pull request at one head, or to one merge: two branches, their merge base, a direction and a strategy. In 2.0 there is at most one live assignment per pull request and per merge. Two agents on one job is left for later ([section 17](#17-non-scope)).

### 5.2 Its life

`Starting → Working ⇄ Waiting for you → Done`, with `Paused`, `Stopped` and `Failed` beside them. Those are the words on screen. *Waiting for you* is the only state that puts a dot on the rail and may send a notification.

### 5.3 Where it runs

Never in the person's working copy, and never on the checked-out branch's files. A review runs in a scratch worktree at the pull request's head, under the git directory. A merge runs in the dry run's scratch worktree (FEAT-100) or a worktree of its own. Both are removed when the assignment ends, and found again by name if Spagitty restarts mid-assignment.

### 5.4 It survives a restart; its process does not

The assignment, its timeline and its proposals are written as they happen — temporary file and rename, events one JSON object per line: the farm's store, reused. A local agent's process does not outlive Spagitty. On the next start the assignment reads *Stopped when Spagitty closed*, with *Resume*, which starts a new run from the last finished step and tells the agent what was already done.

### 5.5 The record

Kept in application data, per repository, beside the review state (FEAT-087) and the merger state (FEAT-102), so a pull request's review and a merge's choices can point at the assignment that worked on them. It holds the agent and its version, or the provider and model; the level and every change to it; the head or the tips it worked against; each step; each proposal, and who decided it; and the raw transcript. It is the answer to *who decided this line*, and it is never sent anywhere.

## 6. Levels: how far the agent goes alone

FEAT-073 made the farm's autonomy a sentence about where the human is, not a magnitude. Assignments keep that: four levels, each named for what the person does. The design is the artboard *Levels and gates*.

### 6.1 The gates

| Job | Gate | What waits there |
| --- | --- | --- |
| Review | **Plan** | The agent's reading plan: which files matter, in what order, and what it will look for. |
| Review | **Each file** | That file's findings, as proposed comments. |
| Review | **Verdict** | The summary of the whole pull request, and a suggested Comment, Approve or Request changes. |
| Review | **Send** | *Finish review*: everything reaches the host, and the author sees it. |
| Merge | **Each conflict** | The proposed resolution of one conflict region. |
| Merge | **Checks** | The repository's checks, run on the resolved result. |
| Merge | **Land** | The commit, and the receiving branch moving (FEAT-101). |

A rebase in Merger (FEAT-103) is a sequence of stops. Each stop's conflicts are *Each conflict* gates, and *Land* is *Finish the rebase*.

### 6.2 The four levels

| Level | You… | The agent stops at | The last act is done by |
| --- | --- | --- | --- |
| **Suggest** | do the work; the agent prepares it | nothing — it runs to the end, and nothing it proposes is applied until you accept it, one by one or all at once | you |
| **Step by step** | approve every step | every gate | you |
| **Sign off** | approve the end | *Send* or *Land*, plus anything it is unsure of and any failing check | you |
| **Unattended** | read the record afterwards | only what no level lifts ([6.3](#63-what-no-level-lifts)) | the agent, inside limits |

On the two screens:

- **Suggest** is the quiet one. The agent's findings arrive as proposals beside the person's own reading, to be decided whenever they are reached. In Merger the checks still run — on the agent's proposed result, in its own worktree — so the person knows whether its version builds before accepting any of it.
- **Step by step** is reviewing together: the plan, then one file at a time; or one conflict at a time. The agent waits at each gate, and *Accept*, *Edit*, *Redo with a note* or *Take over* lets it go on.
- **Sign off** lets the agent apply its own proposals as it goes — its comments become pending drafts, its resolutions become resolver choices — and it stops at the irreversible act. The person gets the familiar *Finish review* card or commit dialog, filled in, with every item marked as the agent's.
- **Unattended** also does the last act: it sends the review or lands the merge. It is offered only where the repository allows it ([4.4](#44-defaults-and-the-repositorys-rules)), and for a merge only where checks are configured.

**Unsure always stops.** Every proposal carries the agent's own *sure* or *unsure*. An unsure proposal stops at *Sign off* and at *Unattended* too, and is never applied without the person. An agent that marks everything sure earns nothing by it: the limits below do not rely on the agent's word.

### 6.3 What no level lifts

Enforced by Spagitty around the agent, not asked of it in a prompt.

1. **Never the person's working copy**, and never a file outside the worktree it was given.
2. **Never a push**, never a force of anything, never a branch deleted, and no ref moved but the merge's receiving branch.
3. **No verdict on the host** — Approve or Request changes — unless the repository allows it. An unattended review without that permission is sent as *Comment*.
4. **Never someone else's thread**: no resolving, reopening or editing it. A reply in another person's thread is always a proposal.
5. **An agent never reviews its own work.** A pull request with commits the agent wrote — named by a `Co-authored-by` trailer, or because they came from a farm task it ran — refuses that agent, with the reason. This is the farm's rule (`crates/spagitty-farm/src/review/reviewer.rs`), carried over to Review.
6. **A merge changes only conflict regions.** The result an agent lands differs from the dry run only inside the conflicts it resolved. A change anywhere else — even one that makes a test pass — is refused and reported. It is a new change, not a resolution, and it belongs in a commit of its own that a person asked for.
7. **Unattended landing needs passing checks**, current to the result being landed, and both branches still where the plan read them (FEAT-101 already refuses a branch that moved).
8. **A limit reached stops the agent** at the end of its step, and keeps what it made.

### 6.4 Changing the level mid-way

Lowering takes effect at once: an agent at *Sign off* lowered to *Step by step* stops at the next gate. Raising takes effect from the next gate, and never applies proposals that were already waiting — those stay for the person. The timeline records every change and who made it.

### 6.5 Where the level is chosen

In the *Assign* popover, starting from the default for that job, and offering nothing above the repository's highest level. It stays on the Agent card as a menu for the life of the assignment.

## 7. Watching an agent work

### 7.1 The work is the display; the log is the footnote

A transcript scrolling past is not a view of work. That is the farm's own lesson, from BUG-021 (*a run says nothing until it ends*) and FEAT-077 (*the farm is worth watching*). What the person needs, in order: is it working, on what, does it need me, what has it done. So four layers, each quieter than the one before:

1. **One sentence, always true**, in the present tense and with a count: *Reading src/auth/session.rs · 4 of 12 files*; *Proposing a resolution · conflict 3 of 7*; *Waiting for you: 2 findings on token.rs*; *Running checks · bun test*. It is on the Agent card, on the inbox card or the Result card, and in the notification.
2. **Presence on the work**: the files list and the lines themselves show where the agent is and what it has touched ([8.4](#84-in-the-room), [9.4](#94-proposals-in-the-resolvers-own-vocabulary)).
3. **The timeline**: one row per step, finished steps folded to a line, the current step open, a gate drawn as a card with its actions.
4. **Raw output**, folded under the timeline: the narrated transcript as the farm's Task screen shows it, and the command line that started the run, quoted so it can be pasted into a shell.

### 7.2 The Agent card

- **The header**: the agent, the level as a menu, *Pause* and *Stop*; then the sentence; then the meters — steps done of steps planned, elapsed time, and for a remote agent the tokens used against its budget.
- **A step row** says in words what was done and how long it took. Opening it lists what the agent read, searched and ran, as narrated events rather than JSON.
- **A gate** is the only tinted card in the timeline. Its first action takes the person to the thing waiting — the comment in the diff, the conflict card — because that is where it is properly decided. *Accept* on the gate card is the shortcut.
- **A quiet agent is marked.** After three minutes with no event the sentence says *No output for 3 minutes* rather than going on claiming it is reading — the farm's quiet-run rule.
- **Failure** is the agent's or the provider's own sentence, and what was kept: *Codex exited with 1 after 4 of 12 files. Its 3 findings are kept.* — with *Resume* and *Raw output*.
- **At the foot**: *Tell the agent…*, a one-line box ([section 10](#10-talking-to-an-agent-while-it-works)), and *Raw output*.

### 7.3 Outside the screen

- **The rail** draws its dot on Review or Merger while an assignment there is *Waiting for you* — the same dot those rows already draw for a review asked of you and for unresolved conflicts.
- **The inbox card and the Result card** carry the agent's sentence in a small line under their own, so an assignment can be followed from the list without opening it.
- **Notifications** (FEAT-114) for *waiting for you*, *finished* and *stopped*, only while the window is not focused, each switchable.

### 7.4 What it must not cost

Events, never polling: the agent's output is narrated where it is read (`crates/spagitty-farm/src/execution/narrate.rs`) and reaches the webview as events, as the farm's does. The timeline draws only the rows in view (`src/lib/ui/VirtualRows.svelte`). No blurred surface beyond the ones these screens already have. Reduced motion stops the working indicator's animation and leaves a still mark. The sentence changes at most a few times a second, however fast the agent prints.

## 8. Review with an agent (1R)

### 8.1 What does not change

The inbox, the room, the review pill, the reading aids, the threads and *Finish review* are as they are in 1.3. A person who never assigns an agent never sees anything in this section.

### 8.2 Assigning

- **Where.** The preview card's *Start review* becomes a split button whose menu holds *Assign an agent…*. A right-click on an inbox card offers the same. In the room, the header has it beside *Check out branch*. The design is the artboard *Review · Assign an agent*.
- **The popover**: the agent (starting from the default), the level as four rows with a sentence each, an optional note (*look at the migration first*), and two plain lines: where it will run (*a scratch copy of the pull request's head — your branch is not touched*) and, for a remote agent, what leaves the machine.
- **Refusals said up front.** An agent that wrote commits in this pull request is listed but disabled, with the reason ([6.3](#63-what-no-level-lifts), rule 5). A remote agent in a repository that has not agreed ([11.3](#113-what-leaves-the-machine)) asks first.

### 8.3 What the agent does

1. **Reads the pull request**: its description, its threads open and resolved, its checks, and which parts are conflict fixes (FEAT-092) — so those are reviewed as resolutions, not as the author's design.
2. **Plans**: the files that matter, in the order it will read them, and what it will look for. The plan is a gate at *Step by step*; the person can reorder it or drop files.
3. **Reads each file** — the change, and as much of the whole file and its callers as it needs — and proposes findings: a line or a range, a severity (*high*, *medium* or *low*, as the farm's reviews are already graded), the comment, and *sure* or *unsure*. A finding that repeats an open thread becomes a proposed reply in that thread instead.
4. **Sums up**: what the pull request does, what it found, and a suggested verdict.

An agent's work here is proposals only. It does not tick files viewed, does not resolve anything, and does not write to the head.

### 8.4 In the room

The design is the artboard *Review room · agent working*.

- **The files list** gains a small mark on the file the agent is reading, and a quiet *agent read* mark on the files it has finished, kept apart from the *Viewed* tick. An agent reading a file never ticks it: *viewed* means a person read it.
- **The diff** draws the agent's proposals where pending comments go, under their lines, in the agent's colour, with its name — *Codex · proposed · high* — and four actions: *Accept*, *Edit*, *Dismiss*, *Ask why*. Accepting makes it one of the person's pending comments, which goes out with *Finish review* like any other. Editing does the same with the person's words. Dismissing keeps it in the record only.
- **The agent's place.** On the file the agent is reading, a second, cooler band beside the focus ruler shows the lines it is on. *Follow the agent*, a toggle on the Agent card, moves the room with it; it is off by default.
- **Filters.** The files list's chips gain *Agent findings*, only while there are some.
- **Checked after each step.** The review worktree is compared with the head after every step. Anything an agent wrote there is reported in the timeline and thrown away.

### 8.5 Finishing

- At **Suggest** and **Step by step**, *Finish review* is the person's, as today. The card says how many of its comments came from the agent.
- At **Sign off**, the card opens filled in: the agent's summary as the words for the whole pull request, and its suggested verdict selected. The person reads it, changes what they want, and sends.
- At **Unattended**, Spagitty sends it — as *Comment* unless verdicts are allowed here — and the notification says what was sent, with the way to the pull request.

### 8.6 What the author of the pull request sees

Comments go out under the person's account, because it is their review. Each comment the agent drafted carries one line at its foot — *Drafted with Claude Code* — and the review's body says how many were. On by default and switchable per repository: whether colleagues are told is the person's decision, but the default is to tell them.

### 8.7 When the head moves

If the author pushes while an agent is assigned, the assignment pauses: *The pull request changed since the agent started.* *Restart on the new head* keeps the findings whose lines are unchanged and lists the rest apart, as FEAT-093 does for the person's own pending comments. *Carry on* finishes against the old head, and says so on every finding.

## 9. Merge with an agent (1S)

### 9.1 What does not change

The plan, the dry run, *Merge now*, the resolver, *Abort* and the commit dialog are as they are in 1.3. Conflicts (1D) shares the resolver, so it gains the agent's origin badge ([9.4](#94-proposals-in-the-resolvers-own-vocabulary)) and nothing else.

### 9.2 Assigning

- **Where.** In the Result card, beside *Resolve N conflicts*; and in the resolving header. The graph's drag still only opens Merger, and the person assigns from there.
- **What to hand over**: *Resolve the conflicts* — the agent proposes, the person lands — or *Resolve, check and land*, which needs *Sign off* or *Unattended* to mean anything.
- **What stays the person's**: the two branches, where the result lands and how — Merge commit, Squash, Rebase then fast-forward, Fast-forward only. The agent may say in a note that another strategy would conflict less. It never changes the plan.

### 9.3 What the agent does

For each conflict, in the order of the files list: it reads both sides, the base, the commit on each side that made the change (the resolver already finds them with `blame`), and whatever else in the tree it needs — callers, tests. Then it proposes a resolution, with one sentence on why, and *sure* or *unsure*. After the last conflict come the checks.

### 9.4 Proposals in the resolver's own vocabulary

The design is the artboard *Merger · agent resolving*.

- **Spagitty names the choice, not the agent.** A local agent edits the conflicted file in its worktree; Spagitty reads each region back and compares it with the sides. If it is exactly A, B, A then B, B then A, or a line-by-line pick, it is that choice; otherwise it is *Edit*, the agent's own text. A remote agent proposes through a typed tool that takes the same choices. Either way the conflict card shows a choice the person already knows how to read.
- **A fourth origin.** Result lines are badged A, B and ✎ *typed by you* today. Text the agent wrote gets a mark and a colour of its own — distinct from A's blue, B's amber, ✎'s purple and the conflict-fix sky, and different from each of them in lightness. An agent's *Take A* is still badged A; the card says who chose it. Editing the agent's text by hand makes those lines ✎.
- **On the card**: the status chip says *Agent proposes: Both, A first*, with its sentence — *A renamed the function; B added a parameter to it; both kept, B's call updated to the new name.* Then *Accept*, the resolver's own chips (unchanged) to choose another way, and *Ask why*.
- **The files list's dots** gain one state: a ring in the agent's colour while it works on that conflict, and a half-filled dot once it has proposed. Green still means resolved.
- **Outside the conflicts**: refused ([6.3](#63-what-no-level-lifts), rule 6). If a local agent's worktree has changes outside a region, the timeline lists them — *changed src/api.rs:40, outside any conflict; not kept* — and the step is marked.

### 9.5 Checks

The repository's checks, as the farm runs them (`crates/spagitty-farm/src/verification/`), on a worktree holding the resolved result, with their output live in the timeline. They run after the last conflict, and again if a resolution changes afterwards. A failure stops at every level and says which check, with its output. At *Step by step* and *Sign off* the person may land anyway, and the commit dialog then says that checks failed. With no checks configured, the gate says so and *Unattended* is not offered.

### 9.6 Landing

The commit dialog (FEAT-101, FEAT-102) lists each conflict, what was chosen and by whom: *you*, *Codex, accepted by you*, or *Codex, unattended*. The message gains a `Co-authored-by` trailer naming the agent, which Spagitty already reads to attribute agent work on the graph.

An unattended landing also needs every conflict *sure*, checks passed and current, neither branch moved since the plan, and the receiving branch not on the repository's never-unattended list.

**Undo**, after an unattended landing: the done state and the notification offer *Undo the merge*, which moves the receiving branch back to the tip it had — only while nothing has been built on it and it has not been pushed, and saying why when it cannot.

### 9.7 A rebase that stops

FEAT-103's stops are taken one at a time: each stop's conflicts are proposed as above, and *Continue* is the gate between stops at *Step by step*. *Skip this commit* is never the agent's choice.

## 10. Talking to an agent while it works

| Control | What it does |
| --- | --- |
| **Pause** | Stops at the end of the current step, and says so: *Pausing after this file*. Cutting a step in half leaves nothing usable. |
| **Stop** | Ends the run now. Everything proposed so far stays as proposals. |
| **Take over** | Stops, and the screen is the hand-review or hand-merge screen with the agent's proposals in it. The card says *You took over at file 5 of 12*. |
| **Tell the agent…** | A line of text, delivered at the next step — or straight into the live session where the provider can resume one (`AgentTraits::resumable_sessions`). For a remote agent it is the next message. The timeline shows it as the person's. |
| **Redo with a note** | On a step or a proposal: the step runs again with what was wrong. The old proposal is kept, marked superseded. |
| **Ask why** | On a proposal: the agent says, in the timeline, what it read and why it chose this. It costs a step, and is not offered at *Unattended*. |
| **Level** | The menu on the card ([6.4](#64-changing-the-level-mid-way)). |

Every control is a button, listed in the palette with its shortcut. None is a gesture only.

## 11. Remote agents

### 11.1 Spagitty runs the loop

A command-line agent brings its own loop and its own tools. A model behind an API has neither, so Spagitty is its loop: it sends the job and the context; the model answers, or asks for a tool; Spagitty runs the tool and answers; until the model says the step is done.

### 11.2 A small, typed set of tools

Read-only, and answered in-process from the repository:

- list the changed files; read a file at a commit; read a diff; search the tree at a commit;
- read the pull request's description, threads and checks; read a conflict's sides and base;
- propose a comment; propose a resolution; propose a verdict; finish a step.

No shell, no writing, no network. Running checks is not a tool the model calls; it is a gate Spagitty runs. A request for anything not on the list is refused and logged.

### 11.3 What leaves the machine

- **Asked once per repository**, at the first assignment of a remote agent there: *This sends parts of team/app — the changed files, and any others the agent asks to read — to Anthropic.* Agree, or use a local agent instead. Changeable in Settings › Agents.
- **Counted as it happens**: each step in the timeline lists the files whose content was sent.
- **An endpoint on this machine** — `localhost`, a local model server — is said to keep everything here, and asks nothing.

### 11.4 Keys and requests

- The key is read from the keychain by the core at request time. The webview asks for a run and never holds a key — the forge's rule, unchanged.
- Requests go through the core's one HTTP client. Today that client is reachable only from `forge/http.rs`, and a test holds it so. 2.0 widens that by exactly one module, for model providers, and the test is changed to name the two — not relaxed.
- Keys and the `Authorization` header are redacted from the transcript and from every error shown.

### 11.5 Budgets and costs

Tokens in and out, per step and per assignment, as the provider reports them. Spagitty keeps no price list, which would be wrong within the month. A person who wants a cost can enter their own rates on the agent ([open question 10](#18-open-questions-for-the-author)).

## 12. Trust

- **Everything an agent reads is data, not instructions.** A pull request's description, its comments, its code and its commit messages are written by other people, and some will try to instruct an agent. The prompt says so; more importantly, the limits in [6.3](#63-what-no-level-lifts) are enforced by Spagitty and hold whatever the agent was persuaded to want.
- **Proposals are checked against what they claim.** A comment is placed only on lines of the diff. A resolution is accepted only for a conflict that exists, with sides unchanged since it was proposed — as the merger's saved choices are already checked on return.
- **The repository's rules go in the prompt**: `AGENTS.md` and the others, assembled as the farm assembles them (`crates/spagitty-farm/src/policy.rs`), so a review holds the change to what the project wrote down, not to an agent's taste.
- **Every decision has an author.** The record ([5.5](#55-the-record)) says, for each comment and each resolved region, who proposed it and who decided it, and the host and the commit say it too ([8.6](#86-what-the-author-of-the-pull-request-sees), [9.6](#96-landing)).

## 13. Designs

On the Claude Design canvas *Spagitty 2.0 — Agents in Review and Merger*, dark theme (Notte), drawn in the spatial shell's language. Fidelity is for structure, copy and the meaning of every colour; exact values come from `metrics.ts` and `app.css`, and the made-up repository, people and code are illustration.

| Artboard | What it shows |
| --- | --- |
| **Settings › Agents** | Local agents as detection found them, an API agent, the per-job switches, the defaults, and this repository's rules. |
| **Add an API agent** | Provider, key into the keychain, model, *Test*, limits, and what leaves the machine. |
| **Review · Assign an agent** | The inbox with its preview card, *Start review ▾* opened, and the Assign popover with the four levels. |
| **Review room · agent working** | The 1.3 room unchanged, plus: agent marks in the files list, the agent's band, two proposed comments, and the Agent tab with a gate waiting. |
| **Merger · agent resolving** | The resolver unchanged, plus: an agent proposal on a conflict, the fourth origin badge, the dots' new state, and the Agent card with checks running. |
| **Levels and gates** | The four levels against the gates of each job, and the limits no level lifts. |

## 14. What the code has today (read on 2026-10-08)

| Need | State |
| --- | --- |
| Finding and running command-line agents | ✅ `crates/spagitty-farm/src/agent/` — adapters for Claude Code, Codex, Cursor, Oh My Pi and Custom; `detector.rs`; `registry.rs` |
| Live, readable output | ✅ `execution/narrate.rs` narrates Claude Code's stream and passes others through verbatim; transcript lines reach the webview as `FarmEvent`s |
| Stopping a run | ✅ `execution/process.rs`, `execution/tree.rs` |
| A machine-wide list of agents | ❌ the registry is per repository (`.spagitty/farm/agents.json`), written when a farm opens; agents are set up in Farm › Setup, and Settings has no Agents section |
| Remote agents | ❌ `AgentInputMode::Api` exists so a definition round-trips; nothing drives it |
| Keys | ✅ `forge/keychain.rs` holds forge tokens; reuse it for provider keys |
| HTTP | ⚠️ one client, in the core, reachable only from `forge/http.rs`, and tested so |
| A reviewer's verdict | ✅ `review/decision.rs` — the `spagitty-review` fence; Approve, RequestChanges, Blocked; issues with severity, file and message. ❌ An issue has no line or range |
| No self-review | ✅ `review/reviewer.rs::check`, for farm tasks only |
| Pending comments and *Finish review* | ✅ FEAT-093, kept in the pull request's record. ❌ A pending comment has no author and no proposal state |
| A pull request's head on disk | ✅ fetched for the room (FEAT-091). ❌ No worktree of it since FEAT-095 removed *Open in worktree*; an agent needs a scratch one |
| Conflict-fix origin | ✅ FEAT-092 |
| Merger: dry run, resolver, landing, rebase stops | ✅ FEAT-100 to FEAT-103; choices kept in `merger_state` |
| Resolver origins | ⚠️ A, B and ✎ only |
| Running the repository's checks | ✅ `verification/` and `evidence.rs`, built for farm tasks; whether they run against a merge result outside a farm is still to be checked |
| Levels | ⚠️ the farm's `Autonomy` (Manual to Unattended, `model/farm.rs`) is farm-wide; nothing is per job |
| Notifications | ✅ FEAT-114 |

## 15. Why this is 2.0.0

From 1.0.0, MAJOR is the only bump that may break what came before (the preamble of `CHANGELOG.md`). 2.0 changes three things that people and files rely on.

1. **The agent registry gains a machine layer**, and the repository's file becomes an override ([4.5](#45-machine-repository-farm)). Repository files read as they did, but a 1.x Spagitty opening a repository that 2.0 has used will not see agents defined only on the machine.
2. **Half of FEAT-073's non-scope is reversed.** It said: *a model of our own, or an API key — Spagitty runs what is installed.* 2.0 still has no model of its own and ships no key, but it holds the person's key and makes model requests. That should be recorded in FEAT-073's item when the work starts, not discovered later.
3. **Code can leave the machine** for a party other than the forge, by the person's choice, per repository. The branding rule stands — nothing in Spagitty is a model, and no copy says *AI-powered* — and the release notes must say plainly what is sent and when.

## 16. Suggested slices

Each is one work item under Amendment 12, numbered when it starts, not before.

1. **Settings › Agents, local.** The section; detection moved from Farm › Setup; *Test*; what each agent may do; defaults; the repository's rules; the machine, repository and farm layers; the first-start offer.
2. **Assignments.** The job, the levels and gates, the limits, the record, the Agent card and its timeline, the controls, the rail's dots and notifications. Built and tested against a fake agent that follows a script, so every level and every limit has a test without a provider.
3. **Review with a local agent.** Assigning, the review worktree, proposals as authored pending comments, presence in the room, finishing at each level, attribution on the host, a head that moves.
4. **Merge with a local agent.** Proposals named by Spagitty, the fourth origin, checks on the result, landing and undo, rebase stops.
5. **Remote agents.** Providers, keys, the HTTP boundary, the loop and its tools, consent, budgets — then Review and Merger with a remote agent.

Dependencies: 2 needs 1. 3 and 4 need 2, and can run side by side. 5 needs 2, and its second half needs 3 and 4.

## 17. Non-scope

- **Agents writing code in a review**: suggested changes, or commits pushed to the pull request's branch.
- **Agents on the host**: pushing, opening pull requests, pressing the host's merge button.
- **Two agents on one job**, or agents handing work to each other — the farm's rule that agents never talk to each other holds here too.
- **Agent summaries in the inbox.** The Review handoff put summaries out of scope; an agent's summary lives in its own assignment.
- **Hosted agent services** that clone a repository on their own machines ([open question 7](#18-open-questions-for-the-author)).
- **Pull requests (1H) and Conflicts (1D)**, beyond the shared resolver's new badge.
- **A model, a key or a bill of Spagitty's own.**

## 18. Open questions for the author

1. **Where Agents sits** in Settings' chip index: after You, since it is about who works with you, or before Behaviour?
2. **A committed rule against remote agents.** Should a team be able to say, in a committed file, *never send this repository to a remote agent* — and is that a section of `AGENTS.md` or a file of its own?
3. **Attribution on the host**: *Drafted with <agent>*, on by default as proposed, and in those words?
4. **Verdicts from an unattended agent**: allowed at all, even with the repository's permission?
5. **Reading ahead at a gate**: may the agent go on reading — proposing nothing — while a gate waits, so the person is not the bottleneck at *Step by step*?
6. **One vocabulary or two**: should the farm's five autonomy levels and these four become one list?
7. **Hosted agent services**, as a third kind beside local and remote?
8. **A fix outside the conflicts**: always stop, as proposed, or allow the agent a separate, labelled commit after the merge at *Step by step*?
9. **The first-time default level** for each job: *Step by step* for both, as proposed?
10. **Costs**: tokens only, as proposed, or money from rates the person enters?
