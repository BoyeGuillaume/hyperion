# Introduction

This document provides an introduction to the internal terminology and concepts used within the Hyperion framework. It aims to clarify the nomenclature and design decisions that underpin the system, facilitating a better understanding for developers and users alike.

## Program and instructions

Hyperion operates on a logical intermediate representation (IR) of programs that closely matches the [llvm IR](https://llvm.org/docs/LangRef.html). Notable features of this IR include:
 - **SSA Form**: Single Static Assignment form, where each variable is assigned exactly once, simplifying data flow analysis.
 - **Typed Instructions**: Each instruction in the IR is associated with a specific type, ensuring type safety during transformations and optimizations.
 - **Blocks and Control Flow**: The IR is structured into basic blocks, with explicit control flow between them, allowing for clear representation of program logic.

An instruction $\mathcal{I}$ in hyperion is represented as a tuple:
$$\mathcal{I} = (op, dst, src_1, src_2, ..., src_n)$$
where:
 - $op$ is the operation code (opcode) defining the operation to be performed.
 - $dst$ is the destination operand where the result of the operation is stored.
 - $src_1, src_2, ..., src_n$ are the source operands that provide the input values for the operation.

A terminator $\mathcal{T}$ is not an instruction but a special kind of operation that marks the end of a basic block and defines the control flow to subsequent blocks. Examples of terminators include branches, returns, and jumps.

A block $\mathcal{B}$ is a sequence of instructions followed by a terminator. It can be represented as:
$$\mathcal{B} = \{\mathcal{I}_1, \mathcal{I}_2, \cdots, \mathcal{I}_m, \mathcal{T}\}$$
where $\mathcal{I}_1, \mathcal{I}_2, ..., \mathcal{I}_m$ are the instructions in the block, and $\mathcal{T}$ is the terminator.

A function $\mathcal{F}$ is a collection of blocks that together define a complete unit of computation. It can be represented as:
$$\mathcal{F} = \{\mathcal{B}_1, \mathcal{B}_2, \cdots, \mathcal{B}_n\}$$
where $\mathcal{B}_1, \mathcal{B}_2, ..., \mathcal{B}_n$ are the blocks in the function.

A program $\mathcal{P}$ is a collection of functions, each consisting of multiple blocks. It can be represented as:
$$\mathcal{P} = \{\mathcal{F}_1, \mathcal{F}_2, \cdots, \mathcal{F}_k\}$$
where $\mathcal{F}_1, \mathcal{F}_2, ..., \mathcal{F}_k$ are the functions in the program.

## Function equivalence

Consider two functions $f, g \in \mathcal{F}$, and a program $\mathcal{P}$ such that
1. $f$ and $g$ have the same type signature, i.e., they accept the same number and types of arguments and return the same type.
2. $\mathcal{P}\{f \rightarrow g\}$ is observably equivalent to $\mathcal{P}$ in that for all states $s$, executing $\mathcal{P}$ from $s$ yields the same observable behavior as executing $\mathcal{P}\{f \rightarrow g\}$ from $s$.

Where $\mathcal{P}\{f \rightarrow g\}$ denotes the program $\mathcal{P}$ with all calls to function $f$ replaced by calls to function $g$.

If the above conditions hold, we say that functions $f$ and $g$ are *equivalent* in the context of program $\mathcal{P}$. We note this as $f \leftrightsquigarrow g$.

Similarly, we say that $f$ and $g$ are *equivalent under $\mathcal{C}$* for some set of preconditions $\mathcal{C}$ if $f$ and $g'$ are equivalent where $g'$ is defined as:
$$g'(x) = \begin{cases}
g(x) & \text{if } \mathcal{C}(x) \text{ holds} \\
f(x) & \text{otherwise}
\end{cases}$$
We note this as $f \leftrightsquigarrow_{\mathcal{C}} g$.

## Postconditions and sufficient equivalence postconditions

A *postcondition* is a logical assertion that describes the expected state of the program after the execution of a function. Formally a postcondition $P$ for a function $f$ and a precondition $C$ is a predicate over the program state $s$ such that 
$$
\forall s. C(s) \implies P(\Gamma_f(s))
$$
where $\Gamma_f(s)$ denotes the state of the program after executing function $f$ from state $s$.

A set of postconditions $\{P_1, P_2, \ldots, P_n\}$ is said to be *sufficient for equivalence* under a precondition $C$ if
$$
\begin{aligned}
    \forall s.&\; C(s) \land (P_1(\Gamma_f(s)) \land P_2(\Gamma_f(s)) \land \ldots \land P_n(\Gamma_f(s))) \\
    &\land (P_1(\Gamma_g(s)) \land P_2(\Gamma_g(s)) \land \ldots \land P_n(\Gamma_g(s))) \\
    \implies& f \leftrightsquigarrow_{\{C\}} g
\end{aligned}
$$

## A note on proof and axiomatic reasoning.

In the *hyperion* framework, we write all proof as program that check the validity of certain conditions. For instance, if we want to make
an argument about a function $f = \{ \mathcal{B}_1, \mathcal{B}_2, \ldots, \mathcal{B}_n \}$, we write a series of *meta-instructions* that 
add assertions about `f`'s behavior at various points in its execution.

- We introduce an `Assert` meta-instruction which is a `no-op` at runtime, but is used to ensure that a `i1` value is **ALWAYS** true. We then allow
to add new condition and checks at different points. For instance loop-invariants can be seen as a condition `assert %cond` in the body of a loop.
- We also introduce the notion of `free-variable` for reasoning about preconditions/postconditions. For instance, a precondition that check a list is sorted
```ll
; Defined above %list_ptr, %n: i32

%i = free_variable i32
%ii = add i32 %i, 1
%cond = icmp slt i32 %ii, %n
assume %cond
%index = getelementptr i32, i32* %list_ptr, i32 %i
%val1 = load i32, i32* %index ; Only load if %cond is true
%val2 = load i32, i32* %index ; Only load if %cond is true
%is_sorted = icmp sle i32 %val1, %val2
assert i1 %is_sorted
``` 

The difference between `assume` and `assert` is that `assume` tells the proof engine to only consider paths where the condition is true. It should in theory not be used
directly when doing axiom derivation unless for free-variables. Can be used to "hide" complex proofs. If $A$ is true due to proof $P$, then `assume A` can be used for compression.

## A note on genericity and abstractions.

When optimizing code and proving equivalence, it is often useful to `abstract` away other functions. For instance, consider a hashmap implementation that uses
a hash function `hash_func`. When reasoning about the hashmap operations, we may not care about the actual implementation of `hash_func`, but only about its
properties. 
The property of a hash function is VERY hard to prove. Furthermore, in many cases it is possible to construct hash collisions.
