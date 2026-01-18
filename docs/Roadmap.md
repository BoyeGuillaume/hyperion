# Roadmap

## Immediate Next Steps

- [x] Extend the specification language to support `meta-instructions` for assumptions and assertions.
- [x] Implement parsing and serialization of specifications to/from a human-readable format (e.g., JSON or YAML).
- [x] Add meta-instructions for complexity analysis (probabilistic time and space complexity).
- [ ] Rework complexity analysis to support multi-function and complex complexity (like `O(n) call to f` inside a loop).
- [ ] Implement instruction for insertion/extraction of (1) structured data and (2) arrays and vectors.
- [ ] Implement the `meta-behavior` instruction family to check whether a function call
  terminates, crashes, or loops based on the `HaltingBehavior` specification.

## Mid-Term Goals

- [ ] Develop derivers for simple specifications (find loop invariants, preconditions, postconditions).
- [ ] Implement a verification engine that can check function equivalence based on provided specifications.
- [ ] Implement searching of target conditions for equivalence using SMT solvers.

## Design Decisions: ProofView and TerminationScope

- Construct: ProofView (aka TheoremDerivationView) overlays an original function, keeps an explicit reference to it, and adds reasoning without mutating the source.
- Reasoning model:
  - Use existing IR ops (e.g., iadd, icmp) in side-effect-free PreBlock and PostBlocks.
  - Employ !assume and !assert to express preconditions, invariants, and postconditions.
  - Order is governed by SSA dependencies; values must be defined before use.
- Termination analysis:
  - Introduce TerminationScope for MetaAnalysisStat::TerminationBehavior:
    - BlockExit: termination of the current block.
    - FunctionExit: termination of the entire function.
    - ReachPoint(label): termination defined as reaching a specific label.
    - ReachAny(labels): termination if any label in the set is reached.
    - ReachAll(labels): termination if all labels in the set are reached.
- Quantified reasoning:
  - Encode ∀ by treating ProofView PreBlock parameters as bound variables; constrain with !assume in PreBlock and conclude with !assert in PostBlocks.
