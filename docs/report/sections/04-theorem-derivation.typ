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

- *Properties to derive* (what you must know)
  1. *Partitionability of state/data*: You can split the "big object" of size $N$ into $k$ parts $P_0, ..., P_(k - 1)$, each of size $N\/k$, and each iteration touches only one part (or a known neighborhood).
  2. *No cross-part interference* (strong form): writes in part $P_i$ do not affect reads/writes in $P_j$ for $i != j$.
  3. *Or: compositionality* (weaker form, divide-and-conquer): each part produces a summary $S_i$, and there exists a deterministic, associative (or otherwise proven-correct) *combine* operator $times.o$ such that overall result is $S_0 times.o S_1 times.o ... times.o S_(k-1)$.

- *Theorem shapes* (sound rewrite rules)
  - *Loop partition theorem*:
    If `iterations = U groups` and groups are independent, then
    `for i in iterations: body(i)` $eq.triple$ `parallel for g in groups: for i in g: body(i)`.
  - *Map/Reduce theorem* (divide-and-conquer):
    If `body` can be written as `map` producing summaries and `reduce` combining them with $times.o$, then sequential fold $eq.triple$ parallel reduction.

- *How to apply* (procedure)
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

- *Properties*
  - *Non-aliasing / disjointness*: two pointers/regions don't refer to overlapping memory.
  - *Effect independence*: instruction `A` does not read/write anything that `B` writes/reads in a conflicting way.
  - *Commutativity / associativity* (when reordering arithmetic or reductions): reordering doesn't change value (or changes only within accepted FP error bounds, if that's a permitted spec).

- *Theorem shapes*
  - *Commutation theorem*: If $A$ and $B$ are independent, then $A; B eq.triple B; A$.
  - *Code motion theorem*: If moving `A` across a region does not violate dependencies, then hoist/sink is semantics-preserving.

- *How to apply*
  1. Compute dependency edges (RAW/WAR/WAW + control dependencies).
  2. Use alias facts and effect summaries to remove false dependencies.
  3. Apply theorems to swap or move operations:
    - swap adjacent independent ops repeatedly, or
    - reorder within a basic block/topological schedule.
  4. Re-check: no dependency cycles introduced; observable behavior (I/O, volatile, atomics) unchanged.

=== Loop unrolling, fusion, and fission

- *Properties*
  - For *unrolling*:
    1. No loop-carried dependency that spans more than the unroll factor in a way that breaks correctness.
    2. Bounds and guards are handled (remainder loop correctness).
  - For *fusion*:
    1. Two loops have compatible iteration spaces.
    2. No harmful dependence from the second loop to the first that requires separation (or it can be ordered within a fused body).
    3. Benefit property: improved locality/reuse.
  - For *fission*:
    1. The loop body can be split into phases with no cross-phase carried dependence (or a proven-safe pipeline).

- *Theorem shapes*
  - *Unroll theorem*: `for i: A(i)` $eq.triple$ `for i step u: A(i); ...; A(i+u-1)` + remainder, under dependence/bounds conditions.
  - *Fusion theorem*: `for i: A(i); for i: B(i)` $eq.triple$ `for i: A(i); B(i)` if dependencies allow.
  - *Fission theorem*: `for i: A(i); B(i)` $eq.triple$ `for i: A(i); for i: B(i)` if `A` and `B` are separable.

- *How to apply*
  1. Build loop summary: iteration domain, per-iteration `RW`, and loop-carried deps.
  2. Pick transform:
    - Unroll: choose factor `u`, generate unrolled body + cleanup loop.
    - Fuse: align bounds/strides, merge bodies, then schedule within fused body respecting deps.
    - Fission: split body into groups of statements, then emit multiple loops.
  3. Re-check invariants: induction variables, exit conditions, and preserved ordering for side-effecting ops.
  4. Benchmark-guided: apply only when summaries predict wins (branch overhead, cache reuse, vectorization).

  === Brute-force search parallelization

- *Properties*
  - *Search space partition*: candidate set `C` can be split into `C0..Ck-1`.
  - *Pure evaluation*: checking a candidate is deterministic and has no shared side effects (or effects are isolated).
  - *Aggregation correctness*: “best”/“exists”/“count” operator is associative/commutative or otherwise order-independent (or you preserve a defined order).

- *Theorem shapes*
  - *Parallel enumeration theorem*: `fold(op, map(eval, C))` $eq.triple$ `reduce(op, [fold(op, map(eval, Ci))])`.
  - *Early-exit theorem* (for existential queries): if `op` is short-circuiting (e.g., OR), you can stop when a witness is found, provided cancellation doesn't affect semantics.

- *How to apply*
  1. Define candidate generator and evaluation function.
  2. Choose partitioning strategy (range split, hashing, domain decomposition).
  3. Compute per-worker partial result.
  4. Combine partial results with proven-correct aggregator.
  5. Ensure reproducibility if required (deterministic tie-breaking for “best”).

=== Greedy algorithms (as theorem-driven approximations)

- *Properties*
  - *Local choice validity*: a local decision never produces an invalid state (invariants preserved).
  - *Progress / termination*: each step decreases a measure.
  - Optionally: *approximation guarantee* under stated assumptions (matroid property, submodularity, etc.).

- *Theorem shapes*
  - *Invariant-preserving step theorem*: If invariant holds before step and the greedy choice satisfies condition `G`, invariant holds after.
  - *Approximation theorem* (optional): Greedy result is within factor $alpha$ of optimal given property `P`.
  - *Proof of bound*: For a certain class of problems, greedy always yields an optimal solution. This class of problems can be characterized by specific properties (e.g., matroids). Prooving a problem is a matroid can help justify the optimality of greedy solutions.

- *How to apply*
  1. Identify the invariant that defines “valid partial solution”.
  2. Prove greedy step preserves it.
  3. Prove termination/progress.
  4. If you need guarantees, state and prove the structural assumption (e.g., submodular objective) and then apply the approximation theorem.

=== Dynamic programming optimizations

- *Properties*
  - *Optimal substructure*: optimal solution for a state depends on optimal solutions of smaller states.
  - *Overlapping subproblems*: many calls repeat the same state.
  - *State definition*: a finite key that uniquely represents a subproblem.

- *Theorem shapes*
  - *Memoization theorem*: replacing repeated evaluation of pure function `f(s)` with a cache keyed by `s` preserves semantics.
  - *Tabulation theorem*: recursive definition ≡ iterative fill in a topological order over the dependency DAG of states.

- *How to apply*
  1. Extract the recurrence and identify the state variables.
  2. Prove purity/determinism of the recurrence (or isolate side effects).
  3. Choose memoization (top-down) or tabulation (bottom-up):
    - Memoization: add cache, keep recursion.
    - Tabulation: compute dependency order, allocate table, fill iteratively.
  4. Validate:
    - Base cases preserved.
    - Table order respects dependencies.
    - Memory/time bounds match expectations.
