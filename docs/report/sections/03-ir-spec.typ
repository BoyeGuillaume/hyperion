= Hyperion IR <ir-section>

This section specifies the *Hyperion IR* as a user-facing language. The goal is to define syntax and semantics precisely enough that (1) derived theorems can rely on a stable model and (2) equivalence-preserving rewrites can be stated as transformations over IR fragments.

The IR is inspired by LLVM IR#footnote[
  LLVM IR is described in detail in the LLVM Language Reference Manual: #link("https://llvm.org/docs/LangRef.html")[https://llvm.org/docs/LangRef.html].
], but diverges where Hyperion needs explicit semantics (notably integer overflow) and a proof layer (meta-level artifacts).

== Conceptual model

Hyperion IR is a typed, block-structured, control-flow graph (CFG) representation organized as *modules* containing *functions*. A function consists of basic blocks; each block contains a sequence of instructions followed by a terminator.

Key properties:

- *Static typing*: every value has a type (integers, floats, pointers, structured aggregates).
- *Explicit control flow*: control flow is represented with explicit terminators; edges are not implicit.
- *Semantics-first design*: operations are split not only by “what they compute” but also by “how they compute it” (e.g., explicit overflow modes).

== Syntax (surface form)

This document uses a textual syntax close to the parser syntax used throughout the repository.

A module is a sequence of function declarations/definitions:

```llvm
; Module defined with following external/internal functions
declare <ret-ty> <name>(<param-tys>*)
define  <ret-ty> <name>(<params>*) { <blocks>* }
```

A function definition contains labeled blocks:

```llvm
define <ret-ty> <name>(<params>*) {
<label>:
  <instructions>*
  <terminator>
}
```

Instruction forms are written as either producing a value or as a statement:

```llvm
%v: <ty> = <opcode> <operands>*
<opcode> <operands>*
```

Literals are written using an explicit type annotation when ambiguous:

```llvm
i32 0
fp32 1.0
```

Labels identify blocks, and functions are referenced by name:

```llvm
jump exit
%r: i32 = call pow_i32, %a, %e
```

== Structuring IR programs

A `Module` is a compilation unit containing an aggregate of `Function` definitions and declarations. External functions are introduced via `declare`.

A `Function` has:
- a name
- a list of parameters
- a return type
- an ordered set of basic blocks

A *basic block* is a straight-line sequence of instructions that ends with a terminator. The first block is the entry block.

== Type system

Hyperion IR is statically typed; types are part of the syntax and are required for verification and theorem derivation.

=== Primary types

- *Integers*: `iN` where `N` is the bit-width (e.g., `i1`, `i8`, `i32`, `i64`, `i565` if you want a 565-bit integer).
  - `i1` is used as the boolean type by convention.
- *Floating-point*: `bf16`, `fp16`, `fp32`, `fp64`, `fp128`, `x86_fp80`, `ppc_fp128` following IEEE semantics
- *Pointers*: `ptr` type representing memory addresses. Pointers are not typed by the pointee type and as such are considered first-class values.
- *Vectors*: `<N x T>` where `N` is the number of lanes and `T` is a primary type.
  - Vectors are fixed-size SIMD-like aggregates.
  - Dynamic-sized vectors exist on some targets written as `<vscale N x T>`, where `N` must be a compile-time multiple of the runtime vector length.

=== Aggregate types

- *Tuples / structs*: `{ T0, T1, ... }` representing fixed-size aggregates of heterogeneous types.
- *Arrays*: `[N x T]` representing fixed-size aggregates of homogeneous types with `N` elements of type `T`.

== Instruction set overview

The Hyperion IR instruction set is organized by operation class. A subset is implemented today; the categorization below is the stable user-facing model. #footnote[
  Meta-level proof artifacts are carried by *meta-functions* and *meta-instructions*; see the dedicated @ir-spec-meta-functions.
]

=== Integer operations

Integer arithmetic is bit-precise and requires explicit overflow semantics.

*Overflow modes* are specified as suffixes: wrapping (`*.wrap`), saturating (`*.usat` / `*.ssat`), trapping (`*.utrap` / `*.strap`).

*Signedness* can be either `signed` or `unsigned` and is specified for operations where it matters (division, remainder).

#set table(
  columns: (1fr, 0.5fr, 2fr),
  inset: 6pt,
)

#table(
  table.header(
    [*Operation*],
    [*Produced*],
    [*Meaning / notes*],
  ),

  [`iadd.<overflow>`],
  [`iN`],
  [Integer addition with explicit overflow semantics.],
  [`isub.<overflow>`],
  [`iN`],
  [Integer subtraction with explicit overflow semantics.],
  [`imul.<overflow>`],
  [`iN` ],
  [Integer multiplication with explicit overflow semantics.],
  [`ineg.<overflow>`],
  [`iN`],
  [Integer negation with explicit overflow semantics.],
  [`idiv.<signedness>`],
  [`iN`],
  [Integer division; interpretation depends on `signed` vs `unsigned`.],
  [`irem.<signedness>`],
  [`iN`],
  [Integer remainder; interpretation depends on `signed` vs `unsigned`.],

  [`icmp.<icmp-cond>`],
  [`i1`],
  [Integer comparison producing `i1`.],

  [`icmp.eq`],
  [`i1`],
  [Equal (==).],
  [`icmp.ne`],
  [`i1`],
  [Not equal (!=).],
  [`icmp.slt`],
  [`i1`],
  [Signed less-than (<).],
  [`icmp.sle`],
  [`i1`],
  [Signed less-or-equal (<=).],
  [`icmp.sgt`],
  [`i1`],
  [Signed greater-than (>).],
  [`icmp.sge`],
  [`i1`],
  [Signed greater-or-equal (>=).],
  [`icmp.ult`],
  [`i1`],
  [Unsigned less-than (<).],
  [`icmp.ule`],
  [`i1`],
  [Unsigned less-or-equal (<=).],
  [`icmp.ugt`],
  [`i1`],
  [Unsigned greater-than (>).],
  [`icmp.uge`],
  [`i1`],
  [Unsigned greater-or-equal (>=).],

  [`iand`],
  [`iN`],
  [Bitwise AND.],
  [`ior`],
  [`iN`],
  [Bitwise OR.],
  [`ixor`],
  [`iN`],
  [Bitwise XOR.],
  [`inot`],
  [`iN`],
  [Bitwise NOT.],
  [`isht.<dir>`],
  [`iN`],
  [Shift/rotate where `<dir>` is `lsl`, `lsr`, `asr`, `rol`, `ror`.],

  [`iimplies`],
  [`iN`],
  [Bitwise implication (per bit).],
  [`iequiv`],
  [`iN`],
  [Bitwise equivalence (per bit).],
)

=== Floating-point operations

Floating-point operations follow IEEE-like semantics (rounding, NaNs, signed zeros are observable) unless additional, explicit facts justify relaxation.

#table(
  columns: (1fr, 0.5fr, 2fr),
  table.header([*Operation*], [*Type*], [*Meaning / notes*]),

  [`fadd`], [`fp*`], [Floating-point addition.],
  [`fsub`], [`fp*`], [Floating-point subtraction.],
  [`fmul`], [`fp*`], [Floating-point multiplication.],
  [`fdiv`], [`fp*`], [Floating-point division.],
  [`fneg`], [`fp*`], [Floating-point negation.],
  [`fcmp.<fcmp-cond>`], [`fp*`], [Floating-point comparison producing `i1` (IEEE caveats around NaNs apply).],

  [`fcmp.oeq`], [`i1`], [Ordered equal (no NaNs, ==).],
  [`fcmp.one`], [`i1`], [Ordered not equal (no NaNs, !=).],
  [`fcmp.olt`], [`i1`], [Ordered less-than (no NaNs, <).],
  [`fcmp.ole`], [`i1`], [Ordered less-or-equal (no NaNs, <=).],
  [`fcmp.ogt`], [`i1`], [Ordered greater-than (no NaNs, >).],
  [`fcmp.oge`], [`i1`], [Ordered greater-or-equal (no NaNs, >=).],
  [`fcmp.ord`], [`i1`], [Ordered (true iff neither operand is NaN).],

  [`fcmp.ueq`], [`i1`], [Unordered equal (true if NaN present or ==).],
  [`fcmp.une`], [`i1`], [Unordered not equal (true if NaN present or !=).],
  [`fcmp.ult`], [`i1`], [Unordered less-than (true if NaN present or <).],
  [`fcmp.ule`], [`i1`], [Unordered less-or-equal (true if NaN present or <=).],
  [`fcmp.ugt`], [`i1`], [Unordered greater-than (true if NaN present or >).],
  [`fcmp.uge`], [`i1`], [Unordered greater-or-equal (true if NaN present or >=).],
  [`fcmp.uno`], [`i1`], [Unordered (true iff either operand is NaN).],
)

=== Conversions and casts

The current IR parser accepts the full set of cast opcodes syntactically, but it does not yet enforce type-compatibility rules for casts during parsing (e.g., it does not reject an ill-typed cast). Type-checking/verification of casts is expected to be part of later IR verification passes.

#table(
  columns: (1fr, 0.5fr, 2fr),
  table.header([*Operation*], [*Type*], [*Meaning / notes*]),

  [`cast.trunc`], [`iN`], [Integer $arrow$ integer, narrowing conversion.],
  [`cast.zext`], [`iN`], [Integer $arrow$ integer, zero-extension.],
  [`cast.sext`], [`iN`], [Integer $arrow$ integer, sign-extension.],

  [`cast.fptrunc`], [`fp*`], [Float $arrow$ float, narrowing conversion.],
  [`cast.fpext`], [`fp*`], [Float $arrow$ float, widening conversion.],

  [`cast.fptoui`], [`iN`], [Float $arrow$ unsigned integer (round toward zero).],
  [`cast.fptosi`], [`iN`], [Float $arrow$ signed integer (round toward zero).],
  [`cast.uitofp`], [`fp*`], [Unsigned integer $arrow$ float.],
  [`cast.sitofp`], [`fp*`], [Signed integer $arrow$ float.],

  [`cast.ptrtoint`], [`iN`], [Pointer $arrow$ integer (size-adjust via zero-extend/truncation).],
  [`cast.inttoptr`], [`ptr`], [Integer $arrow$ pointer (size-adjust via zero-extend/truncation).],

  [`cast.bitcast`], [any primary], [Bitcast between same-size types without changing the bit representation.],
)

=== Aggregate and structural operations

#table(
  columns: (1fr, 0.5fr, 2fr),
  table.header([*Operation*], [*Type*], [*Meaning / notes*]),

  [`insertvalue <agg>, <idx>, <val>`], [array/struct], [Produce a new aggregate with one field updated.],
  [`extractvalue <agg>, <idx>`], [array/struct], [Project a field out of an aggregate.],
)

=== Miscellaneous instructions

#table(
  columns: (1fr, 0.7fr, 2fr),
  table.header([*Instruction*], [*Type*], [*Meaning / notes*]),

  [`select <cond>, <true>, <false>`],
  [any],
  [Select between two values based on an `i1` condition (pure value instruction).],

  [`phi [<v0>, <pred0>], [<v1>, <pred1>], ...`],
  [any],
  [Merge values coming from predecessor blocks; each incoming value is paired with its predecessor label.],

  [`invoke <fn>, <args...>`],
  [any or void],
  [Function call. In textual form it is printed as `invoke` (optionally with a calling convention for externals).],
)

=== Memory operations

Memory operations are the only instructions that can observe or update memory state.

#table(
  columns: (1fr, 1fr, 2fr),
  table.header([*Operation*], [*Type*], [*Meaning / notes*]),

  [`alloca <num>`], [`<ty>`], [Allocate stack storage (lifetime scoped to the function).],
  [`getelementptr <ty>, <ptr>, <idx...>`], [`ptr`], [Compute a derived address.],
  [`load <ptr>`], [any], [Read from memory.],
  [`store <ptr>, <val>`], [N/A], [Write to memory.],
)

Reordering/elimination of memory operations requires proven aliasing and ordering side-conditions.

== Control flow constructs

Control flow is represented by terminator instructions.

=== `ret`

`ret <val>` returns from the current function with a value. `ret void` is used for `void` functions.

=== `jump`

`jump <label>` transfers control unconditionally to a target block.

=== `branch`

`branch <cond>, <then>, <else>` transfers control based on a boolean (`i1`) condition.

=== `trap`

`trap` indicates an unrecoverable error; execution does not continue past this point.

== Meta-functions and meta-instructions <ir-spec-meta-functions>

Hyperion distinguishes between *computational* functions (intended to execute) and *meta-functions* (intended to state or carry proof/analysis artifacts).

A meta-function is a function-like container whose body may contain meta-instructions such as assertions, assumptions, quantifiers, and analysis markers. These instructions enrich the proof/analysis context but must not change the observable runtime behavior of computational functions.

Supported meta-instructions (current):

#table(
  columns: (1fr, 0.7fr, 2fr),
  table.header([*Instruction*], [*Type*], [*Meaning / notes*]),

  [`!assert <cond>`], [N/A], [Assert that `<cond>` holds in the current proof context.],
  [`!assume <cond>`], [N/A], [Assume `<cond>` as a hypothesis for subsequent reasoning.],
  [`!isdef <x>`], [`i1`], [Predicate that the value/operand `<x>` is defined (not `undef`).],
  [`!forall`], [any], [Introduce a universal-quantification marker for meta-level reasoning.],
  [`!prob.<variant> <op...>`], [`fp*`], [Probabilistic meta query (`.prb`, `.xpt`, `.var`).],
  [`!prob.prb op`], [`fp*`], [Probability of event `op` (of type `i1`).],
  [`!prob.xpt op`], [`fp*`], [Expected value of random variable `op` (of type `iN` or `fp*`).],
  [`!prob.var op`], [`fp*`], [Variance of random variable `op` (of type `iN` or `fp*`).],
  [`!analysis.<variant> <op...>`],
  [var],
  [Analysis-statistics / termination queries (e.g., `.excnt`, `.icnt`, `.term.*`).],
)

== Illustrative IR snippet

The following snippet illustrates (1) explicit overflow semantics and (2) common control-flow constructs. This function computes integer exponentiation by repeated multiplication:

#figure(
  ```llvm
  define i32 pow(%a: i32, %n: i32) {
  entry:
      %is_null: i1 = icmp.eq %n, i32 0
      branch %is_null, output, loop

  loop:
      %current_n: i32 = phi [%n, entry], [%next_n, loop]
      %current_res: i32 = phi [i32 1, entry], [%next_res, loop]
      %next_n: i32 = isub.wrap %current_n, i32 1
      %next_res: i32 = imul.usat %current_res, %a
      %loop.is_null: i32 = icmp.eq %next_n, i32 0
      branch %loop.is_null, output, loop

  output:
      %output: i32 = phi [i32 1, entry], [%next_res, loop]
      ret %output
  }
  ```,
  caption: "Example Hyperion IR function implementing integer exponentiation with explicit overflow semantics.",
  kind: "code",
  supplement: [Code],
) <ir-kernel-example-pow>

#pagebreak()
Here is another example implementing square root using the Newton-Raphson method:
```llvm
define fp32 sqrt_newton(%x: fp32) {
entry:
    %half_x: fp32 = fmul %x, fp32 0.5
    %guess0: fp32 = fadd %half_x, fp32 1.0
    %reciprocal: fp32 = fdiv %x, %guess0
    %avg: fp32 = fadd %guess0, %reciprocal
    %guess1: fp32 = fmul %avg, fp32 0.5
    %reciprocal2: fp32 = fdiv %x, %guess1
    %avg2: fp32 = fadd %guess1, %reciprocal2
    %guess2: fp32 = fmul %avg2, fp32 0.5
    ret %guess2
}
```
