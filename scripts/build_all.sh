#!/bin/bash

set -e

echo "[1/3] Creating build directories..."
mkdir -p bin
mkdir -p target_cel

echo "[2/3] Compiling celeste standard library..."
clang -c stdlib/stdlib.c -o bin/stdlib.o

ar rcs bin/libceleste_std.a bin/stdlib.o

echo "[3/3] Compiling celeste compiler..."
cargo build --release

cp target/release/celeste ./celeste

echo "---------------------------------------"
echo "Build Successful!"
echo "To compile a Celeste file, use:"
echo "./celeste ./tests/test.cel"
echo "---------------------------------------"
