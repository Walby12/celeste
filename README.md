# celeste csompiler

A lightweight Ahead-of-Time (AOT) compiler built in Rust using the **Cranelift** code generation framework. celeste compiles `.cel` files into native executables.

> [!WARNING]
> **Project Status: Experimental** > This compiler is currently a proof-of-concept and is in early development. It is **not** intended for production use. The syntax and backend implementation are subject to frequent breaking changes. Use at your own risk when executing generated binaries.

## Features
- **Fast Compilation**: Powered by Cranelift.
- **AST Dumping**: Visualize the compiler's intermediate representation.
- **Auto-Linking**: Automatically invokes `clang` to produce final executables.

## Prerequisites
- **Rust**: For building the compiler
- **LLVM/Clang**: Required for linking object files. (Ensure `clang` is in your PATH).
