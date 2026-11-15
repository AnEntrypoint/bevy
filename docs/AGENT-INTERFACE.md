# Agent interface

How an agent works in this repository. The rules in AGENTS.md are the source of truth; this page explains the routine around them and does not override them.

## Operating rules

- Read AGENTS.md before editing. Its Rules section records non-obvious facts about the runtime, the build profiles and the measured baselines; check a fact against the code before relying on it, and update it when a change makes it false.
- Make the smallest change that satisfies the task. Do not add features, refactors or formatting outside the requested scope.
- Source files carry no comments. A fact the code cannot express goes into AGENTS.md, after checking it is still true.
- Keep generated output, benchmark logs and local reports out of commits. Build products live under `target/`.
- Report what was run and what was not run. A check that was skipped or could not run is stated as such, never described as passing.

## Git workflow

- Work on `main` in the checkout you were given. Do not clone the repository or create worktrees for scratch copies.
- Fetch before pushing: `git fetch origin main`, then rebase or merge onto `origin/main`, then push. A push that is behind origin is refused, and other agents and people also commit to `main`.
- Stage explicit paths with `git add <path>`. Never use `git add -A` or `git commit -a` in a shared checkout, because they sweep in other people's uncommitted work.
- Check `git status` and `git diff --cached` before committing, and confirm the staged set contains only the intended change.
- Commit subjects are one plain sentence that says what changed, for example "Add the agent interface page and link it from the rules." Do not use file names, function names or a diagnosis as the subject.
- The commit body explains how the change was made and why, in plain prose. Wrap it at about 72 columns.
- Commit as the configured git identity. Do not add co-author or tool attribution lines unless the repository owner asks for them.
- Push with `git push origin main` after committing, then confirm the remote head matches the local commit.

## Running checks and scripts

- Run the checks in the order the repository documents, starting with the cheapest. For this repository that is a debug build for quick checks, then the release scripts for verification.
- Long release builds with fat LTO should not run alongside another cargo build; use the dev profile for iteration.
- Scripts under `scripts/` are run from the repository root or with their absolute path. Read a script before running it, and pass paths as arguments.
- Scripts that start servers or browsers must stop them on exit. Check that no background process remains before finishing.
- Benchmark numbers are only comparable when measured back-to-back on the same host. Record the host and command with any number that is reported.

## Secrets

- Credentials live in an external token file outside the repository, readable only by its owner (mode 600). Point tools at that file through an environment variable or a credential helper, not by pasting the value.
- Never commit a token, key or password, and never add the secrets path to a tracked file. If a secret is committed by accident, revoke it first and then remove it from history.
- Never print a secret to the terminal, a log, a commit message, a PR description or a chat reply. Refer to it by file name only.
- Do not write a secret into a script or a fixture. Test fixtures use placeholder values.
