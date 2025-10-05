# HyFormal
This is a formal, symbolic engine for hyperion. It aims to provide a framework for automatic theorem proving and reasoning, using a formal system based on first-order logic.

## Formal-system syntax

We now use a single unified syntax where types, terms, and logical formulas are all expressions. Runtime checks are
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

Where $x$ is a variable, $T$ is a type, $P$ is a proposition and $E$ is an expression. The expr $\dagger$ symbolizes a never (crash or hang). Notice,
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

### Representation in code (sketch)

- Unified expressions live under `src/expr`: constructors for logic (True, False, And, Or, …), terms (Var, App, If, Tuple, …), and types (Bool, Arrow, Tuple, Power, …). A private sealing trait keeps construction controlled.
- Dynamic encoding uses a single RPN bytecode with opcodes in `encoding::magic` and zero-copy borrowed decoding via `DynBorrowedExpr`.

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
