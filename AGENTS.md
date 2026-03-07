# AGENTS.md

This file is the repo-local operating guide for coding agents working in `Machine_Empire`.

## Core Behavior

1. **Listen to all instructions** - Read the user's request carefully. Understand exactly what they are asking before doing anything.
2. **Only do what is asked** - Do NOT modify code that wasn't explicitly requested. Do NOT "fix" things that aren't broken. Do NOT make improvements or refactors unless specifically asked.
3. **Explain before making changes** - Before editing any file, explain what you plan to change and why. Wait for confirmation if the change is significant.
4. **Double check all work** - After making changes, verify they are correct. Re-read the original request to confirm you addressed what was actually asked, not what you assumed.
5. **Complete all tasks fully** - Do not simplify, Do not use placeholders, Complete all tasks fully and completely.
6. **When commiting and pushing to Github** - Commiter and Author should be Tarvorix...no mention of Claude anywhere in any commit message.
7. **No Placeholder Code** - Absolutely no placeholder code everything must be implemented fully
8. **No Simplification** - Absolutely no simplification of code everything must be implemented fully
9. **Preserve all existing functionality** - Never remove functionality from code

## File Deletion Rules

- NEVER delete any file or directory without explicit user confirmation
- Before ANY rm, rm -rf, or delete operation: list exactly what will be deleted and ask "Should I delete these? (yes/no)"
- Wait for explicit "yes" before proceeding
- No exceptions

## Before Any Code Change

Ask yourself:
1. Did the user explicitly ask for this change?
2. Is this code actually broken, or am I assuming?
3. Will this change affect other working functionality?
4. Have I explained what I'm about to do?

## If Uncertain

ASK. Do not guess. Do not assume. Ask the user to clarify.

## Git Author Configuration

All commits must use Tarvorix as author and committer. No mention of "Claude" anywhere in git commits.

Author name: Tarvorix
Committer name: Tarvorix
Never use "Claude" or any variation in commit author, committer, or commit messages or email
Configure git before committing:
`git config user.name "Tarvorix"`
`git config user.email "Tarvorix@users.noreply.github.com"`

## Project Overview

- Develop Machine Empire
- Global Strategy with RTS Game
- Use `Machine_Empire_Architecture.md`, `Machine_Empire_Game_Design.md` and `Machine_Empire_Art_Bible.md` for documentation while building.

## Project Rules

- Always create plan
- Always write plan to `todo.md`
- Update `todo.md` after every change

## Additional Repo Notes

- Keep campaign and RTS battle changes isolated when the user scopes work to one mode. Do not touch campaign if the request is RTS-only, and do not change RTS battle behavior when the request is campaign-only.
- `client/src/pkg` is generated WASM output. If Rust or WASM bridge code changes, refresh it with `npm run wasm:build` from `client/`.
- Standard verification commands:
  - `cargo test`
  - `npm run wasm:build`
  - `npm run build`
- GitHub Pages deployment is driven by pushing the relevant commit(s) to `main`. If the user asks for deployment, ensure generated artifacts that are expected in-repo are up to date before pushing.
- Prefer surgical fixes over refactors. This repo has active in-progress work; do not revert unrelated user changes.
- Use `todo.md` chunk entries to record meaningful work so later turns have a clear project log.
