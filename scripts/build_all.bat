@echo off
setlocal enabledelayedexpansion

echo [1/3] Creating build directories...
if not exist "bin" mkdir bin
if not exist "target_cel" mkdir target_cel

echo [2/3] Compiling celeste standard library...
clang -c stdlib/stdlib.c -o bin/stdlib.obj
if %errorlevel% neq 0 (
    echo Error: Failed to compile stdlib.c
    exit /b 1
)
llvm-ar rcs bin/celeste_std.lib bin/stdlib.obj
if %errorlevel% neq 0 (
    echo Error: Failed to archive celeste_std.lib
    exit /b 1
)

echo [3/3] Compiling celeste compiler...
cargo build --release
if %errorlevel% neq 0 (
    echo Error: Rust build failed
    exit /b 1
)

copy /y target\release\celeste.exe celeste.exe

echo ---------------------------------------
echo Build Successful!
echo To compile a Celeste file, use:
echo .\celeste.exe .\tests\test.cel
echo ---------------------------------------
