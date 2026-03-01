mod ast;
mod codegen;
mod compiler;
mod lexer;
mod parser;
mod tokens;
mod typechecker;

use clap::Parser;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::exit;
use std::time::Instant;

use crate::codegen::*;
use crate::compiler::*;

#[derive(Parser, Debug)]
#[command(author, version, about = "The Celeste Compiler", long_about = None)]
struct Args {
    input: String,

    #[arg(short, long, default_value = "out")]
    output: String,

    #[arg(short, long, default_value_t = false)]
    dump_ast: bool,
}

fn main() {
    let start_time = Instant::now();
    let args = Args::parse();

    let input_path = Path::new(&args.input);
    match input_path.extension().and_then(|s| s.to_str()) {
        Some("cel") => (),
        _ => {
            eprintln!("error: input file must have a .cel extension");
            exit(1);
        }
    }

    let output_path: PathBuf = if args.output == "out" {
        input_path.with_extension("obj")
    } else {
        Path::new(&args.output).with_extension("obj")
    };

    let src = match read_to_string(input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("error reading file '{}': {}", args.input, e);
            exit(1);
        }
    };

    let mut comp = Compiler::new(src, input_path);
    let program = parser::parse(&mut comp);
    comp.register_functions(&program);
    let mut checker = typechecker::TypeChecker::new(&mut comp);
    checker.check_program(&program);

    if args.dump_ast {
        let mut ast_content = String::new();
        for stmt in &program.stmts {
            ast_content.push_str(&format!("{:#?}\n", stmt));
        }
        let ast_path = input_path.with_extension("celeste_ast.txt");
        std::fs::write(&ast_path, ast_content).expect("Failed to write AST dump");
        println!("AST dumped to: {}", ast_path.display());
    }

    let mut backend = CraneliftAOTBackend::new();
    println!("Compiling {}...", input_path.display());
    backend.compile_program(&program, &mut comp);

    let output_str = output_path
        .to_str()
        .expect("Output path contains invalid characters");
    backend.finalize_to_file(output_str);

    println!("Success: Generated {}", output_str);

    let exe_extension = if cfg!(target_os = "windows") {
        "exe"
    } else {
        ""
    };
    let exe_path = if args.output == "out" {
        input_path.with_extension(exe_extension)
    } else {
        Path::new(&args.output).with_extension(exe_extension)
    };

    let exe_str = exe_path.to_str().expect("Invalid path");

    let mut linker = Command::new("clang");

    linker.arg(output_str);

    linker.arg("-L./bin");
    linker.arg("-lceleste_std");

    linker.arg("-o").arg(exe_str);

    if cfg!(target_os = "windows") {
        linker
            .arg("-lmsvcrt")
            .arg("-llegacy_stdio_definitions")
            .arg("-Wno-deprecated-declarations")
            .arg("-D_CRT_SECURE_NO_WARNINGS")
            .arg("-Wl,/NODEFAULTLIB:libcmt");
    } else if cfg!(target_os = "macos") {
        linker.arg("-lSystem");
    } else {
        linker.arg("-lc").arg("-lm");
    }

    let status = linker.status().expect("Failed to execute link command");

    if status.success() {
        println!("Linking successful! Created: {}", exe_str);
    } else {
        eprintln!("Linking failed with code: {:?}", status.code());
    }

    let duration = start_time.elapsed();
    println!("------------------------------------------");
    println!("Compilation finished in {:.4}s", duration.as_secs_f64());
}
