#import "@preview/diatypst:0.8.0": *
#import "@preview/thmbox:0.3.0": *
#import "@preview/rustycure:0.2.0": qr-code
#import "@preview/diagraph:0.3.6": raw-render

#show: thmbox-init()
#show: slides.with(
  title: "Hyperion: Technical Overview",
  subtitle: "Presentation of hyperion core",
  date: {
    datetime.today().display("[day] [month repr:long] [year repr:full]")
  },
  authors: "Guillaume Boyé",

  // Optional styling
  ratio: 16 / 9,
  layout: "medium",
  title-color: blue.darken(60%),
  theme: "full",
  toc: true,
  count: "number",
)
#let todo = thmbox.with(
  color: colors.dark-red,
  variant: translations.variant("Todo"),
  numbering: none,
  sans: false,
  fill: colors.dark-red.lighten(90%),
)

= Formalism

== Instruction set

#v(5mm)

Much like compilers, hyperion at its core relies on an *intermediate representation* of programs, called the hyperion IR. Taking inspiration from _LLVM IR_#footnote[https://llvm.org/docs/LangRef.html], hyperion IR is a low-level typed assembly-like language that is platform-agnostic.

#v(5mm)

It's key features are:
- *SSA* (Static Single Assignment) form, meaning each variable is assigned exactly once.
- *Typed* instructions, meaning each instruction has a well-defined type.
- Seamless support for *SIMD* operations, allowing vectorized computations.
- Phi-nodes for control flow merging.

As mention, it is design for *intercompatibility* with _LLVM_ to ease compilation to native code.

== Instruction set -- Example

Here is a simple example of a hyperion IR function that returns the integer `1`:

```rust
let a_block = BasicBlock {
  label: Label::NIL, // entry block
  instructions: vec![todo!()],
  terminator: Terminator::Return {
    value: Some(Operand::Imm(1u32.into())),
  }.into(),
}

let function = Function {
  parameters: vec![],
  return_type: Type::U32,
  basic_blocks: vec![a_block],
  /// Other fields...
};
```

== Instruction set -- Formalism

We want to define a formal semantics of functions mathematically.

#definition(title: "Proposal 1")[
  A function $f in bb(F)$ is a mapping from arguments $A_1,..., A_n$ to $O$.

  Where:
  - $A_i$ are the argument types.
  - $O$ is the output type.
]

*Limitation*:
- *Memory* is limited requiring our model to account for memory state. This model cannot account for side-effects.

#pagebreak()

We want to define a formal semantics of functions mathematically.

#definition(title: "Proposal 2")[
  A function $f in bb(F)$ is defined as a mapping $f in (A_1, ..., A_n, Gamma) -> O times Gamma$

  Where:
  - $A_i$ are the argument types.
  - $O$ is the output type.
  - $Gamma$ represents the memory state.
]

*Limitation*:
- Does not account for non-terminating/crashing functions.
- Does not account for concurrency.

#pagebreak()
We want to define a formal semantics of functions mathematically.

#definition(title: "Proposal 3")[
  A function $f in bb(F)$ is defined as a mapping $f in (A_1, ..., A_n, Gamma) -> (O union {bot, lozenge} times Gamma)$

  Where:
  - $A_i$ are the argument types.
  - $O$ is the output type.
  - $Gamma$ represents the memory state.
  - $bot$ represents non-termination.
  - $lozenge$ represents a crash.
]

*Limitation*:
- Does not account for *concurrency*.

#pagebreak()

We choose *Proposal 3* as our formal model for hyperion functions. Additionally, for any function $f$ we define the following~:

#definition(title: "Memory/function interaction")[
  - The *observed memory* of $f$ is defined as the set of memory locations read/written by $f$, during execution noted $delta \{f\}(a_1, ..., a_n, gamma)$.
  - The *modified memory* of $f$ is defined as the set of memory locations written by $f$, during execution noted $mu \{f\}(a_1, ..., a_n, gamma)$.
]

#note[
  We have that for any function $f$ and any arguments $a_1, ..., a_n, gamma$~:
  $
    delta{f}(a_1, ..., a_n, gamma) supset mu{f}(a_1, ..., a_n, gamma)
  $
]

== Instruction set -- A note on concurrency
Concurrency is crutial to model due to the inherent limitation of _single-threaded models_. However in concurrency, state of memory is shared between potentially multiple functions $f$ and we need to consider the *resulting operations*.

#definition(title: "Non interfering")[
  We say that two functions $f$ and $g$ are *non-interfering* if for any inputs, the *observed memory* of $f$ does not intersect with the *modified memory* of $g$ and vice-versa~:
  $
    forall a_1, ..., a_n in A_1 times ... times A_n, b_1, ..., b_m in B_1 times ... times B_m forall gamma in Gamma. \
    (delta \{f\}(a_1, ..., a_n, gamma) inter mu \{g\}(b_1, ..., b_m, gamma) union \
    (delta \{g\}(b_1, ..., b_m, gamma) inter mu \{f\}(a_1, ..., a_n, gamma)) = emptyset
  $
]

#pagebreak()

#v(5mm)

In hyperion, we have that if two functions $f$ and $g$ are executed concurrently then one of the following holds~:
- $f$ and $g$ are non-interfering,
- We can define a *synchronization mechanism* to ensure that we can split $f$ and $g$ into a series of non-interfering or non-concurrent sub-functions.

  In this case, we need to optimize the entire program/system to ensure that the synchronization mechanism is respected.

== Analysis of a function

Given two functions $f, g in bb(F)$ we say that $f$ and $g$ are *equivalent* if for any inputs~:
$
  forall a_1, ..., a_n in A_1 times ... times A_n, gamma in Gamma. \
  f(a_1, ..., a_n, gamma) = g(a_1, ..., a_n, gamma) and \
  delta \{f\}(a_1, ..., a_n, gamma) = delta \{g\}(a_1, ..., a_n, gamma)
$

Furthermore, we say that $f$ is *strongly equivalent* to $g$ (noted $f arrow.l.r.wave g$) if for any program using $f$, replacing $f$ with $g$ results in an observable-equivalent program.

_Note_: Strong equivalence implies equivalence, but the converse is not necessarily true. *Strongly equivalent* provide guarantees about the order of memory operations remain identic between $f$ and $g$.

We say that $f$ and $g$ are *(strongly) equivalent under $C(a_1, ..., a_n, Gamma)$* (noted $f arrow.l.r.wave_C g$) iif $f$ and $f'$ are (strongly) equivalent where:
$
  f'(a_1, ..., a_n, gamma) := cases(
    g(a_1, ..., a_n, gamma) "if" C(a_1, ..., a_n, gamma),
    f(a_1, ..., a_n, gamma) "otherwise"
  )
$

#pagebreak()
We define the *optimization* problem as follow~:

Given a program $P$ using a set of functions $F = {f_1, ..., f_n}$, and
and oracle $cal(O)$ mapping $cal(O) : cal(P) times cal(A) times cal(Gamma) -> bb(R)$. Find a
set of functions $F' = {f'_1, ..., f'_n}$ such that~:
- $forall 1 <= i <= n.space f_i arrow.l.r.wave f'_i$
- $F' = "argmin"_F space EE_(a, gamma ~ cal(A) times Gamma) (cal(O)(P_F', a, gamma)$)

As such the *goal* of this framework is to explore the set of possible functions $F'$ that are (strongly) equivalent to $F$ and find the set that optimizes the oracle $cal(O)$.

= Optimization
== State of optimization -- Definitions
We define the *state of optimization* as follow~:
#definition(title: "CFG")[
  Given a function $f in bb(F)$, made of basic blocks $B = {b_1, ..., b_m}$, we define the *control flow graph* of $f$ as a directed graph $"CFG"(f) = (V, E)$ where~:
  - $V = b_i$
  - $E subset V times V$ where $(b_i, b_j) in E$ if there exists a possible execution path from $b_i$ to $b_j$.
]

#definition(title: "Reachability")[
  Given a function $f in bb(F)$, made of basic blocks $B = {b_1, ..., b_m}$, we say that a basic block $b_j$ is *reachable* from a basic block $b_i$ if there exists a path from $b_i$ to $b_j$ in the control flow graph $"CFG"(f)$.
]

// #pagebreak()
// We attach to each function a series of `assertions`. Those assertions can be understand as formal theorems about the function. Those
// assertions linked with a series of *meta-instructions* that do not contribute to the actual execution of the function.

// _Meta-instructions_ can be understand as *hints* given to the optimizer about potential transformations that can be applied to the function.
// Those assertions are always either
// 1. Attached at the *beginning* of the function, in which case they are called *pre-conditions*, unlike standard assertions they provide conditions
//   for which the proof is valid.
// 2. Attached at the *end* of the function, in which case they are called *post-conditions*, unlike standard assertions they provide guarantees about the function output.
// 3. Internal to the function, in which case they are always attached to a certain *block* and are called *invariants*, those provide guarantees
//   about the state of the function at a certain point.

== State of optimization -- A Simple Example
Suppose we have the following function

#text(size: 0.8em)[
  ```ll
  define i32 @pow(i32 %base, i32 %exp) {
  entry:
    %is_zero = icmp eq i32 %exp, 0 ; Check if exponent is zero
    br i1 %is_zero, label %output, label %loop ; Branch based on exponent

  loop: ; Main loop for exponentiation
    %current = phi i32 [1, %entry], [%next, %loop]
    %current_exp = phi i32 [%exp, %entry], [%next_exp, %loop]
    %next_exp = sub i32 %current_exp, 1 ; Decrement exponent
    %next = mul i32 %current, %base ; Multiply current result by base
    %is_done = icmp eq i32 %next_exp, 0 ; Check if exponentiation is done
    br i1 %is_done, label %output, label %loop ; Branch based on completion

  output:
    %result = phi i32 [1, %entry], [%next, %loop] ; Final result
    ret i32 %result ; Return the result
  }
  ```
]

#pagebreak()
We perform the following analysis:

We perform a *simple-disjunction* to determine based on the follow CFG~:
#align(center, {
  raw-render(
    ```dot
    digraph {
      node [shape=circle, style=filled, fillcolor=lightgrey];
      rankdir="LR";

      entry -> loop;
      loop -> loop;
      loop -> output;
      entry -> output;
    }
    ```,
  )
})

We split the function into two possible paths~:
- Path 1: `entry -> output` when `exp == 0`
- Path 2: `entry -> loop* -> output` when `exp != 0`
This mean we add two entries with two sets of precondition. *We merge them later*.

#pagebreak()
Path 1 is trivial, as such we will only analyse path 2.

*Step 1*: Determine reachability of condition `is_done`:
- Reachability same as `%next_expr == 0`
- Relise that `%next_expr` is a `loop covariant` and depend on `%exp`
- Simplify it (only consider element directly dependent and relize that `%next_expr` is decremented by `1` each loop)
- Arithmetic progression, we relize that `is_done` is reachable when `%exp >= 1`
Conclusion of *step 1*:
1. $1 <= %"current_exp" <= %"exp"$
2. `is_done` is *always* reachable when `%exp >= 1`
3. Number of loop iteration is exactly `%exp`
4. At loop exit, we have that `%next_exp == 0`

#pagebreak()
*Step 2*: Determine value of `%current` at loop exit:
- We relize that `%current` is multiplied by `%base` each loop iteration
- We have that `%current` starts at `1` and is multiplied by `%base` exactly `%exp` times
Conclusion of *step 2*:
1. At loop exit, we have that `%current == %base * %base * ... * %base` (`%exp` times)

#todo[
  Currently we cannot express the conclusion as a post-condition directly as we don't have a `pow` operator in the framework.
]

== State of optimization -- Determinism

What could be the post-condition for non-deterministic functions ?
```ll
declare i32 @nondet random();
```
1. We have that `random` is `non-deterministic`
2. Condition: every possibility is reached after finite times
```
random() == 0
```

#todo[
  Need to formalize non-determinism in the framework (with probability ?)
]

== State of optimization -- Memory side-effects
What could be the post-condition for memory-side effect functions ?
Suppose a simple `reduce_add` function that adds all elements of an array
into a single integer, array being defined as `type { i32, i32, ptr }`. Notice that we
use wildcards to represent the input/output elements.
```ll
define T @callback(T, T);
define T @reduce(ptr %array)
```
1. We have that `reduce` reads from memory location defined by `%array`
2. Condition: Does not crash if no memory location at `%array..%array+12` and
  at `*(%array + 8)..*(%array + 8 + 4 * ( *(%array + 4) - 1))` are valid
2.a. Condition of concurrency: No other function modifies those memory locations during execution
3. Post-condition: returns the sum of all elements in the array

== State of optimization -- HashMap
What could be the post-condition for complex data-structure manipulation functions ?
```ll
@requiquesite type T, U;
@requiquesite define i64 h(T key) satisfy "hash";
define U @hashmap_get(ptr %hashmap, T key);
define void @hashmap_set(ptr %hashmap, T key, U value);
```

1. Condition on hash. Probability of collision is low
$
  forall k_1, k_2 in T. space k_1 != k_2 => P(h(k_1) = h(k_2)) <= 1/(2^64) + epsilon
$

2. Condition on ordering and construction.
#todo[
  Find a way to construct ordering and constraint between multiple function.
]

== State of optimization
As such, we have seen what constraints and conditions we should expect for a good axiomatic framework. Namely~:
1. Should allow for _intermediary assertions_ and *behavior-characterization* of functions. It should also allow loop time analysis.
2. Should allow precondition to capture complex condition for flow disjunction.
3. Should allow for post-condition to capture complex behavior of many-functions.
4. Should account for *non-determinism* and *probabilistic algorithms*.
5. Should account for *memory side-effects* and *concurrency* by providing ordering constraints.

#pagebreak()
Here is a list of elements to consider

#note[
  - Building of _concepts_ which are a group of *preconditions* on potentially generic function.
    *Examples*: `commutative<T, op>`, `hash<T, h>`, `container<C, T, get, set, size>`, etc.
]

#proposition(title: "Non-Determinism")[
  We define the probability function $P : Omega -> [0, 1]$ where $Omega$ is the set of all possible outcomes of a non-deterministic function. By definition, $P$ always return in type $RR$.

  For instance we can model the quality of an hash function $h$ as follows~:
  $forall k_1, k_2 in T. space k_1 != k_2 => P(h(k_1) = h(k_2)) <= (1 + epsilon) dot 1/(2^64 - 1)$
]



= Conclusion
== Thanks for your attention!
#v(5mm)
Thank you for your attention! Any questions?

#align(center, [
  #qr-code(
    "https://github.com/BoyeGuillaume/hyperion",
    width: 60mm,
    height: 60mm,
    alt: "QR Code",
    fit: "contain",
  )

  #set text(size: 0.8em)
  Visit the Hyperion GitHub Repository at https://github.com/BoyeGuillaume/hyperion
])
