use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use cranelift_native::builder as native_builder;
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;

use crate::ast::*;

pub struct CraneliftAOTBackend {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    module: ObjectModule,
}

impl CraneliftAOTBackend {
    pub fn new() -> Self {
        let target_isa_builder = native_builder().expect("Host machine is not supported");
        let flag_builder = settings::builder();
        let isa = target_isa_builder
            .finish(settings::Flags::new(flag_builder))
            .expect("Failed to create ISA");

        let obj_builder = ObjectBuilder::new(
            isa,
            "celeste_module",
            cranelift_module::default_libcall_names(),
        )
        .expect("Failed to create object builder");

        let module = ObjectModule::new(obj_builder);

        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        }
    }

    pub fn compile_program(&mut self, program: &Program) {
        for stmt in &program.stmts {
            match stmt {
                Stmt::Function {
                    name,
                    body,
                    locals,
                    return_type,
                } => {
                    self.compile_function(name, body, locals, return_type);
                }
                _ => {
                    todo!()
                }
            }
        }
    }

    fn compile_function(
        &mut self,
        name: &str,
        body: &[Stmt],
        locals: &HashSet<String>,
        return_type: &String,
    ) {
        self.ctx.func.clear();
        self.ctx.func.signature.params.clear();
        self.ctx.func.signature.returns.clear();

        if return_type == "int" {
            self.ctx
                .func
                .signature
                .returns
                .push(AbiParam::new(types::I32));
        }

        let func_id = self
            .module
            .declare_function(name, Linkage::Export, &self.ctx.func.signature)
            .expect("Function declaration failed");

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
        let entry_block = builder.create_block();

        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut var_map = HashMap::new();
        let mut sorted_locals: Vec<_> = locals.iter().collect();
        sorted_locals.sort();

        for (i, var_name) in sorted_locals.into_iter().enumerate() {
            let var_ref = Variable::new(i);
            builder.declare_var(var_ref, types::I32);
            var_map.insert(var_name.clone(), var_ref);
        }

        let mut terminated = false;

        for stmt in body {
            terminated = Self::translate_stmt(&mut builder, stmt, &var_map, return_type);
        }

        if !terminated {
            if return_type == "void" {
                builder.ins().return_(&[]);
            } else {
                let zero = builder.ins().iconst(types::I32, 0);
                builder.ins().return_(&[zero]);
            }
        }

        builder.finalize();

        self.module.define_function(func_id, &mut self.ctx).unwrap();
        self.module.clear_context(&mut self.ctx);
    }

    fn translate_stmt(
        builder: &mut FunctionBuilder,
        stmt: &Stmt,
        var_map: &HashMap<String, Variable>,
        return_type: &String,
    ) -> bool {
        match stmt {
            Stmt::Let { name, value } => {
                let val = Self::translate_expr(builder, value, var_map, return_type);

                let var_ref = var_map
                    .get(name)
                    .expect("Variable not found in map (Compiler/Parser desync)");

                builder.def_var(*var_ref, val);
                false
            }
            Stmt::Return { value } => {
                let val = Self::translate_expr(builder, value, var_map, return_type);

                if return_type == "int" {
                    builder.ins().return_(&[val]);
                } else {
                    builder.ins().return_(&[]);
                }
                true
            }
            _ => false,
        }
    }

    fn translate_expr(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        var_map: &HashMap<String, Variable>,
        return_type: &String,
    ) -> Value {
        match expr {
            Expr::Integer(n) => builder.ins().iconst(types::I32, *n as i64),
            Expr::Variable(name) => {
                let var = var_map.get(name).expect("Usage of undefined variable");
                builder.use_var(*var)
            }
        }
    }

    pub fn finalize_to_file(self, path: &str) {
        let product = self.module.finish();
        let bytes = product.emit().expect("Failed to emit bytes");

        let mut file = File::create(path).expect("File creation failed");
        file.write_all(&bytes).expect("Write failed");
    }
}
