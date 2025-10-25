# HyInstr is the instruction set portion of the hyperion framework

This module contains the instruction set definitions along with some other constructs
that are used to define and manipulates programs.

## Description

`HyInstr` defines the instruction set, modules, and programs for the Hyperion framework. It allows for a cross-platform
representation of programs, enabling analysis, transformation, and execution across different architectures. It closely
matches the `llvm` intermediate representation (IR).

### Goals
- Provide a cross-platform instruction set representation.
- Enable analysis and transformation of programs.
- Facilitate execution of programs on different architectures.
- Maintain a close relationship with the `llvm` IR for compatibility and ease of translation.
- Support modular program structures for better organization and reuse.

### Notes on non-conformant hardware

Certain hardware architectures may differ significantly making their usage more difficult. Notably non-von Neumann
architectures such as fixed-pipeline DSPs or GPUs may not map well to the HyInstr representation.
We can represent `FPGA` functionally as a device capable of executing operations in parallel.

For other fixed-function hardware, representing them in HyInstr may not be *feasible*.


