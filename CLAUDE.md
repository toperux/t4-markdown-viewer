# CLAUDE.md

## Workflow for every change

Follow these steps in order. Don't skip a step, and don't merge two steps into one.

1. **Prompt.** The owner says what needs to happen or be implemented.
2. **Plan.** Write a plan in `docs/plans/<name>.md` for the owner to review and refine.
3. **Plan review loop.** Review the plan against the code, fix what the review finds, and review again. Repeat until
   a pass finds no issues.
   - When a finding needs a decision, ask the owner at once, not at the end of the loop, so the next pass reviews
     the answer.
   - Any change to the plan means another review pass: a fix, an answer to a question, or an edit by the owner.
4. **Execute, only on the owner's go.** A final plan is not a go. Ask first, and never start on your own. Commit
   the work locally.
5. **Change review loop.** Review the changes, fix the findings, and review again. Repeat until a pass finds no
   issues.
   - When a finding needs a decision, ask the owner at once, as in step 3.
   - Collect every finding that was skipped, and every limit to propose for acceptance. At the end of the loop,
     show the list to the owner.
6. **Triage, one item at a time with the owner.** For each item the owner decides one of:
   - **Fix it.** It goes through another fix → review loop (step 5).
   - **Defer it.** Add an unticked entry to `docs/open-items.md` under `## Deferred`, with a reopen trigger.
   - **Accept it.** With a reopen trigger, it's an open accepted item: `docs/open-items.md` → `## Accepted
     limits`. With none, it's a closed accepted item kept for reference: `docs/closed-items.md` → `## Accepted
     limits`.
7. **Package.** Only once the review-fix loops are done, squash related commits into logical ones for the push or
   PR.
   - Rehearse the squash in a throwaway worktree first, and check that the final tree is identical before moving
     the branch.
   - **Ask explicitly before every push, PR, tag and release.** Never do any of them on your own. A go on the task
     is not a go to push; each needs the owner's go-ahead at that moment.

## Commit messages

- **Summary line:** a plain sentence with no full stop at the end. Aim for 72 characters; never go over 100.
  - Without a prefix, it's a release-note line: say what changed for someone using the app, not which code changed.
  - With a `docs:`, `ci:` or `chore:` prefix, just say what changed.
  - After the colon, start lowercase unless the first word is a name, an acronym or a code identifier.
- **Prefixes:** start housekeeping commits with `docs:`, `ci:` or `chore:`, and name release commits
  `Release X.Y.Z`. Release notes are built from summary lines and leave these out (`release.yml`, "Build the release
  notes").
- **Body:**
  - None, if the summary line says it all.
  - Otherwise 1–2 lines.
  - A squashed commit, a release commit, or a commit with several topics gets one line per topic (none if the
    summary says it all), with a blank line between topics and no bullets.
  - Wrap lines at 72 characters.
- **No AI attribution:** no `Co-Authored-By: Claude` trailer, no `Claude-Session:` line, or any other AI attribution
  line.
- Dependabot's own commits and GitHub's merge commits (`Merge pull request #N …`) are exempt.

## Decisions

The owner makes every decision that has a real tradeoff, or where the options are about equally viable: review
rulings, plan changes and scope. Ask with a recommendation; never rule alone on these. A reviewer's "leave it" or
"fine as is" on such a choice is a recommendation to put to the owner, not a decision.

When one option is clearly right and the others are clearly worse, go with it and list what was chosen in the
report for that step or review pass. If unsure whether it's obvious, ask.

These stay with the owner however obvious they look: skipped findings, accepted limits and deferrals (step 6
triage), and every push, PR, tag, release and delete (step 7).

Explain each decision so it can be made without digging, even by someone who has been away from the work:

- **What it is.** The problem in plain words: what the user sees, or what breaks, and when. Don't just cite a
  finding number, a function or a file.
- **Why it's being asked now.** What triggered it, and what depends on the answer.
- **The options.** For each, what changes, what it costs, and what it risks.
- **What's known and what isn't.** Say what was measured and what is only reasoned or assumed. An unverified
  claim must not read as a fact.
- **The recommendation, and why.**

Be concise, but never drop a detail that could lead to a wrong choice: a side effect, an unmeasured case, a
platform difference, a cost that only shows later. Spell out every term and shorthand, such as check letters,
commit hashes and internal names. When a question depends on earlier context, restate that context in a
sentence; don't assume it is remembered.
