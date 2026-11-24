# Rule scans

Read this when running or continuing a rule-scan pass. A pass reads a repository against `core.md`, the references it imports and its `AGENTS.md` files, and fixes where the code misses a written rule; it invents no rules.

## Scope and verification

- A pass covers the whole repository, or the projects it names at the start.
- Only verified findings count. A finding names the rule (file and rule title), the code that misses it (path and line) and why it misses, confirmed by reading that code on the current `main`. A suspicion, an unread pattern match, or a finding already fixed or on the roadmap is not counted.

## Classes

Each verified finding has one class:

- **Bug:** behaviour is wrong for an end user, an operator or the data (a wrong answer, a lost write, a double charge).
- **Copy or rule miss:** behaviour is right, but the code breaks a rule: a second copy of a value or rule, a crossed layer, a missing type.
- **UI-only:** fixing it changes what a screen shows.
- **Question for the user:** fixing it needs a choice the written rules do not make, or two rules conflict.

## Fixing

- **One PR per finding,** mergeable on its own.
- **Behaviour first.** A fix comes with a test of the owning function's behaviour that fails before the fix.
- **A source guard only where it binds a rule.** A test that reads source text is added only for a structural rule (layers, imports, the layout against its `AGENTS.md` list, declaration order, typed ids, a committed generated file, a closed set against its source), or as a row in the project's source-rules test naming the file that owns the rule and the reason, when nothing else can see the copy.
- **No guard that only pins a past spelling:** none that refuses one old spelling in one or two files, or repeats what a type check, a lint rule or another guard refuses. A pattern the linter can check goes to the linter.
- **UI-only findings and questions for the user are listed apart and never fixed silently.** A UI-only fix waits for the user's look at its before-and-after screenshots (`web-interface.md`); a question goes to the user with options and a recommendation, and its answer is recorded in a decision record before any fix.

## Records

- **One research note per pass** beside the decision records: date, scope, and each verified finding with its class, rule, location and PR or roadmap item.
- **One roadmap line per pass:** its date, its count of verified findings by class, and its PRs. Findings not fixed in the pass stay on the roadmap.

## When the series stops

- A pass with fewer than five verified findings and no bug ends the series of full scans.
- After that, each PR's review ("Review before every PR" in `core.md`) checks the rules for its own diff, and one full pass runs before each release, under this procedure.
