# HyFormal

Unified expressions + zero-copy encoding for types, terms, and logic. This crate provides:

- A single expression language (in `expr::*`) covering type constructors (Bool, Omega, Powerset, Tuple),
  term constructors (Variable, Lambda, Call, If, Tuple), and logic (True, False, Not, And, Or, Implies, Iff, Equal, Forall, Exists).
- A compact append-only encoding (`encoding::tree::TreeBuf`) with borrowed decoding (`AnyExprRef`) for allocation-free traversal.
- A pretty-printer (`expr::pretty`) and a parser (`parser`) for a readable concrete syntax.

Quickstart

```rust
use hyformal::prelude::*;

let x = InlineVariable::new_from_raw(0);
let prop = forall(x, Bool, implies(x, equals(x, x)));
let e = prop.encode();
assert!(matches!(e.as_ref().view().type_(), expr::variant::ExprType::Forall));

## Arena-backed building and transformation

For short-lived construction and rewriting of expressions without allocation churn, use the arena API in `arena`.
You can mix structural arena nodes with borrowed pre-encoded subtrees and deep-copy results when needed.

```rust
use hyformal::arena::{ExprArenaCtx, ArenaAnyExpr};
use hyformal::expr::{Expr, view::ExprView, variant::ExprType};
use hyformal::expr::defs::{True, False, And};

let ctx = ExprArenaCtx::new();

// Mix arena views with borrowed encoded subtrees
let pre = And { lhs: True, rhs: False }.encode();
let leaf = ctx.reference_external(pre.as_ref());
let wrapped = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(leaf)));

// Deep copy an arena expression inside the same context
let copy = ctx.deep_copy(wrapped);

let e1 = wrapped.encode();
let e2 = copy.encode();
assert_eq!(e1.as_ref().type_(), ExprType::Not);
assert!(e1 == e2);
```
```

## Formal-system syntax

We use a single unified syntax where types, terms, and logical formulas are all expressions. Runtime checks are
responsible for validating well-formed uses (e.g., using type-shaped expressions in quantifiers or equalities).

```math
\begin{aligned}
  P &:=\;& \text{true} \mid \text{false} \mid P \land P | P \lor P \\
  &&\mid \neg P \mid P \Rightarrow P
  \mid P \Leftrightarrow P \mid \forall x: T.\; P \\
  &&\mid \exists x: T.\; P \mid E_1 = E_2 \\
  \\

  E &:=\;& P \mid x \mid \dagger \mid x(E) \\
  &&\mid  \text{if } P \text{ then } E_1 \text{ else } E_2 \mid E_1, E_2 \\
  \\

  T &:=\;& \text{Bool} \mid \Omega \mid \dagger \mid T_1 \rightarrow T_2 \mid T_1 \times T_2 \mid \mathcal{P}(T) \mid x \\
  &&
\end{aligned}
```

Where $x$ is a variable, $T$ is a type, $P$ is a proposition and $E$ is an expression. The expr $\dagger$ symbolizes a never (crash or hang). Notice
that generic types can be represented using the power set constructor $\mathcal{P}(T)$, however a strict type hierarchy **must** be observed to avoid
the [Russell's paradox](https://en.wikipedia.org/wiki/Russell%27s_paradox). We also decided against defining other useful types like *integers* as they
can be encoded as a type themselves.

## Axiom inference and automated proof

This section outlines how axioms and inference rules can be represented and how an automated prover can search for proofs in this formalism. The API is evolving; examples below describe the intended workflow that maps to the unified `expr` module.

### Core concepts

- Sequent: $\Gamma \vdash P$, where $\Gamma$ is a finite set (context) of propositions (assumptions) and $P$ a goal proposition.
- Rule: A partial function on sequents that, when applicable, reduces a goal into zero or more sub-goals. Axioms are rules with zero sub-goals.
- Derivation tree: A tree built by applying rules until all leaves are axioms; if such a tree exists, $\Gamma \vdash P$ is proven.

### Built-in axioms and rules (natural deduction style)

Propositional connectives:
- True-intro ($\top-I$): $\Gamma \vdash \text{true}$.
- False-elim ($\bot-E$): from $\Gamma \vdash \text{false}$ derive $\Gamma \vdash P$ (ex falso).
- And-intro ($\wedge-I$): from $\Gamma \vdash P$ and $\Gamma \vdash Q$ derive $\Gamma \vdash P \wedge Q$.
- And-elim ($\wedge-E_1/E_2$): from $\Gamma \vdash P \wedge Q$ derive $\Gamma \vdash P$ and $\Gamma \vdash Q$.
- Or-intro ($\vee$-I_1/I_2$): from $\Gamma \vdash P$ derive $\Gamma \vdash P \vee Q$; from $\Gamma \vdash Q$ derive $\Gamma \vdash P \vee Q$.
- Or-elim ($\vee-E$): from $\Gamma \vdash P \vee Q$, $\Gamma,P \vdash R$, and $\Gamma,Q \vdash R$ derive $\Gamma \vdash R$.
- Implication-intro ($\to-I$): from $\Gamma,P \vdash Q$ derive $\Gamma \vdash P \to Q$ (discharge $P$).
- Implication-elim ($\to-E$, Modus Ponens): from $\Gamma \vdash P \to Q$ and $\Gamma \vdash P$ derive $\Gamma \vdash Q$.
- Not: define $\neg P$ as $P \to \text{false}$; use implication rules + $\bot-E$.
- Iff-intro ($\leftrightarrow$-I): from $\Gamma \vdash P \to Q$ and $\Gamma \vdash Q \to P$ derive $\Gamma \vdash P \leftrightarrow Q$; elim derives each direction.

Quantifiers (first-order):
- Forall-intro ($\forall$-I): if $x$ is fresh in $\Gamma$ and the proof, from $\Gamma \vdash P[x]$ derive $\Gamma \vdash \forall x: T.\; P[x]$.
- Forall-elim ($\forall$-E): from $\Gamma \vdash \forall x: T.\; P[x]$ derive $\Gamma \vdash P[t]$ for any expr $t:T$.
- Exists-intro ($\exists$-I): from $\Gamma \vdash P[t]$ with $t:T$ derive $\Gamma \vdash \exists x: T.\; P[x]$.
- Exists-elim ($\exists$-E): from $\Gamma \vdash \exists x: T.\; P[x]$ and $\Gamma,P[x] \vdash R$ with $x$ fresh in $R$ derive $\Gamma \vdash R$.

Equality:
- Refl: $\Gamma \vdash E = E$.
- Subst: from $\Gamma \vdash E_1 = E_2$ and $\Gamma \vdash P[E_1]$ derive $\Gamma \vdash P[E_2]$, respecting typing.

These rules constitute the kernel of the prover. Domain theories can add axioms (e.g., arithmetic) but should preserve consistency by restricting to well-founded theories or using definitional extensions.

### Representation in code

- Unified expressions live under `src/expr`: see `defs.rs` (constructors), `func.rs` (builders), `variant.rs` (discriminants), `view.rs` (decoded views), and `pretty.rs` (pretty-printer).
- Dynamic encoding uses `encoding::tree::TreeBuf`; borrowing is via `AnyExprRef` and ownership via `AnyExpr`.

### Proof search strategy

We adopt a goal-directed, backward search with focusing:
- Normalization: Convert the goal to a normal form when helpful (e.g., push negations inward using $\neg P := P \to \text{false}$).
- Invertible rules first: Apply safe rules eagerly (e.g., $\wedge-E$, $\forall-E$, $\to-E$ on available assumptions) to reduce branching.
- Introduction at goal: Use intro rules to structure subgoals (e.g., to prove $P \wedge Q$, prove $P$ then $Q$; to prove $P \to Q$, assume $P$ and prove $Q$).
- Elimination on context: Use elim rules on $\Gamma$ to derive helpful inexprediate facts; maintain a derived-facts queue.
- Unification: For $\forall/\exists$ and $=$, compute substitutions that make side-conditions type-correct; prefer most-general unifiers.
- Loop checking: Cache visited sequents modulo $\alpha$-renaming and normalization to avoid cycles.
- Resource bounds: Limit depth, breadth, and time; use iterative deepening with heuristics (size of goal, connective weights).

### Lemmas and theory plugins

- Lemma database: A registry of named theorems (proven once) that can be used as rewrite or inference steps during proof search.
- Decision procedures: For specific theories (e.g., propositional SAT core, congruence closure for equality, simple quantifier instantiation), plug specialized solvers that produce proof objects.

### Proof objects and checking

- Each rule application yields a node with: rule name, premises (sub-goals), discharged assumptions, and side conditions (e.g., freshness, typing). The proof object is the full derivation tree.
- A small checker replays the proof using only the kernel rules; anything produced by plugins must reduce to kernel steps. This keeps the trusted base minimal.

### Example (sketch)

Goal: $\Gamma \vdash P \wedge Q$.
1) Apply $\wedge-I$ to split into subgoals $\Gamma \vdash P$ and $\Gamma \vdash Q$.
2) Solve each subgoal by context lookup, intro/elim, or plugin assistance.
3) When all leaves are axioms (e.g., $\Gamma$ contains $P$ or $\text{true}$), the proof is complete.

### Roadmap in this repo

1) Extend logical/type/term constructors in `expr/defs.rs` and add derived builders and rewrites.
2) Implement capture-avoiding substitution and α-equivalence services for variables in unified expressions.
3) Implement a minimal kernel of rules and a backtracking searcher over unified expressions, returning a proof tree.
4) Add a proof checker and a compact, human-readable pretty-printer for derivations.
5) Provide examples and tests (unit + property tests) to validate soundness of the kernel and completeness for propositional fragments.
