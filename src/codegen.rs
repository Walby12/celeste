use cranelift::codegen::ir::StackSlot;
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

        let func_id = match self.module.get_name(name) {
            Some(FuncOrDataId::Func(id)) => id,
            _ => panic!("Function {} not declared", name),
        };

        let sig = self
            .module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();
        self.ctx.func.signature = sig;

        let ptr_type = self.module.target_config().pointer_type();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
        let entry_block = builder.create_block();

        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut stack_slots = HashMap::new();
        for (var_name, local) in locals {
            let size = match local.ty {
                CelesteType::Int => 4,
                CelesteType::String => ptr_type.bytes() as u32,
                _ => 4,
            };
            let slot = builder
                .create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, size));
            stack_slots.insert(var_name.clone(), slot);
        }

        for (i, param) in params.iter().enumerate() {
            if let Some(slot) = stack_slots.get(&param.name) {
                let val = builder.block_params(entry_block)[i];
                builder.ins().stack_store(val, *slot, 0);
            }
        }

        let mut terminated = false;
        for stmt in body {
            terminated = Self::translate_stmt(
                &mut builder,
                stmt,
                &stack_slots,
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
        stack_slots: &HashMap<String, StackSlot>,
        return_type: &String,
        module: &mut ObjectModule,
        str_count: &mut usize,
    ) -> bool {
        match stmt {
            Stmt::Let { name, value } => {
                let val = Self::translate_expr(
                    builder,
                    value,
                    stack_slots,
                    return_type,
                    module,
                    str_count,
                );
                let slot = stack_slots.get(name).expect("Variable missing");
                builder.ins().stack_store(val, *slot, 0);
                false
            }
            Stmt::Assign { name, value } => {
                let val = Self::translate_expr(
                    builder,
                    value.as_ref(),
                    stack_slots,
                    return_type,
                    module,
                    str_count,
                );
                let slot = stack_slots.get(name).expect("Variable missing");
                builder.ins().stack_store(val, *slot, 0);
                false
            }
            Stmt::Return { value } => {
                let val = Self::translate_expr(
                    builder,
                    value,
                    stack_slots,
                    return_type,
                    module,
                    str_count,
                );
                builder.ins().return_(&[val]);
                true
            }
            Stmt::Expression(expr) => {
                Self::translate_expr(builder, expr, stack_slots, return_type, module, str_count);
                false
            }
            Stmt::If {
                condition,
                then_block,
                else_ifs,
                else_block,
            } => {
                let merge_block = builder.create_block();

                let mut next_test_block = builder.create_block();
                let cond_val = Self::translate_expr(
                    builder,
                    condition,
                    stack_slots,
                    return_type,
                    module,
                    str_count,
                );

                let then_body_block = builder.create_block();
                builder
                    .ins()
                    .brif(cond_val, then_body_block, &[], next_test_block, &[]);

                builder.switch_to_block(then_body_block);
                builder.seal_block(then_body_block);
                let mut then_terminated = false;
                for stmt in then_block {
                    then_terminated |= Self::translate_stmt(
                        builder,
                        stmt,
                        stack_slots,
                        return_type,
                        module,
                        str_count,
                    );
                }
                if !then_terminated {
                    builder.ins().jump(merge_block, &[]);
                }

                for (ei_cond, ei_body) in else_ifs {
                    builder.switch_to_block(next_test_block);
                    builder.seal_block(next_test_block);

                    let ei_cond_val = Self::translate_expr(
                        builder,
                        ei_cond,
                        stack_slots,
                        return_type,
                        module,
                        str_count,
                    );
                    let ei_body_block = builder.create_block();
                    let ei_next_test = builder.create_block();

                    builder
                        .ins()
                        .brif(ei_cond_val, ei_body_block, &[], ei_next_test, &[]);

                    builder.switch_to_block(ei_body_block);
                    builder.seal_block(ei_body_block);
                    let mut ei_terminated = false;
                    for stmt in ei_body {
                        ei_terminated |= Self::translate_stmt(
                            builder,
                            stmt,
                            stack_slots,
                            return_type,
                            module,
                            str_count,
                        );
                    }
                    if !ei_terminated {
                        builder.ins().jump(merge_block, &[]);
                    }
                    next_test_block = ei_next_test;
                }

                builder.switch_to_block(next_test_block);
                builder.seal_block(next_test_block);
                if let Some(body) = else_block {
                    let mut else_terminated = false;
                    for stmt in body {
                        else_terminated |= Self::translate_stmt(
                            builder,
                            stmt,
                            stack_slots,
                            return_type,
                            module,
                            str_count,
                        );
                    }
                    if !else_terminated {
                        builder.ins().jump(merge_block, &[]);
                    }
                } else {
                    builder.ins().jump(merge_block, &[]);
                }

                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);

                false
            }
            _ => false,
        }
    }

    fn translate_expr(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        stack_slots: &HashMap<String, StackSlot>,
        return_type: &String,
        module: &mut ObjectModule,
        str_count: &mut usize,
    ) -> Value {
        let ptr_type = module.target_config().pointer_type();

        match expr {
            Expr::Integer(n) => builder.ins().iconst(types::I32, *n as i64),
            Expr::Variable(name) => {
                let slot = stack_slots.get(name).expect("Undefined variable");
                builder.ins().stack_load(types::I32, *slot, 0)
            }
            Expr::Binary { op, lhs, rhs } => {
                let l = Self::translate_expr(
                    builder,
                    lhs.as_ref(),
                    stack_slots,
                    return_type,
                    module,
                    str_count,
                );
                let r = Self::translate_expr(
                    builder,
                    rhs.as_ref(),
                    stack_slots,
                    return_type,
                    module,
                    str_count,
                );
                match op {
                    '+' => builder.ins().iadd(l, r),
                    '-' => builder.ins().isub(l, r),
                    '*' => builder.ins().imul(l, r),
                    '/' => builder.ins().sdiv(l, r),

                    '<' => {
                        let res = builder.ins().icmp(IntCC::SignedLessThan, l, r);
                        builder.ins().uextend(types::I32, res)
                    }
                    '>' => {
                        let res = builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
                        builder.ins().uextend(types::I32, res)
                    }
                    '=' => {
                        let res = builder.ins().icmp(IntCC::Equal, l, r);
                        builder.ins().uextend(types::I32, res)
                    }
                    _ => panic!("Unsupported binary operator: {}", op),
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
                    .unwrap();
                module.define_data(data_id, &data_ctx).unwrap();
                *str_count += 1;
                let local_id = module.declare_data_in_func(data_id, &mut builder.func);
                builder.ins().symbol_value(ptr_type, local_id)
            }
            Expr::Call { name, args } => {
                let func_id = match module.get_name(name) {
                    Some(FuncOrDataId::Func(id)) => id,
                    _ => {
                        eprintln!("error: undeclared function '{}'", name);
                        exit(1);
                    }
                };

                let mut arg_values = Vec::new();
                let mut call_sig = module.make_signature();
                call_sig.call_conv = module.target_config().default_call_conv;

                for arg in args {
                    let val = Self::translate_expr(
                        builder,
                        arg,
                        stack_slots,
                        return_type,
                        module,
                        str_count,
                    );
                    arg_values.push(val);
                    let arg_ty = builder.func.dfg.value_type(val);
                    call_sig.params.push(AbiParam::new(arg_ty));
                }

                call_sig.returns.push(AbiParam::new(types::I32));

                let sig_ref = builder.import_signature(call_sig);

                let local_func = module.declare_func_in_func(func_id, &mut builder.func);
                let func_ptr = builder.ins().func_addr(ptr_type, local_func);

                let call = builder.ins().call_indirect(sig_ref, func_ptr, &arg_values);

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
