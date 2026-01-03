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
