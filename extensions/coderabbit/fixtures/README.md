<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# CodeRabbit fixtures

**Every file here is constructed from CodeRabbit's published documentation; none
was captured from a real CLI run.** The names say so: `documented-*`.

They follow the `--agent` output contract in CodeRabbit's CLI reference
(<https://docs.coderabbit.ai/cli/reference>, "`--agent` review output",
"Failed or incomplete reviews") and the usage-based consent section of
<https://docs.coderabbit.ai/cli>, as read on 2026-10-06 for CLI 0.7.7–0.8.1.
Field names the documentation states are used as stated (`type`, `severity`,
`fileName`, `codegenInstructions`, `suggestions`, `comment`, `status`,
`findings`, `outcome`, `unreviewedFileCount`, `message`, `candidates`,
`candidatesNote`, `authenticated`, `confirmationHeadCommitId`). Where it names a
value without a field — the billable file count and maximum price of an
`action_required` result — the fixture's field names are a guess, and the
adapter reads several spellings.

No repository content, account, token or real commit is in any file.

**Replacing them with captured output** is the open step: with a signed-in
CLI, run the opt-in smoke test (`CODERABBIT_SMOKE=1`, see
`extensions/coderabbit/README.md`), redact what it writes to
`fixtures/captured/`, name the CLI version in the file names, and point the
adapter tests at them.
