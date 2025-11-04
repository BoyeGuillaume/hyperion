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
