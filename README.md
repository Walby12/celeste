# Celeste Compiler

A lightweight, high-performance Ahead-of-Time (AOT) compiler built in Rust. Celeste leverages the **Cranelift** code generation framework to turn `.cel` source code into optimized native executables.

> [!WARNING]
> **Project Status: Experimental** > Celeste is currently in early development. While it supports fundamental programming constructs, the syntax and ABI are subject to frequent changes as the language evolves.

## Features

- ** Blazing Fast**: High-speed machine code generation via Cranelift.
- ** Seamless C Interop**: Easily call functions from the C standard library (printf, scanf, etc.) using `extrn`.
- ** Integrated Toolchain**: Automatically handles the transition from source -> object file -> linked executable via `clang`.
- ** Debug Ready**: Built-in AST dumping to visualize how your code is parsed and structured.

---

## Prerequisites
- **Rust**: For building the compiler
- **LLVM/Clang**: Required for linking object files. (Ensure `clang` is in your PATH).
