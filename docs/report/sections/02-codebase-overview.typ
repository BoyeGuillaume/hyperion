#import "@preview/cetz:0.4.2"

#let vc_add = (coordinate, delta) => (coordinate.at(0) + delta.at(0), coordinate.at(1) + delta.at(1))
#let vc_sub = (coordinate, delta) => (coordinate.at(0) - delta.at(0), coordinate.at(1) - delta.at(1))
#let vc_scale = (coordinate, factor) => (coordinate.at(0) * factor, coordinate.at(1) * factor)
#let vc_midpoint = (coord_a, coord_b, pos: 50%) => {
  let t = pos / 100%
  (
    coord_a.at(0) + t * (coord_b.at(0) - coord_a.at(0)),
    coord_a.at(1) + t * (coord_b.at(1) - coord_a.at(1)),
  )
}
#let vc_distance = (coord_a, coord_b) => {
  let dx = coord_b.at(0) - coord_a.at(0)
  let dy = coord_b.at(1) - coord_a.at(1)
  calc.sqrt(dx * dx + dy * dy)
}

#let blob = (_content, coord_a, size: (3, 1), color: red, padding: 1, text_padding: 0.3, text_anchor: "center") => {
  import cetz.draw: *

  let coord_a = vc_scale(coord_a, 1 + padding)
  let coord_b = vc_add(coord_a, size)

  let coord_c = vc_midpoint(coord_a, coord_b)

  if text_anchor.starts-with("top-") {
    coord_c = (coord_c.at(0), coord_b.at(1) - text_padding)
  } else if text_anchor.starts-with("bottom-") {
    coord_c = (coord_c.at(0), coord_a.at(1) + text_padding)
  }
  if text_anchor.ends-with("-left") {
    coord_c = (coord_a.at(0) + text_padding, coord_c.at(1))
  } else if text_anchor.ends-with("-right") {
    coord_c = (coord_b.at(0) - text_padding, coord_c.at(1))
  }

  rect(
    coord_a,
    coord_b,
    fill: color.lighten(60%),
    stroke: color.darken(30%) + 0.5mm,
    anchor: "center",
    radius: 2mm,
  )
  content(coord_c, text(fill: color.darken(60%), size: 1.1em, font: "Open Sans", _content))
}

#let arrow = (from, to, color: black, width: 0.5mm, side: 3, head-size: 0.1, padding: 1, horizontal: false) => {
  import cetz.draw: *
  let from = vc_scale(from, 1 + padding)
  let to = vc_scale(to, 1 + padding)

  let mid_a = (0, 0)
  let mid_b = (0, 0)
  if (horizontal) {
    mid_a = (to.at(0), from.at(1))
    mid_b = (from.at(0), to.at(1))
  } else {
    mid_a = (from.at(0), to.at(1))
    mid_b = (to.at(0), from.at(1))
  }

  let direction = vc_scale(vc_sub(to, mid_b), 1 / (vc_distance(mid_b, to) + 1e-3))
  let angle = calc.atan2(direction.at(0), direction.at(1))
  let to = vc_sub(to, vc_scale(direction, head-size))

  // Project mid on either the
  // line(from, mid_a, mid_b, to, stroke: blue + width)
  bezier(
    from,
    to,
    mid_a,
    mid_b,
    stroke: color + width,
  )
  polygon(
    to,
    side,
    angle: angle + 0deg,
    radius: head-size,
    fill: color,
  )
}

= Codebase overview

Hyperion's codebase is structured to promote a clear separation between the core IR language, the runtime/engine that consumes it, and the integration surfaces (bindings and examples) that exercise stable abstractions. This section provides an architectural overview of these components and their interactions.

== Architecture at a glance

#figure(
  cetz.canvas(
    {
      import cetz.draw: *
      // Public API
      blob([Public API], (1.8, -1.4), size: (3.8, 4.2), color: yellow.lighten(60%), text_anchor: "bottom-center")
      blob([Instance], (2, 0), size: (3, 1), color: yellow)
      blob([Module], (2, -1), size: (3, 1), color: yellow)

      // IR components
      blob([IR], (3.8, -3.4), size: (7.8, 8.2), color: red.lighten(60%), text_anchor: "bottom-center")
      blob([Module], (5, 0), size: (3, 1), color: red)
      blob([Function], (5, -1), size: (3, 1), color: red)
      blob([BasicBlock], (5, -2), size: (3, 1), color: red)
      blob([Instruction], (4, -3), size: (3, 1), color: red)
      blob([Terminator], (6, -3), size: (3, 1), color: red)

      // Optimizer components
      blob([Theorem Library], (8, 0), size: (4, 1), color: blue)
      blob([Theorem], (8.25, -1), size: (3, 1), color: blue)
      blob([State], (8.25, -2), size: (3, 1), color: color.navy)

      arrow((2.75, 0), (2.75, -0.48), side: 4)

      arrow((5.75, -.01), (5.75, -0.48), side: 4)
      arrow((5.75, -1.01), (5.75, -1.48), side: 4)
      arrow((5.75, -2.01), (4.75, -2.48), side: 4)
      arrow((5.75, -2.01), (6.75, -2.48), side: 3)
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
