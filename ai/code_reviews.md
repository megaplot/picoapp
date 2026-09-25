# Code review rules and patterns

Of course in general standard code review rules apply -- high quality code, plus general patterns outlined in CLAUDE.md.

There are a few specific things to watch out for because the agents may be particularly prone to them:

- Re-use across code base: The agent either misses existing things, or introduces new things that should then be rolled out into existing code for consistency. Check for both. (In the UI layer, shared visual decisions belong in `src/ui/style.rs`, not re-composed locally.)
- Don't-repeat-yourself: This seems to be the biggest weakness of agents, which frequently produce non-DRY code.
- Bad placement of code: The agents seem to squeeze code in locally without paying attention to where, because they optimize for small edits. This can have nasty effects: I've seen cases where the agent placed a helper function in between a function and its docstring -- which is of course wrong. We care a lot about code ordering. When adding a function/method/test there should be a good motivation for placing it exactly *there* and not somewhere else -- e.g. logical grouping, call order, symmetries/asymmetries. Test modules should mirror the order of the main module for consistency and easier orientation, and more general test cases come before more specific ones. In Rust the `#[cfg(test)] mod tests` block goes at the end of the file, after all the code it tests (not in the middle of it). A highly exotic/special method of a type should not be added as the *first* method -- first methods should be the important ones, or the methods should follow typical call order. Always ask yourself: Why there?
- Ordering of fields or function arguments: The agents often have a very bad sense of ordering fields / function arguments (struct fields, enum variants, match arms, Python parameters). In many cases they form subgroups, or there are highly related sibling fields/args. The agents regularly mess these up by mixing groups, or putting a semantically different argument in between siblings. Other rules to consider: order from more basic/fundamental to more specific/exotic, and input args before output args. We care about meaningful orders. (Where Rust and Python mirror each other -- e.g. an input class's private attributes and the Rust extractor reading them -- keep the same order on both sides.)
- Removal and rewriting of comments: The agents tend to drop or reword comments accidentally. Adjusting a comment makes sense when the code it describes changed, and occasionally a comment fundamentally doesn't apply any more, then it can be removed. But if nothing fundamental has changed, a comment must not be removed or rephrased. We rely on comments to make sense of things; dropping them is an information loss. This includes `TODO`s, links (e.g. the PyO3 discussion link above `FromPyObject` for `Slider`) and "why" notes.
- Unnecessary renames and restructuring of existing code: Names of existing functions, types, fields, files and modules stay unless there is a reason that is part of the change. Do not convert existing idioms into different ones without a reason the original idiom cannot serve (e.g. replacing a `FromPyObject` impl -- the idiomatic PyO3 way to turn a Python object into a Rust value -- by a free function, or `x.foo()` style by a differently named helper). If a change needs a different shape, first check whether it can be done by *adding* next to the existing code instead of rewriting it.
- Effective `git diff` against `main`: When working on a feature branch and moving code around multiple times, watch the effective diff against `main`. We don't want to accumulate accidental diffs from moving things around and making tiny random changes (whitespace, reflowed comments, reordered imports/items, renamed locals, changed formatting of untouched lines). Any change in the diff against `main` must be intentional -- for every hunk that touches pre-existing code ask: *is this required by the feature?* If not, it should be reverted. Meaningful refactorings that are a clear improvement/fix are allowed to mix into a feature.
- Python <-> Rust contract: Class names and private attribute names are part of the contract between `python/picoapp/` and `src/` (see CLAUDE.md). Changes on one side without the matching change on the other are bugs; `_picoapp.pyi` must match the Rust signatures.
- Check whether CLAUDE.md (architecture, conventions, commands) needs adjustment for the change, and that no stale statements remain in it.
- Check for left-over debugging output, `println!`/`eprintln!`/`print`: picoapp must print nothing by default (its stdout/stderr belong to the user's callback).

**Concrete patterns**

Keep the diff of pre-existing code minimal:

```rust
// Avoid: rewording a comment that is still correct
-// Leaky abstraction: So far the following is only supported (or makes only sense)
-// for float sliders.
+// Only meaningful for float sliders.

// Avoid: renaming an existing item without need
-pub fn reactive_input_output_widget(..)
+pub fn build_level_widget(..)
```
