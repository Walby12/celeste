use cranelift::prelude::*;
use cranelift_module::FuncOrDataId;
use cranelift_module::{DataDescription, Linkage, Module};
use cranelift_native::builder as native_builder;
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::process::exit;

use crate::ast::*;
use crate::compiler::*;

pub struct CraneliftAOTBackend {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    module: ObjectModule,
    str_count: usize,
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
            str_count: 0,
        }
    }

    pub fn compile_program(&mut self, program: &Program) {
        let ptr_type = self.module.target_config().pointer_type();
        let default_conv = self.module.target_config().default_call_conv;

        for stmt in &program.stmts {
            match stmt {
                Stmt::Extern {
                    name,
                    arg_types,
                    return_type,
                } => {
                    let mut sig = self.module.make_signature();
                    sig.call_conv = default_conv;

                    for arg in arg_types {
                        sig.params.push(AbiParam::new(match arg {
                            CelesteType::Int => types::I32,
                            CelesteType::String => ptr_type,
                            _ => types::I32,
                        }));
                    }

                    if *return_type == CelesteType::Int {
                        sig.returns.push(AbiParam::new(types::I32));
                    }

                    self.module
                        .declare_function(name, Linkage::Import, &sig)
                        .unwrap();
                }

                Stmt::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let mut sig = self.module.make_signature();
                    sig.call_conv = default_conv;

                    for param in params {
                        sig.params.push(AbiParam::new(match param.ty {
                            CelesteType::Int => types::I32,
                            CelesteType::String => ptr_type,
                            _ => types::I32,
                        }));
                    }

                    if return_type == "int" {
                        sig.returns.push(AbiParam::new(types::I32));
                    } else if return_type == "string" {
                        sig.returns.push(AbiParam::new(ptr_type));
                    }

                    self.module
                        .declare_function(name, Linkage::Export, &sig)
                        .unwrap();
                }
                _ => {}
            }
        }

        for stmt in &program.stmts {
            if let Stmt::Function {
                name,
                params,
                body,
                locals,
                return_type,
            } = stmt
            {
                self.compile_function(name, params, body, locals, return_type);
            }
        }
    }

    fn compile_function(
        &mut self,
        name: &str,
        params: &[Param],
        body: &[Stmt],
        locals: &HashMap<String, Local>,
        return_type: &String,
    ) {
        self.ctx.func.clear();
        self.ctx.func.signature.params.clear();
        self.ctx.func.signature.call_conv = self.module.target_config().default_call_conv;
        self.ctx.func.signature.returns.clear();

        let ptr_type = self.module.target_config().pointer_type();

        for param in params {
            let ty = match param.ty {
                CelesteType::Int => types::I32,
                CelesteType::String => ptr_type,
                _ => types::I32,
            };
            self.ctx.func.signature.params.push(AbiParam::new(ty));
        }

        if return_type == "int" {
            self.ctx
                .func
                .signature
                .returns
                .push(AbiParam::new(types::I32));
        } else if return_type == "string" {
            self.ctx
                .func
                .signature
                .returns
                .push(AbiParam::new(ptr_type));
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
        sorted_locals.sort_by_key(|(name, _)| *name);

        for (i, (var_name, local)) in sorted_locals.into_iter().enumerate() {
            let var_ref = Variable::new(i);
            let cranelift_type = match local.ty {
                CelesteType::Int => types::I32,
                CelesteType::String => ptr_type,
                CelesteType::Void => continue,
            };

            builder.declare_var(var_ref, cranelift_type);
            var_map.insert(var_name.clone(), var_ref);
        }

        for (i, param) in params.iter().enumerate() {
            if let Some(var_ref) = var_map.get(&param.name) {
                let val = builder.block_params(entry_block)[i];
                builder.def_var(*var_ref, val);
            }
        }

        let mut terminated = false;
        for stmt in body {
            terminated = Self::translate_stmt(
                &mut builder,
                stmt,
                &var_map,
                return_type,
                &mut self.module,
                &mut self.str_count,
            );
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
        module: &mut ObjectModule,
        str_count: &mut usize,
    ) -> bool {
        match stmt {
            Stmt::Let { name, value } => {
                let val =
                    Self::translate_expr(builder, value, var_map, return_type, module, str_count);
                let var_ref = var_map.get(name).expect("Variable missing in codegen");
                builder.def_var(*var_ref, val);
                false
            }
            Stmt::Assign { name, value } => {
                let val =
                    Self::translate_expr(builder, value, var_map, return_type, module, str_count);
                let var_ref = var_map.get(name).expect("Variable missing in codegen");
                builder.def_var(*var_ref, val);
                false
            }
            Stmt::Return { value } => {
                let val =
                    Self::translate_expr(builder, value, var_map, return_type, module, str_count);
                if return_type == "void" {
                    builder.ins().return_(&[]);
                } else {
                    builder.ins().return_(&[val]);
                }
                true
            }
            Stmt::Expression(expr) => {
                Self::translate_expr(builder, expr, var_map, return_type, module, str_count);
                false
            }
            _ => false,
        }
    }

    fn translate_expr(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        var_map: &HashMap<String, Variable>,
        return_type: &String,
        module: &mut ObjectModule,
        str_count: &mut usize,
    ) -> Value {
        match expr {
            Expr::Integer(n) => builder.ins().iconst(types::I32, *n as i64),
            Expr::Variable(name) => {
                let var = var_map.get(name).expect("Usage of undefined variable");
                builder.use_var(*var)
            }
            Expr::Binary { op, lhs, rhs } => {
                let l = Self::translate_expr(builder, lhs, var_map, return_type, module, str_count);
                let r = Self::translate_expr(builder, rhs, var_map, return_type, module, str_count);
                match op {
                    '+' => builder.ins().iadd(l, r),
                    '-' => builder.ins().isub(l, r),
                    '*' => builder.ins().imul(l, r),
                    '/' => builder.ins().sdiv(l, r),
                    _ => unreachable!(),
                }
            }
            Expr::StringLiteral(s) => {
                let mut data_ctx = DataDescription::new();
                let mut bytes = s.as_bytes().to_vec();
                bytes.push(0);
                data_ctx.define(bytes.into_boxed_slice());

                let name = format!("str_{}", *str_count);
                let data_id = module
                    .declare_data(&name, Linkage::Export, false, false)
                    .expect("Failed to declare string data");

                module
                    .define_data(data_id, &data_ctx)
                    .expect("Failed to define data");
                *str_count += 1;

                let local_id = module.declare_data_in_func(data_id, &mut builder.func);
                let pointer_type = module.target_config().pointer_type();
                builder.ins().symbol_value(pointer_type, local_id)
            }
            Expr::Call { name, args } => {
                let func_id = match module.get_name(name) {
                    Some(FuncOrDataId::Func(id)) => id,
                    _ => {
                        eprintln!("error: call to undeclared function '{}'", name);
                        exit(1);
                    }
                };

                let mut arg_values = Vec::new();
                let mut call_sig = module.make_signature();
                call_sig.call_conv = module.target_config().default_call_conv;

                for arg in args {
                    let val =
                        Self::translate_expr(builder, arg, var_map, return_type, module, str_count);
                    arg_values.push(val);

                    let ty = builder.func.dfg.value_type(val);
                    call_sig.params.push(AbiParam::new(ty));
                }

                call_sig.returns.push(AbiParam::new(types::I32));

                let sig_ref = builder.import_signature(call_sig);
                let local_func = module.declare_func_in_func(func_id, &mut builder.func);

                builder.func.dfg.ext_funcs[local_func].signature = sig_ref;
                let call = builder.ins().call(local_func, &arg_values);

                let results = builder.inst_results(call);
                if results.is_empty() {
                    builder.ins().iconst(types::I32, 0)
                } else {
                    results[0]
                }
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
