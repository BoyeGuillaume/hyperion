#import "../utils/cetz.typ": *

= Codebase overview

Hyperion's codebase is structured to promote a clear separation between the core IR language, the runtime/engine that consumes it, and the integration surfaces (bindings and examples) that exercise stable abstractions. This section provides an architectural overview of these components and their interactions.

== Architecture at a glance

#figure(
  cetz.canvas(
    {
      import cetz.draw: *
      // Public API
      blob([Public API], (-.5, -2), size: (4, 4), color: yellow.lighten(60%), text_anchor: "bottom-center")
      blob([Instance], (0, 0.5), size: (3, 1), color: yellow)
      blob([Module], (0, -1), size: (3, 1), color: yellow)

      // IR components
      blob([IR], (4, -6), size: (8, 8), color: red.lighten(60%), text_anchor: "bottom-center")
      blob([Module], (6.5, 0.8), size: (3, 1), color: red)
      blob([Function], (4.5, -1), size: (3, 1), color: red)
      blob([Global], (8.5, -1), size: (3, 1), color: red)
      blob([BasicBlock], (4.5, -3), size: (3, 1), color: red)
      blob([Instruction], (4.5, -5), size: (3, 1), color: red)
      blob([Terminator], (8.5, -5), size: (3, 1), color: red)

      // Optimizer components
      blob([Theorem Library], (13, 0.5), size: (4, 1), color: blue)
      blob([Theorem], (13.5, -1), size: (3, 1), color: blue)
      blob([Attached Function], (13, -3), size: (4, 1), color: blue)
      // blob([State], (13.5, -5), size: (3, 1), color: color.navy)

      arrow(((1.5, 0.5), (1.5, 0)), symbol: "<>")

      arrow(((8, 0.8), (8, 0.4), (6, 0.4), (6, 0)), symbol: "<>")
      arrow(((8, 0.8), (8, 0.4), (10, 0.4), (10, 0)), symbol: "<>")
      arrow(((6, -1), (6, -2)), symbol: "<>")
      arrow(((6, -3), (6, -4)), symbol: "<>")
      arrow(((6, -3), (6, -3.5), (10, -3.5), (10, -4)), symbol: "<>")
      arrow(((15, 0.5), (15, 0)), symbol: "<>")
      arrow(((15, -2), (15, -1.5), (6.5, -1.5), (6.5, -1)), symbol: ">")
      // arrow(((15, -3), (15, -4)), symbol: "<>")
    },
    padding: (5mm, 0),
  ),
  caption: "High-level architecture of Hyperion's codebase, showing the core IR components (red), public API (yellow), and optimizer components (blue).",
)


== API surface

To simplify the use of `Hyperion`, the framework exposes a facade API that hides internal complexity. This facade serves as the interface between user code and the underlying internal components. It enables (1) standardized interaction patterns, (2) easier maintenance and evolution of internal components while preserving stability and backward and forward compatibility, and (3) multi-language bindings, currently `Python` and `C`.

Current and planned facade API features include:
- A library `Instance` that owns configuration and extension state. This is the main entry point for users.
- An extension mechanism for plugins, enabling optional features to be registered and discovered.
- Compilation from the textual IR into an internal representation.
- Loading of compiled IR into a `Module` from file or memory.

Planned milestones extend this core loop into an execution and optimization platform:

- An `OptimizerPipeline` abstraction constructed from optimization passes.
- Explicit data and execution abstractions: `Buffer`/`BufferView`, `Device`, and higher-level `DeviceCluster` plus `Network` for inter-device communication.
- An `Executor` abstraction that executes functions on a device cluster with buffer-based I/O.
- A progression from low-level optimizations to “smart” pipelining and on-the-fly optimization.

== Internal architecture overview

At the core of Hyperion is a single *intermediate representation* (IR) that acts as the "shared language" between compilation, reasoning, optimization, and execution. Unlike conventional compiler IRs that only model programs, Hyperion's IR is intended to represent both the *program being executed* and the *proof artifacts* (proof obligations, derived lemmas, and theorems) about that program. In practice, this means the same structural vocabulary (modules, functions, basic blocks, and instructions) is used to describe executable computations and equivalence-preserving transformations. You can check @ir-section for a detailed specification of the IR.

Typical usage of Hyperion involves the following steps:

- *Represent*: ingest user code (via the facade API) and lower it to IR, producing a `Module` of `Function`s.
- *Extract*: analyze each function to identify semantic invariants and candidate rewrite opportunities, expressed as theorem statements over IR fragments.
- *Prove and catalog*: discharge proof obligations and store derived theorems as reusable transformation rules.
- *Synthesize*: when a theorem establishes that an alternative implementation is behaviorally equivalent (or equivalent under explicit preconditions), build a new function that realizes the proven transformation. When possible, these synthesized functions can expose additional structure, such as parallelism, vectorization opportunities, or improved asymptotic behavior.
- *Execute*: select an implementation (original or synthesized) and run it through the execution subsystem, targeting the available devices.

This architecture makes theorem derivation a first-class optimization mechanism: rather than relying only on local, pattern-based compiler passes, Hyperion seeks to *discover* semantic facts about functions and then use those facts to construct new implementations with the same observable behavior. In the long term, this is what enables the framework to move from "optimizing a given function" to "finding another function that behaves the same way, but is cheaper to run", and then executing that replacement.
