# Roadmap

This document outlines the planned features and milestones for the Hyperion project. It serves as a guide for development priorities and helps track progress over time.

## Draft for API

- [x] Add `Instance` to represent a library instance
- [x] Add EXTENSION mechanism for plugins
- [x] Add compilation to internal representation from `IR`
- [x] Add loading compiled `IR` from file/memory into `Module`
- [ ] Add `OptimizerPipeline` with `create`, `destroy`, construct from passes
- [ ] Add `Buffer`, `BufferView`
- [ ] Add `Device` to represent CPU/GPU/TPU devices
- [ ] Add list of `Device`s (multi-device, multi-node, multi-cloud) to form a `DeviceCluster`
- [ ] Add `Network` to represent inter-device communication
- [ ] Add `Executor` with `create`, `destroy`, `execute_function`. Execute on a `DeviceCluster` with `BufferView` inputs/outputs.
- [ ] Add some low-level optimization passes
- [ ] Add some smart pipelining/lazy eval/on-the-fly optimization passes

## Short-term goals

- [x] Construct core IR data structures
- [x] Build typesystem to support typed IR
- [ ] Construct a type-checker for the IR
- [x] Implement plugin system for extensibility
- [x] Implement logging and error handling framework
- [x] Add python bindings for core components
- [x] Add C/C++ bindings for core components
- [ ] Implement basic IR theorem-derivation and proof system
- [ ] Implement theorem derivation strategies
- [ ] Build equivalent function from theorem

## Medium-term goals

- [ ] Build executor to run compiled code
- [ ] Build interpreter for IR
- [ ] Integrate with LLVMs for codegen backend
- [ ] Integrate basic codegen: x86, ARM
- [ ] Integrate codegen on GPU: CUDA, ROCm. Integrate with their respective drivers.
- [ ] Implement algorithm reusal and caching with pre-built proofs

## Long-term goals

- [ ] Conceptualize a language frontend for generating IR
- [ ] Implement this frontned
- [ ] Integrate with python trying to make it as seamless as possible
- [ ] Build standard library of optimized routines
