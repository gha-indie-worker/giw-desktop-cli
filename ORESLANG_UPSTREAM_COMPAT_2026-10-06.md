# Oreslang upstream compatibility — 2026-10-06

This repository is audited against the latest 10 PRs in `ores-truffle-oreslang/oreslang-source.java` (#405, #406, #408, #409, #410, #411, #412, #414, #415, #416).

## Upstream contract

- #405 `do select` / `do nb select` no-result syntax; legacy select remains compatible.
- #406 arm-local `return` inside `do select`, with `W-SELECT-RETURN` diagnostics.
- #408 surfaces those warnings through the CLI.
- #409 additive runtime `SelectPlan`; no source rewrite required and traditional select stays on `SelectSet`.
- #410 permits task-local data-only `Channel<T>` at async boundaries and makes explicit `rt take` consuming.
- #411 defines binding capabilities: `const` fixed/read-only, `val` = `const mut`, `let` rebindable/read-only, `let mut` rebindable/mutable; adds select-case bindings.
- #412 expands do-select control-flow/actor-domain/atomic-dispatch coverage.
- #414 hardens source continuations so selected arms/cleanup run once.
- #415 adds `define actor` parsing/isolation groundwork and host `ready`/`done`; source `spawn Worker()` lifecycle is still deferred.
- #416 shares immutable checked code images across same-process actor contexts; not cross-process or machine-code sharing.

## Repository impact

Formatter/parser preservation is the main impact: retain `do select`, `do nb select`, binding-first select cases, and the #411 `const`/`val`/`let`/`mut` distinctions without normalizing them into older spellings.

## Integration policy

The ten PRs do not form one linear compiler head. Several are stacked on separate bases, and #415/#416 are a separate actor stack. Do not repin this repository to an arbitrary draft head and describe it as the integrated language. Migrate source to the intended contract, then advance the compiler ref only when the required upstream stack has exact-head green CI.

If source-org Actions cannot allocate a runner, mirror the exact source Git tree/blob SHAs into a funded test-org branch and run Actions there without private cross-org tokens or secrets.

## Guardrails

- `do select` / `do nb select` are no-result side-effecting forms; selected-arm returns are local/discarded.
- Traditional `select` remains supported and independent of #409 SelectPlan.
- `cb select` / `nb cb select` are noncanonical.
- Preserve #411 binding capability distinctions; do not mechanically rewrite all declarations to one keyword.
- Keep Oreslang pointer-free and use runtime ownership operations rather than `&` / `*`.
- Actor-mailbox data is serialized; local task channels may carry only what their local contract permits.
- #416 is same-process immutable code-data sharing, not cross-process or JIT-machine-code sharing.


## Main-branch recent-commit audit

The formatter is also checked against the latest main-branch language-bearing
commits through `f1866a7589259f6d000f71d8545c0afdbf68cbc8`.

Relevant merged contracts include:

- #395 / `67d0a7df...`: expression-bodied lambdas, explicit statement semicolons,
  nested closure/delimiter hardening, and slim-arrow executable syntax.
- #389 / `622c4cd9...`: `gen*` / `generator*` sugar, `yield*` delegation,
  and canonical `rt cooperate` with `rt yield` retained only as compatibility input.
- #380 / `5040b348...`: same-line `else if` disambiguation and explicit mutable
  method receivers alongside native Array/List member execution.
- #372 / `18a8c69a...`: callable channel payload rejection is semantic; the
  formatter preserves such source so the compiler remains the authority.

Formatter policy for these merged changes:

1. Canonicalize `gen*` / `generator*` declaration sugar to `generator`.
2. Preserve generator `yield` / `yield*`; rewrite only runtime `rt yield`
   to `rt cooperate`.
3. Treat a multiline pipe-lambda expression body as a continuation until the
   containing statement's explicit semicolon.
4. Do not collapse an `else` followed by a nested `if` on the next line into
   `elif`; only same-line `else if` is the branch alias.
5. Preserve explicit mutable receiver syntax and native collection member access.
