#import "@preview/cheq:0.3.0": checklist

#show: checklist.with(fill: luma(95%), radius: .2em, stroke: blue)

= Roadmap

This section outlines the planned roadmap for the development and enhancement of the Hyperion system. The roadmap is divided into short-term, mid-term, and long-term goals, each focusing on different aspects of the system's capabilities and performance.

== Short-Term Goals

- [x] Add `Instance` to represent a library instance.
- [x] Add extension mechanism with plugin systems.
- [x] Add basic python bindings with `pyo3` and `maturin`.
- [x] Implement automated CI for python bindings.
- [x] Add basic C bindings with `cbindgen` and compatibility layer.
- [x] Add parser for simple logical expressions and able to parse basic assembly in custom IR syntax.
- [x] Add `fmt` support for core data structures.
- [x] Add compilation API endpoints and compile to internal representation (serialized format of the AST)
- [x] Allow to load from serialized IR
- [x] Add zstd to compress serialized IR
- [ ] Add `Device` to represent different hardware device that can be targeted (e.g., CPU, GPU, TPU).
- [?] Add `Network` to represent a network between devices, (need to be designed)
- [ ] Add `DeviceCluster` to represent a cluster/group of such devices.
- [ ] Add `Executor` trait to represent execution strategies on devices or clusters.
- [ ] Implement first `Executor`, a simple single-threaded evaluator.
- [x] Construct core IR data structures
- [x] Build typesystem to support typed IR
- [!] Add type checker for the IR
- [ ] Implement basic IR theorem-derivation and proof system
- [ ] Implement theorem derivation strategies
- [ ] Build equivalent function from theorem
- [ ] Figure out memory schema and management within the formal system

== Mid-Term Goals

- [ ] Construct plugin to transpile from custom IR to LLVM IR
- [ ] Build executor to run compiled code using LLVM JIT
- [ ] Integrate basic codegen: x86, ARM
- [ ] Integrate codegen on GPU: CUDA, ROCm. Integrate with their respective drivers.
- [ ] Implement algorithm reusal and caching with pre-built proofs
- [ ] Add basic complexity analysis for IR functions
- [ ] Figure out multi-threaded concurrent program and how to reason and build proofs about them and their execution.
- [ ] Start massively parallel execution strategies on clusters of devices.

== Long-Term Goals

- [ ] Attempt compiling python to our IR using custom frontend
- [ ] Similar with some low language with LLVM IR frontend (e.g., Rust, C, C++). Probably write bindings and variant to allow for easier interop.
- [ ] Conceptualize a language frontend for generating IR
- [ ] Implement this frontend
- [ ] Build standard library of optimized routines
- [ ] Figure out shared filesystem


