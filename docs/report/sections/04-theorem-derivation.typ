= Theorem derivation

In hyperion framework we wanted optimizations to be more powerful than traditional compiler passes, which typically rely on local, pattern-based rewrites. Instead, we aimed to enable optimizations that utilise global context and semantic understanding of functions, allowing for more sophisticated transformations, including those that can change the asymptotic complexity of algorithms or introduce parallelism. With this in mind, we designed a *theorem derivation* system that automatically discovers and proves equivalence-preserving assertions about functions. These assertions serve as a foundation for safe optimizations, providing the necessary facts to justify complex rewrites.

== Example of an assertion on `pow`

Consider the kernel presented in @ir-kernel-example-pow, which computes the power of a base integer raised to an exponent using a loop. The end goal of theorem derivation in this context is to automatically derive the following theorem about this function:

#figure(
  ```llvm
  define void pow_t1(%a: i32, %b: i32, %c: i32) {
  entry:
    %term: i8 = !analysis.term.blockexit ; This block will always terminate
    %ok_term: i1 = icmp.eq %term, i8 0 ; Normal termination (always terminate)
    !assert %ok_term
    %pab: i32 = invoke pow, %a, %b
    %pac: i32 = invoke pow, %a, %c
    %pas: i32 = imul.wrap %pab, %pac
    %sum: i32 = iadd.wrap %b, %c
    %pas2: i32 = invoke pow, %a, %sum
    %ok: i1 = icmp.eq %pas, %pas2
    !assert %ok
    ret void
  }
  define void pow_t2(%a: i32) {
  entry:
    %pa0: i32 = invoke pow, %a, i32 0
    %ok0: i1 = icmp.eq %pa0, i32 1
    !assert %ok0
    %pa1: i32 = invoke pow, %a, i32 1
    %ok1: i1 = icmp.eq %pa1, %a
    !assert %ok1
    %pa2: i32 = invoke pow, i32 1, %a
    %ok2: i1 = icmp.eq %pa2, i32 1
    %a_eq_zero: i1 = icmp.eq %a, i32 0
    %okk2: i1 = ior %ok2, %a_eq_zero
    !assert %okk2
    ret void
  }
  ```,
  caption: [Example of theorems for `pow` in @ir-kernel-example-pow],
  kind: "code",
  supplement: [Code],
)

Future extension will also enable to encapsulate more complex analysis such as (1) time complexity, (2) probabilistic behavior, and (3) termination properties.

== Theorem derivation overview

In hyperion, theorem derivation is the process of automatically generating and proving assertions about functions in the intermediate representation (IR). These assertions capture semantic properties that are essential for justifying optimizations. We devise strategies to derive these assertions, leveraging a combination of normalization, theorem application, and verification condition generation.

#enum(
  [
    *Simplification and normalization*: Transform pre-existing proofs by applying a set of canonicalization rules that preserve semantics. This step ensures that the function is in a stable form for further reasoning.
  ],
  [
    *Theorem application*: Utilize the `TheoremLibrary` to identify and apply relevant theorems to derive new assertions about the function. This involves pattern matching and side-condition checking.
  ],
  [
    *Loop pattern recognition*: Identify common loop patterns (e.g., reductions, counting loops) and derive invariants or summaries that capture their behavior.
  ],
  [
    *Loop Invariant Generation*: Generate loop invariants that hold at loop headers, enabling reasoning about loops and their transformations (check CEGAR-style refinement).
  ],
  [
    *Verification Condition (VC) generation*: Formulate VCs that must hold for the function to satisfy certain properties. These VCs are then discharged using automated theorem proving techniques.
  ],
  [
    *Similar function reuse*: Leverage previously derived assertions and proofs from similar functions to accelerate the derivation process. This problem is hard and still under research.
  ],
)

== From theorems to transformations

Certain theorems can be sufficient to provide ground for specific optimizations. We introduce transformation strategy which rely upon the derived assertions to safely apply optimizations. Each optimization is treated as a guarded rewrite, where the derived assertions serve as the guards that justify the transformation.

Examples of such transformations include:
- *Divide / divide-and-conquer optimizations*: If a loop's invariant and update rule show the iteration space can be partitioned into independent regions, split the work into sub-loops that run in parallel; more advanced strategies relax "independence" by adding a provably-correct combine step.
- *Operation reordering*: If derived assertions establish non-aliasing, commutativity, or independence between effects, reorder instructions (or whole blocks) to reduce stalls, improve locality, and increase ILP (without changing semantics).
- *Loop unrolling, fusion, and fission*: Using loop summaries (reads/writes, carried dependencies, invariants), transform loop structure—unroll to reduce overhead and expose vectorization; fuse to reuse data in cache; fission to separate independent streams or isolate slow paths (sometimes *adding* loops improves cache behavior).
- *Brute-force parallelization*: When a search is correct-by-enumeration, prove the search space partitions and the aggregation operator is sound; then distribute chunks across threads/GPUs and combine results.
- *Greedy rewrites*: When exact optimization is infeasible, apply locally-justified rules that preserve validity and optionally provide an approximation guarantee under stated assumptions.
- *Dynamic-programming optimizations*: If you can prove overlapping subproblems and optimal substructure, replace naive recursion or redundant computation with memoization/tabulation and (when safe) reorder evaluation to improve locality/parallelism.

#pagebreak()

=== Divide / divide-and-conquer optimizations

*A. Properties to derive (what you must know)*
1. *Partitionability of state/data*: You can split the "big object" of size $N$ into $k$ parts $P_0, ..., P_(k - 1)$, each of size $N\/k$, and each iteration touches only one part (or a known neighborhood).
2. *No cross-part interference* (strong form): writes in part $P_i$ do not affect reads/writes in $P_j$ for $i != j$.
3. *Or: compositionality* (weaker form, divide-and-conquer): each part produces a summary $S_i$, and there exists a deterministic, associative (or otherwise proven-correct) *combine* operator $times.o$ such that overall result is $S_0 times.o S_1 times.o ... times.o S_(k-1)$.

*B. Theorem shapes (sound rewrite rules)*
- *Loop partition theorem*:
  If `iterations = U groups` and groups are independent, then
  `for i in iterations: body(i)` $eq.triple$ `parallel for g in groups: for i in g: body(i)`.
- *Map/Reduce theorem* (divide-and-conquer):
  If `body` can be written as `map` producing summaries and `reduce` combining them with $times.o$, then sequential fold $eq.triple$ parallel reduction.

*C. How to apply (procedure)*
1. Prove or infer a *footprint* per iteration: `RW(i)` (read/write set).
2. Choose partition function `part(i) -> {0..k-1}`.
3. Check either:
  - $"RW"_i inter "RW"_j = emptyset$ for $"part"(i) != "part"(j)$ (independence), or
  - the only interaction is through a summary with a proven combine operator.
4. Rewrite:
  - Create `k` subloops (or a parallel loop over parts).
  - If using summaries: allocate per-part accumulator $S_i$, compute locally, then combine with $times.o$.
5. Validate side conditions: determinism, absence of hidden global state, and that combine preserves semantics.

=== Operation reordering

*A. Properties*
1. *Non-aliasing / disjointness*: two pointers/regions don't refer to overlapping memory.
2. *Effect independence*: instruction `A` does not read/write anything that `B` writes/reads in a conflicting way.
3. *Commutativity / associativity* (when reordering arithmetic or reductions): reordering doesn't change value (or changes only within accepted FP error bounds, if that's a permitted spec).

*B. Theorem shapes*
- *Commutation theorem*: If $A$ and $B$ are independent, then $A; B eq.triple B; A$.
- *Code motion theorem*: If moving `A` across a region does not violate dependencies, then hoist/sink is semantics-preserving.

*C. How to apply*
1. Compute dependency edges (RAW/WAR/WAW + control dependencies).
2. Use alias facts and effect summaries to remove false dependencies.
3. Apply theorems to swap or move operations:
  - swap adjacent independent ops repeatedly, or
  - reorder within a basic block/topological schedule.
4. Re-check: no dependency cycles introduced; observable behavior (I/O, volatile, atomics) unchanged.

=== Loop unrolling, fusion, and fission

*A. Properties*
- For *unrolling*:
  1. No loop-carried dependency that spans more than the unroll factor in a way that breaks correctness.
  2. Bounds and guards are handled (remainder loop correctness).
- For *fusion*:
  1. Two loops have compatible iteration spaces.
  2. No harmful dependence from the second loop to the first that requires separation (or it can be ordered within a fused body).
  3. Benefit property: improved locality/reuse.
- For *fission*:
  1. The loop body can be split into phases with no cross-phase carried dependence (or a proven-safe pipeline).

*B. Theorem shapes*
- *Unroll theorem*: `for i: A(i)` $eq.triple$ `for i step u: A(i); ...; A(i+u-1)` + remainder, under dependence/bounds conditions.
- *Fusion theorem*: `for i: A(i); for i: B(i)` $eq.triple$ `for i: A(i); B(i)` if dependencies allow.
- *Fission theorem*: `for i: A(i); B(i)` $eq.triple$ `for i: A(i); for i: B(i)` if `A` and `B` are separable.

*C. How to apply*
1. Build loop summary: iteration domain, per-iteration `RW`, and loop-carried deps.
2. Pick transform:
  - Unroll: choose factor `u`, generate unrolled body + cleanup loop.
  - Fuse: align bounds/strides, merge bodies, then schedule within fused body respecting deps.
  - Fission: split body into groups of statements, then emit multiple loops.
3. Re-check invariants: induction variables, exit conditions, and preserved ordering for side-effecting ops.
4. Benchmark-guided: apply only when summaries predict wins (branch overhead, cache reuse, vectorization).

  === Brute-force search parallelization

*A. Properties*
1. *Search space partition*: candidate set `C` can be split into `C0..Ck-1`.
2. *Pure evaluation*: checking a candidate is deterministic and has no shared side effects (or effects are isolated).
3. *Aggregation correctness*: “best”/“exists”/“count” operator is associative/commutative or otherwise order-independent (or you preserve a defined order).

*B. Theorem shapes*
- *Parallel enumeration theorem*: `fold(op, map(eval, C))` $eq.triple$ `reduce(op, [fold(op, map(eval, Ci))])`.
- *Early-exit theorem* (for existential queries): if `op` is short-circuiting (e.g., OR), you can stop when a witness is found, provided cancellation doesn't affect semantics.

*C. How to apply*
1. Define candidate generator and evaluation function.
2. Choose partitioning strategy (range split, hashing, domain decomposition).
3. Compute per-worker partial result.
4. Combine partial results with proven-correct aggregator.
5. Ensure reproducibility if required (deterministic tie-breaking for “best”).

=== Greedy algorithms (as theorem-driven approximations)

*A. Properties*
1. *Local choice validity*: a local decision never produces an invalid state (invariants preserved).
2. *Progress / termination*: each step decreases a measure.
3. Optionally: *approximation guarantee* under stated assumptions (matroid property, submodularity, etc.).

*B. Theorem shapes*
- *Invariant-preserving step theorem*: If invariant holds before step and the greedy choice satisfies condition `G`, invariant holds after.
- *Approximation theorem* (optional): Greedy result is within factor $alpha$ of optimal given property `P`.
- *Proof of bound*: For a certain class of problems, greedy always yields an optimal solution. This class of problems can be characterized by specific properties (e.g., matroids). Prooving a problem is a matroid can help justify the optimality of greedy solutions.

*C. How to apply*
1. Identify the invariant that defines “valid partial solution”.
2. Prove greedy step preserves it.
3. Prove termination/progress.
4. If you need guarantees, state and prove the structural assumption (e.g., submodular objective) and then apply the approximation theorem.

=== Dynamic programming optimizations

*A. Properties*
1. *Optimal substructure*: optimal solution for a state depends on optimal solutions of smaller states.
2. *Overlapping subproblems*: many calls repeat the same state.
3. *State definition*: a finite key that uniquely represents a subproblem.

*B. Theorem shapes*
- *Memoization theorem*: replacing repeated evaluation of pure function `f(s)` with a cache keyed by `s` preserves semantics.
- *Tabulation theorem*: recursive definition ≡ iterative fill in a topological order over the dependency DAG of states.

*C. How to apply*
1. Extract the recurrence and identify the state variables.
2. Prove purity/determinism of the recurrence (or isolate side effects).
3. Choose memoization (top-down) or tabulation (bottom-up):
  - Memoization: add cache, keep recursion.
  - Tabulation: compute dependency order, allocate table, fill iteratively.
4. Validate:
  - Base cases preserved.
  - Table order respects dependencies.
  - Memory/time bounds match expectations.

#pagebreak()

== BULLSHIT BELLOW
Optimization is only safe if it preserves the function's observable behavior. In Hyperion's setting, "equivalence" must account for more than return values: it must preserve the input-output relation together with relevant state behavior as well as memory and control effects.

A suitable contract is:
- *Input-output relation*: for the same inputs, the original and transformed functions produce the same outputs.
- *State behavior*: for all reachable executions, the same externally-observable state effects occur (e.g., memory reads/writes, trapped behavior if any is modeled, and control-dependent behaviors that are observable via outputs or state).
- *Reachable state preservation*: the transformation must not introduce new reachable states nor remove reachable states, relative to the semantics under which the IR is defined.
- *Memory model adherence*: all memory operations must respect the defined memory model (e.g., no out-of-bounds accesses, no undefined behavior due to aliasing violations). This is relaxed to only memory operations that are observable (i.e., reads/writes that affect outputs or state).

Equivalence-preserving assertions are needed because most optimizations are only valid under side-conditions: non-aliasing, bounds safety, absence of overflow (or the presence of explicit wrap/saturating semantics), and floating-point restrictions. Assertions make these side-conditions explicit, verifiable, and reusable.

== Core artifact: `FunctionView`

`FunctionView` is the central product of theorem derivation for a single function.

Definition (draft): `FunctionView` is a canonical semantic view of a function together with an internally verified set of facts (assertions) that are sufficient to justify a class of equivalence-preserving rewrites.

A `FunctionView` contains:

- A normalized representation of the function suitable for reasoning (canonical SSA names, canonical block/phi forms).
- A set of *assertions* proven about the function (loop invariants, path predicates, bounds, alias facts, algebraic identities, FP side-conditions).
- A set of *proof objects* (certificates) for the assertions, so that downstream optimizers can trust and reuse them.
- Optional *summaries* and *path conditions* that allow local reasoning to be lifted to whole-function equivalence.

Production and consumption:

- The derivation engine constructs `FunctionView` from the IR function, performing normalization and generating verification conditions.
- Optimizations query `FunctionView` for facts needed to justify candidate rewrites.
- When a rewrite is applied, its proof obligation is discharged under the `FunctionView` assertions and recorded as a certificate.

== Assertion taxonomy

Assertions are classified by their role in guarded rewriting.

=== Loop invariants and summaries

- *Loop invariants*: predicates over SSA state that hold at loop headers.
- *Summaries*: relationships between inputs and outputs of a loop or region, often expressed as a closed-form expression or an inductively-defined relation.

These are the primary enablers for replacing loops with closed forms, folding reductions, and proving strength reductions.

=== Path predicates

Path predicates capture the condition under which a block or region is executed. They are essential for reasoning about piecewise functions and about transformations that rely on branch feasibility.

=== Bounds and alias facts

For memory reasoning, two families of facts are critical:

- *Bounds*: which addresses are in-range, alignment constraints, and index ranges.
- *Alias/purity*: when two addresses are provably distinct, when a function is pure (no writes), and when a load is invariant across a region.

=== Algebraic identities and domain side-conditions

- Integer identities must respect explicit overflow semantics (wrap vs saturating).
- Floating-point identities must respect NaN/Inf/signed-zero behavior and are conservative by default.

=== Floating-point permissions (rewrite enablement)

Rewrites such as reassociation, commutation, or contraction are only allowed if a sufficient permission is proven and recorded. This permission is best treated as an assertion in `FunctionView` (rather than as a global flag) so that it can be scoped and justified.

=== Termination and reachability facts (optional but related)

Termination and reachability facts can be represented as meta-level artifacts. They are related but separable: equivalence-preserving optimization must be correct even without a complete reachability analysis, but some transformations (e.g., dead-branch elimination) rely on provable unreachability.

== Derivation pipeline (methodology)

The derivation pipeline is designed to be deterministic and terminating, and to produce reusable artifacts.

=== Normalization and simplification (strictly equivalence-preserving)

Apply a terminating set of canonicalization rules that do not change semantics:

- Canonical SSA naming and block ordering.
- Canonical `phi` placement and predecessor ordering.
- Simple local simplifications with explicit side-conditions.

Rule families (illustrative):

- Boolean algebra for `i1`: idempotence, absorption, constant folding.
- Integer algebra respecting overflow intent: identities for wrap/sat operators and constant folding when exact.
- Conservative floating-point identities: e.g., `x * 1.0`, `x + 0.0` where exact.
- Control simplifications when conditions are provably constant under the current proof context.
- Memory simplifications only under proven alias/bounds facts.

The intent is to make subsequent proof search and theorem application stable by putting the function into a canonical form.

=== Theorem library application

A theorem library contains typed theorem schemas:

- Premises: a pattern over IR expressions/regions.
- Side-conditions: well-typedness, absence of UB (as defined), alias/bounds, FP permissions.
- Conclusion: a derived assertion or an equivalence between an “old” and “new” IR fragment.

Application proceeds by pattern search (e.g., opcode/type/shape indexing) with alpha-renaming, followed by side-condition discharge. Each successful application produces:

- A derived assertion stored in `FunctionView`.
- A proof object referencing the instantiated theorem and discharged side-conditions.

=== Strategy layer: patterns, VC generation, and refinement

Beyond local theorems, a strategy layer identifies higher-level patterns and generates verification conditions (VCs).

Typical strategies:

- *Loop pattern recognition*: counting loops, sentinel loops, reductions.
- *Region summarization*: derive a summary for a region and re-use it for multiple rewrites.
- *VC generation*: generate weakest-precondition or strongest-postcondition constraints over SSA.
- *CEGAR-style refinement*: propose a candidate invariant/summary; attempt proof; if refuted, refine using counterexamples.

=== Proof production and caching

Proof production should be explicit and compositional:

- Each assertion and each applied rewrite is associated with a certificate.
- Certificates may be discharged by SMT, proof-checking kernels, or specialized decision procedures.
- Proof caching stores reusable certificates and normal forms to accelerate repeated derivations and enable algorithm reuse.

== Rewrite gating: applying transformations safely

Every candidate optimization is treated as a guarded rewrite:

1. *Candidate selection*: identify a local or regional IR pattern that could be replaced.
2. *Side-condition query*: query `FunctionView` for required facts (e.g., bounds, non-aliasing, FP permissions).
3. *Obligation generation*: generate a proof obligation of equivalence under the current assertions.
4. *Discharge*: prove the obligation; if successful, record the certificate.
5. *Apply rewrite*: rewrite the IR fragment, preserving well-formedness.

#align(center)[
  #box(
    inset: 10pt,
    radius: 4pt,
    fill: luma(248),
    stroke: luma(220),
    [
      #grid(
        columns: (1fr, 1fr, 1fr, 1fr, 1fr),
        column-gutter: 10pt,
        row-gutter: 6pt,

        [#align(center)[*Candidate*]],
        [#align(center)[*Facts*]],
        [#align(center)[*Obligation*]],
        [#align(center)[*Proof*]],
        [#align(center)[*Rewrite*]],

        [pattern match], [query `FunctionView`], [generate VC], [discharge + certificate], [apply + record],
      )
    ],
  )
]

This gating approach ensures that optimizations never rely on undocumented assumptions. It also creates a durable record explaining why a rewrite was allowed.

== Safety: avoiding unsound FP and memory rewrites

=== Floating point

By default, floating-point is treated conservatively:

- Avoid reassociation and commutation.
- Treat NaNs, infinities, and signed zeros as semantically relevant.
- Allow only identities that are provably exact and do not change exceptional behavior.

If more aggressive FP rewrites are desired, they must be enabled by explicit, proven permissions recorded in `FunctionView`.

=== Memory

Memory rewrites require explicit proof obligations:

- Non-aliasing between addresses involved in reorderings or eliminations.
- Bounds and alignment for loads.
- Absence of intervening writes (or proof that a write does not affect the loaded location).

== Worked examples (conceptual archetypes)

The examples below illustrate the kind of invariants/summaries that derivation should produce and the rewrites they enable.

=== Pow-like loops (integer saturating)

Archetype: a loop multiplies an accumulator by a base for a decreasing counter.

Derived artifacts:

- Invariant at loop header: the accumulator equals a saturating power of the base for the progress already made.
- Bounds/progress fact: the counter remains non-negative and decreases toward zero.

Enabled rewrites:

- Replace the loop by a closed-form saturating exponentiation intrinsic, if available.
- Apply strength reductions and constant folding inside the loop, but only if the exit condition and invariant are preserved.

=== Dot-product reductions (floating point)

Archetype: fold `acc := acc + a[i]*b[i]` over a range.

Derived artifacts:

- Index bounds: `i` ranges over a half-open interval.
- Summary: `acc` equals the sequence-defined sum with the original evaluation order.
- Memory facts: loads are in-bounds and, when needed, non-aliasing.

Enabled or forbidden rewrites:

- Enable loop-to-reduction recognition for code generation while preserving evaluation order.
- Forbid algebraic reassociation unless an FP permission is proven.

=== Max reductions (unsigned integers)

Archetype: fold `m := max(m, data[i])`.

Derived artifacts:

- Summary: `m` equals the maximum over the prefix processed so far.
- Index bounds and in-bounds loads.

Enabled rewrites:

- Replace the loop by a max-reduction intrinsic under a proven memory model.

=== Clamp piecewise semantics

Archetype: a conditional chain that returns `lo` if `x < lo`, `hi` if `x > hi`, else `x`.

Derived artifacts:

- Range assertion: result lies in `[lo, hi]`.
- Piecewise characterization under path predicates.

Enabled rewrites:

- Replace with a clamp intrinsic *only if* the intrinsic’s semantics match the piecewise definition exactly.

=== Newton iteration (floating point)

Archetype: iterate `y_{k+1} = 0.5*(y_k + x/y_k)`.

Derived artifacts:

- Conditional facts (e.g., `x > 0` implies `y_k > 0` if the iteration is well-formed).
- Heuristic monotonicity or convergence claims must be marked as such unless proven under the chosen FP semantics.

Enabled or forbidden rewrites:

- Enable recognition for specialized implementations while maintaining conservative FP semantics.
- Forbid “mathematically true” transforms that are not FP-sound.

== Engineering concerns

=== Determinism and termination

The derivation engine must be deterministic:

- Deterministic rule ordering and canonicalization.
- Saturation limits, cost metrics, and memoization to ensure termination.

=== Proof object format and storage

Proof objects should be versioned and portable:

- Stable identifiers for theorem schemas.
- Explicit bindings and discharged side-conditions.
- Storage suitable for caching and regression testing.

=== Regression strategy

A regression suite should cover:

- Rewrite soundness: each rewrite rule has positive and negative tests.
- End-to-end examples: functions with known summaries and expected derived assertions.
- Proof stability: changes to normalization do not invalidate proofs unless semantics change.

The overall goal is that theorem derivation acts as the correctness substrate for optimization, providing both the facts that enable rewrites and the certificates that justify them.
