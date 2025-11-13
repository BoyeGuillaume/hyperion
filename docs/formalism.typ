#import "@preview/diatypst:0.8.0": *

#show: slides.with(
  title: "Hyperion: Technical Overview",
  subtitle: "Presentation of hyperion core",
  date: "13.11.2025",
  authors: "Guillaume Boyé",

  // Optional styling
  ratio: 16 / 9,
  layout: "medium",
  title-color: blue.darken(60%),
  theme: "full",
  toc: true,
  count: "number",
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

== Instruction set -- Example 1

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
#v(5mm)

We want to define a formal semantics of functions mathematically.

#v(5mm)

*Proposal 1*:
A function $f in bb(F)$ is a mapping from arguments $A_1,..., A_n$ to $O$.
Where:
- $A_i$ are the argument types.
- $O$ is the output type.

*Limitation*:
- *Memory* is limited requiring our model to account for memory state. This model cannot account for side-effects.

#pagebreak()
#v(5mm)

We want to define a formal semantics of functions mathematically.
#v(5mm)

*Proposal 2*:
A function $f in bb(F)$ is defined as a mapping $f in (A_1, ..., A_n, Gamma) -> O times Gamma$
Where:
- $A_i$ are the argument types.
- $O$ is the output type.
- $Gamma$ represents the memory state.

*Limitation*:
- Does not account for non-terminating/crashing functions.
- Does not account for concurrency.

#pagebreak()
#v(5mm)
We want to define a formal semantics of functions mathematically.
#v(5mm)

*Proposal 3*:
A function $f in bb(F)$ is defined as a mapping $f in (A_1, ..., A_n, Gamma) -> (O union {bot, lozenge} times Gamma)$
Where:
- $A_i$ are the argument types.
- $O$ is the output type.
- $Gamma$ represents the memory state.
- $bot$ represents non-termination.
- $lozenge$ represents a crash.

*Limitation*:
- Does not account for *concurrency*.

#pagebreak()
#v(5mm)

We choose *Proposal 3* as our formal model for hyperion functions. Additionally, for any function $f$ we define the following~:

#v(5mm)

- The *observed memory* of $f$ is defined as the set of memory locations read/written by $f$, during execution noted $delta \{f\}(a_1, ..., a_n, gamma)$.

- The *modified memory* of $f$ is defined as the set of memory locations written by $f$, during execution noted $mu \{f\}(a_1, ..., a_n, gamma)$.

#v(5mm)

Note: $delta{f}(...) supset mu{f}(...)$

== Instruction set -- A note on concurrency
#v(5mm)

Concurrency is crutial to model due to the inherent limitation of _single-threaded models_. However in concurrency, state of memory is shared between potentially multiple functions $f$ and we need to consider the *resulting operations*.

We say that two functions $f$ and $g$ are *non-interfering* if for any inputs, the *observed memory* of $f$ does not intersect with the *modified memory* of $g$ and vice-versa~:
$
  forall a_1, ..., a_n in A_1 times ... times A_n, b_1, ..., b_m in B_1 times ... times B_m. \
  (delta \{f\}(a_1, ..., a_n, gamma) inter mu \{g\}(b_1, ..., b_m, gamma) union \
  (delta \{g\}(b_1, ..., b_m, gamma) inter mu \{f\}(a_1, ..., a_n, gamma)) = emptyset
$

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

Furthermore, we say that $f$ is *strongly equivalent* to $g$ if for any program using $f$, replacing $f$ with $g$ results in an observable-equivalent program.

_Note_: Strong equivalence implies equivalence, but the converse is not necessarily true. *Strongly equivalent* provide guarantees about the order of memory operations remain identic between $f$ and $g$.

We say that $f$ and $g$ are *(strongly) equivalent under $C(a_1, ..., a_n, Gamma)$* iif $f$ and $f'$ are (strongly) equivalent where:
$
  f'(a_1, ..., a_n, gamma) := cases(
    g(a_1, ..., a_n, gamma) "if" C(a_1, ..., a_n, gamma),
    f(a_1, ..., a_n, gamma) "otherwise"
  )
$

#pagebreak()

#lorem(50)
