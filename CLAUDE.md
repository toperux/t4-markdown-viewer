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

## Decisions

The owner makes every decision: review rulings, skipped findings, accepted limits, deferrals and plan changes. A
reviewer's "leave it" or "fine as is" is a recommendation to put to the owner, not a decision. Ask with a
recommendation; never rule alone.

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
