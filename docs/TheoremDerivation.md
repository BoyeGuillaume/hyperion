# Theorem derivation

This document outlines the strategy and steps taken to derive theorems in the context of our IR formal system.
Strategies are themselves categorizable into:

1. **Simplification Rules**: These rules are applied to reduce expressions to their simplest form.
2. **Theorem Library Application**: This involves applying known theorems and axioms from a predefined library to derive new results.

## Simplification Rules

This family of rules are used to (1) simplify, (2) normalize, and (3) combine expressions. They are applied repeatedly until no further simplifications can be made.

- **DNF/CNF Conversion**:
    Convert logical expressions to Disjunctive Normal Form (DNF) or Conjunctive Normal Form (CNF).

    *Example*: Rewrite expressions $A \land (B \lor C)$ to $(A \land B) \lor (A \land C)$.

    ```llvm
    %a: i1 = ...
    %b: i1 = ...
    %c: i1 = ...
    ---
    %e0: i1 = or %b, %c
    %e1: i1 = and %a, %e0
    =>
    %e2: i1 = and %a, %b
    %e3: i1 = and %a, %c
    %e4: i1 = or %e2, %e3
    ```

- **Remove tautologies**:
    Combine expressions that are always true or false.
    *Example*: $A \lor A$ simplifies to $A$. $A \land \neg A$ simplifies to false.

    ```llvm
    %a: i1 = ...
    ---
    %e0: i1 = or %a, %a
    =>
    %e1: i1 = i32 1 ; ToDO: enable constant to be specified as a normalized mode (like iadd i32 0, i32 1)
    ```

## Theorem Library Application

This family of strategies find theorems and axioms within a library and apply them to the current module to derive useful results.
Theorems present in the library consist of a set of premises and already derived conclusions.

- **Pattern Search**:
    Use tables to search for similar patterns in the current module that match the premises of known theorems.

- **Similar function search**: (**hard**)
    Find functions that are similar to the current function under analysis, or that are a subset/superset
    of the current function's behavior. This can help avoiding redundant code and leverage existing proofs.

## General pattern and strategy

Attempt to find general patterns and strategy through exploration

- **General loop patterns**: Use preset of known loop patterns to identify common structures.
  - Examples: counting loops, sentinel loops.
    1. Counting loops: loops that iterate a fixed number of times. Nesteds loops
       just treated as multiple counting loops.
    2. Sentinel loops: loops that continue until a specific condition is met.

- **Loop invariant**: Identify loop invariants that hold true before and after each iteration of a loop.
  - Check *covariants* within the loop, attempt find invariants conditions that hold true.

- **Inductive reasoning**: Use inductive reasoning to prove properties about loops and recursive functions.
  - Base case: Prove the property for the initial case (e.g., when the loop runs zero times).
  - Inductive step: Assume the property holds for n iterations, then prove it holds for n+1 iterations.

  To find inductive reasoning, we can use:
  - **Pattern identification**: Identify patterns in the loop structure that suggest inductive reasoning.
    Examples: if a loop modifies a variable in a consistent way each iteration.

- **Hypothesis/Proof generation**: Generate hypotheses about the behavior of loops and functions, then attempt to prove or disprove them using theorems and axioms from the library.
  - Use counterexamples to disprove hypotheses.
  - Use theorem application to prove hypotheses.
