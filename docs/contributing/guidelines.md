# Guidelines

How work is organised, written and landed here. Four documents, each answering a
different question.

## The guides

| | Answers |
| :--- | :--- |
| [branching.md](guides/branching.md) | Where does this work go, and how big should it be? |
| [writing.md](guides/writing.md) | How do I write the commit, the pull request, the release note, the README? |
| [merging.md](guides/merging.md) | How does this land on `main`? |

Read in that order the first time. After that they are reference.

## Planning notes are not published

`.artifacts/` holds the plans, decisions and sprint records this work is shaped
by. It is gitignored, so a fresh clone does not have it and a reader of the
public history cannot open it.

Never cite it. A commit or pull request naming a milestone, a step letter or an
ADR number points at something nobody outside can read, and it dates the moment
the plan moves on. Say what the work is FOR instead.

## The short version

**One observable goal per branch**, and the prefix says which kind of work it is.
`bootstrap/` is the exception, being a milestone rather than a goal.

**The branch prefix decides how it merges.**
`bootstrap/` gets a merge commit; `feat/` and `fix/` are squashed. One line on
`main` per unit of work, so `git log --first-parent` reads as a list of what
landed.

**Write for the reader you actually have.**
A commit is read by someone running `git blame` in two years; a pull request by a
reviewer now; a release note by someone upgrading; a README by someone deciding
whether to stay. Most bad writing in all four is aimed at the wrong one.

**Plain ASCII in prose.**
No em dashes, curly quotes, arrows or emoji, in commits, pull requests, releases
or documentation. Source and what a program prints are not prose and are not
covered. A character you cannot type is one that gets pasted
inconsistently.

**`main` is always green.**
Every commit on it compiles, passes its checks, and runs.

## Conventions that live elsewhere

- Commit types and scopes are validated by
  [`scripts/commitlint.luau`](../../scripts/commitlint.luau). Scopes map to
  directories.
- The applet contract, for anyone writing a widget rather than changing the host,
  is [docs/applet_contract.md](../applet_contract.md).
