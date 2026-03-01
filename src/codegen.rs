use cranelift::prelude::*;
use cranelift_module::{DataDescription, FuncOrDataId, Linkage, Module};
use cranelift_native::builder as native_builder;
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::fs::File;
use std::io::Write;

use crate::ast::*;
use crate::compiler::*;

pub struct CraneliftAOTBackend {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    module: ObjectModule,
    str_count: usize,
    var_count: u32,
}

impl CraneliftAOTBackend {
    pub fn new() -> Self {
        let target_isa_builder = native_builder().expect("Host machine is not supported");
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
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
            var_count: 0,
        }
    }

    pub fn compile_program(&mut self, program: &Program, comp: &mut Compiler) {
        let ptr_type = self.module.target_config().pointer_type();
        let default_conv = self.module.target_config().default_call_conv;

        for stmt in &program.stmts {
            match stmt {
                Stmt::Extern {
                    name,
                    arg_types,
                    return_type,
                    ..
                } => {
                    let mut sig = self.module.make_signature();
                    sig.call_conv = default_conv;
                    for arg in arg_types {
                        sig.params.push(AbiParam::new(match arg {
                            CelesteType::Int => types::I64,
                            CelesteType::String => ptr_type,
                            CelesteType::Pointer(_) => ptr_type,
                            _ => types::I64,
                        }));
                    }
                    if *return_type == CelesteType::Int {
                        sig.returns.push(AbiParam::new(types::I64));
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
                            CelesteType::Int => types::I64,
                            CelesteType::String => ptr_type,
                            _ => types::I64,
                        }));
                    }
                    if return_type == "int" {
                        sig.returns.push(AbiParam::new(types::I64));
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
                return_type,
                ..
            } = stmt
            {
                self.compile_function(name, params, body, return_type, comp);
            }
        }
    }

    fn compile_function(
        &mut self,
        name: &str,
        params: &[Param],
        body: &[Stmt],
        return_type: &String,
        comp: &mut Compiler,
    ) {
        self.ctx.func.clear();
        self.var_count = 0;

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

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        comp.enter_scope();

        for (i, param) in params.iter().enumerate() {
            let ty = match param.ty {
                CelesteType::Int => types::I64,
                CelesteType::String => self.module.target_config().pointer_type(),
                _ => types::I64,
            };
            let var = Variable::from_u32(self.var_count);
            self.var_count += 1;
            builder.declare_var(var, ty);
            builder.def_var(var, builder.block_params(entry_block)[i]);

            comp.add_variable(
                param.name.clone(),
                VariableInfo {
                    cranelift_var: Some(var),
                    var_type: param.ty.clone(),
                    is_mutable: true,
                    stack_slot: None,
                },
            );
        }

        let mut terminated = false;
        for stmt in body {
            terminated = Self::translate_stmt(
                &mut builder,
                &mut self.module,
                &mut self.var_count,
                &mut self.str_count,
                stmt,
                comp,
                return_type,
            );
        }

        if !terminated {
            if return_type == "int" {
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().return_(&[zero]);
            } else {
                builder.ins().return_(&[]);
            };
        }

        comp.exit_scope();
        builder.finalize();
        self.module.define_function(func_id, &mut self.ctx).unwrap();
        self.module.clear_context(&mut self.ctx);
    }

    fn translate_stmt(
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        var_count: &mut u32,
        str_count: &mut usize,
        stmt: &Stmt,
        comp: &mut Compiler,
        return_type: &String,
    ) -> bool {
        match stmt {
            Stmt::Let { name, value, .. } => {
                let val = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    value,
                    comp,
                    return_type,
                );

                let slot = builder
                    .create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8));
                builder.ins().stack_store(val, slot, 0);

                let var = Variable::from_u32(*var_count);
                *var_count += 1;
                builder.declare_var(var, types::I64);
                builder.def_var(var, val);

                comp.add_variable(
                    name.clone(),
                    VariableInfo {
                        cranelift_var: Some(var),
                        stack_slot: Some(slot),
                        var_type: CelesteType::Int,
                        is_mutable: true,
                    },
                );
                false
            }
            Stmt::Assign { name, value, .. } => {
                let val = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    value.as_ref(),
                    comp,
                    return_type,
                );
                let info = comp.lookup_variable(name).unwrap();
                builder.def_var(info.cranelift_var.unwrap(), val);
                false
            }
            Stmt::If {
                condition,
                then_block,
                else_ifs,
                else_block,
                ..
            } => {
                let exit_block = builder.create_block();
                let mut next_conditional_block = builder.create_block();

                let cond_val = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    condition,
                    comp,
                    return_type,
                );
                let then_block_entry = builder.create_block();

                builder
                    .ins()
                    .brif(cond_val, then_block_entry, &[], next_conditional_block, &[]);

                builder.switch_to_block(then_block_entry);
                builder.seal_block(then_block_entry);
                comp.enter_scope();
                let mut then_terminated = false;
                for s in then_block {
                    then_terminated = Self::translate_stmt(
                        builder,
                        module,
                        var_count,
                        str_count,
                        s,
                        comp,
                        return_type,
                    );
                }
                comp.exit_scope();
                if !then_terminated {
                    builder.ins().jump(exit_block, &[]);
                }

                for (ei_cond, ei_body) in else_ifs {
                    builder.switch_to_block(next_conditional_block);
                    builder.seal_block(next_conditional_block);

                    let ei_then_block = builder.create_block();
                    next_conditional_block = builder.create_block();

                    let ei_cond_val = Self::translate_expr(
                        builder,
                        module,
                        var_count,
                        str_count,
                        ei_cond,
                        comp,
                        return_type,
                    );
                    builder.ins().brif(
                        ei_cond_val,
                        ei_then_block,
                        &[],
                        next_conditional_block,
                        &[],
                    );

                    builder.switch_to_block(ei_then_block);
                    builder.seal_block(ei_then_block);
                    comp.enter_scope();
                    let mut ei_terminated = false;
                    for s in ei_body {
                        ei_terminated = Self::translate_stmt(
                            builder,
                            module,
                            var_count,
                            str_count,
                            s,
                            comp,
                            return_type,
                        );
                    }
                    comp.exit_scope();
                    if !ei_terminated {
                        builder.ins().jump(exit_block, &[]);
                    }
                }

                builder.switch_to_block(next_conditional_block);
                builder.seal_block(next_conditional_block);
                if let Some(eb) = else_block {
                    comp.enter_scope();
                    let mut else_terminated = false;
                    for s in eb {
                        else_terminated = Self::translate_stmt(
                            builder,
                            module,
                            var_count,
                            str_count,
                            s,
                            comp,
                            return_type,
                        );
                    }
                    comp.exit_scope();
                    if !else_terminated {
                        builder.ins().jump(exit_block, &[]);
                    }
                } else {
                    builder.ins().jump(exit_block, &[]);
                }

                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
                false
            }
            Stmt::For {
                init,
                condition,
                post,
                body,
            } => {
                comp.enter_scope();
                if let Some(i) = init {
                    Self::translate_stmt(
                        builder,
                        module,
                        var_count,
                        str_count,
                        i,
                        comp,
                        return_type,
                    );
                }

                let header = builder.create_block();
                let body_blk = builder.create_block();
                let exit_blk = builder.create_block();

                builder.ins().jump(header, &[]);
                builder.switch_to_block(header);

                if let Some(cond) = condition {
                    let c = Self::translate_expr(
                        builder,
                        module,
                        var_count,
                        str_count,
                        cond,
                        comp,
                        return_type,
                    );
                    builder.ins().brif(c, body_blk, &[], exit_blk, &[]);
                } else {
                    builder.ins().jump(body_blk, &[]);
                }

                builder.switch_to_block(body_blk);
                builder.seal_block(body_blk);

                for s in body {
                    Self::translate_stmt(
                        builder,
                        module,
                        var_count,
                        str_count,
                        s,
                        comp,
                        return_type,
                    );
                }

                if let Some(p) = post {
                    Self::translate_stmt(
                        builder,
                        module,
                        var_count,
                        str_count,
                        p,
                        comp,
                        return_type,
                    );
                }

                builder.ins().jump(header, &[]);

                builder.switch_to_block(exit_blk);
                builder.seal_block(header);
                builder.seal_block(exit_blk);
                comp.exit_scope();
                false
            }
            Stmt::Return { value, .. } => {
                let val = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    value,
                    comp,
                    return_type,
                );
                builder.ins().return_(&[val]);
                true
            }
            Stmt::Expression(expr, ..) => {
                Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    expr,
                    comp,
                    return_type,
                );
                false
            }
            Stmt::PtrAssign { ptr_expr, value } => {
                let addr = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    ptr_expr,
                    comp,
                    return_type,
                );

                let val_to_store = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    value,
                    comp,
                    return_type,
                );

                builder.ins().store(MemFlags::new(), val_to_store, addr, 0);
                false
            }
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                let ptr_ty = module.target_config().pointer_type();

                let base_addr = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    array,
                    comp,
                    return_type,
                );
                let idx_val = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    index,
                    comp,
                    return_type,
                );
                let val_to_store = Self::translate_expr(
                    builder,
                    module,
                    var_count,
                    str_count,
                    value,
                    comp,
                    return_type,
                );

                let scale = builder.ins().iconst(ptr_ty, 8);
                let offset = builder.ins().imul(idx_val, scale);
                let final_addr = builder.ins().iadd(base_addr, offset);

                builder
                    .ins()
                    .store(MemFlags::new(), val_to_store, final_addr, 0);
                false
            }
            _ => false,
        }
    }

    fn translate_expr(
        builder: &mut FunctionBuilder,
        module: &mut ObjectModule,
        var_count: &mut u32,
        str_count: &mut usize,
        expr: &Expr,
        comp: &mut Compiler,
        _rt: &String,
    ) -> Value {
        match expr {
            Expr::Integer(n) => builder.ins().iconst(types::I64, *n as i64),
            Expr::Variable(name) => {
                let info = comp.lookup_variable(name).unwrap();
                if let Some(var) = info.cranelift_var {
                    builder.use_var(var)
                } else if let Some(slot) = info.stack_slot {
                    builder.ins().stack_load(types::I64, slot, 0)
                } else {
                    panic!("Variable {} has no storage", name);
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                let l = Self::translate_expr(builder, module, var_count, str_count, lhs, comp, _rt);
                let r = Self::translate_expr(builder, module, var_count, str_count, rhs, comp, _rt);

                match op {
                    '+' => builder.ins().iadd(l, r),
                    '-' => builder.ins().isub(l, r),
                    '*' => builder.ins().imul(l, r),
                    '/' => builder.ins().sdiv(l, r),
                    '%' => builder.ins().srem(l, r),

                    '=' => {
                        let res = builder.ins().icmp(IntCC::Equal, l, r);
                        builder.ins().uextend(types::I64, res)
                    }
                    'N' => {
                        let res = builder.ins().icmp(IntCC::NotEqual, l, r);
                        builder.ins().uextend(types::I64, res)
                    }
                    '<' => {
                        let res = builder.ins().icmp(IntCC::SignedLessThan, l, r);
                        builder.ins().uextend(types::I64, res)
                    }
                    '>' => {
                        let res = builder.ins().icmp(IntCC::SignedGreaterThan, l, r);
                        builder.ins().uextend(types::I64, res)
                    }
                    'L' => {
                        let res = builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
                        builder.ins().uextend(types::I64, res)
                    }
                    'G' => {
                        let res = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r);
                        builder.ins().uextend(types::I64, res)
                    }
                    _ => {
                        eprintln!("Codegen Error: Operator '{}' not implemented", op);
                        std::process::exit(1);
                    }
                }
            }
            Expr::StringLiteral(s) => {
                let mut dc = DataDescription::new();
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                dc.define(b.into_boxed_slice());
                let name = format!("str_{}", str_count);
                *str_count += 1;
                let id = module
                    .declare_data(&name, Linkage::Export, false, false)
                    .unwrap();
                module.define_data(id, &dc).unwrap();
                let loc = module.declare_data_in_func(id, &mut builder.func);
                builder
                    .ins()
                    .symbol_value(module.target_config().pointer_type(), loc)
            }
            Expr::Call { name, args } => {
                let fid = match module.get_name(name) {
                    Some(FuncOrDataId::Func(id)) => id,
                    _ => panic!("Function {} not found", name),
                };

                let mut av = Vec::new();
                for arg in args {
                    av.push(Self::translate_expr(
                        builder, module, var_count, str_count, arg, comp, _rt,
                    ));
                }

                let base_sig = module
                    .declarations()
                    .get_function_decl(fid)
                    .signature
                    .clone();

                if av.len() != base_sig.params.len() {
                    let mut dynamic_sig = base_sig.clone();

                    if av.len() > base_sig.params.len() {
                        for i in base_sig.params.len()..av.len() {
                            let actual_ty = builder.func.dfg.value_type(av[i]);
                            dynamic_sig.params.push(AbiParam::new(actual_ty));
                        }
                    }

                    let sig_ref = builder.import_signature(dynamic_sig);
                    let func_ref = module.declare_func_in_func(fid, &mut builder.func);
                    let ptr_type = module.target_config().pointer_type();
                    let callee_ptr = builder.ins().func_addr(ptr_type, func_ref);

                    let inst = builder.ins().call_indirect(sig_ref, callee_ptr, &av);

                    if base_sig.returns.is_empty() {
                        builder.ins().iconst(types::I64, 0)
                    } else {
                        builder.inst_results(inst)[0]
                    }
                } else {
                    let lfunc = module.declare_func_in_func(fid, &mut builder.func);
                    let inst = builder.ins().call(lfunc, &av);

                    if base_sig.returns.is_empty() {
                        builder.ins().iconst(types::I64, 0)
                    } else {
                        builder.inst_results(inst)[0]
                    }
                }
            }
            Expr::Unary { op, right } => {
                let val =
                    Self::translate_expr(builder, module, var_count, str_count, right, comp, _rt);
                match op {
                    '!' => {
                        let zero = builder.ins().iconst(types::I64, 0);
                        let res = builder.ins().icmp(IntCC::Equal, val, zero);
                        builder.ins().uextend(types::I64, res)
                    }
                    _ => todo!(),
                }
            }
            Expr::AddressOf(inner) => {
                let ptr_ty = module.target_config().pointer_type();

                match &**inner {
                    Expr::Variable(name) => {
                        let var_info = comp.lookup_variable(name).unwrap_or_else(|| {
                            panic!("Variable {} not found in codegen", name);
                        });

                        let slot = var_info
                            .stack_slot
                            .expect("Cannot take address of a variable with no stack slot");
                        builder.ins().stack_addr(ptr_ty, slot, 0)
                    }

                    Expr::Index { array, index } => {
                        let base_addr = Self::translate_expr(
                            builder, module, var_count, str_count, array, comp, _rt,
                        );

                        let idx_val = Self::translate_expr(
                            builder, module, var_count, str_count, index, comp, _rt,
                        );

                        let scale = builder.ins().iconst(ptr_ty, 8);
                        let offset = builder.ins().imul(idx_val, scale);
                        builder.ins().iadd(base_addr, offset)
                    }

                    _ => todo!("Cannot take address of this expression type: {:?}", inner),
                }
            }
            Expr::Deref(inner_expr) => {
                let ptr_val = Self::translate_expr(
                    builder, module, var_count, str_count, inner_expr, comp, _rt,
                );

                let inner_ty = comp.get_expr_type(inner_expr);

                let cl_ty = match inner_ty {
                    CelesteType::Pointer(base_ty) => comp.celeste_to_cranelift(&base_ty),
                    _ => types::I64,
                };

                builder.ins().load(cl_ty, MemFlags::new(), ptr_val, 0)
            }
            Expr::Index { array, index } => {
                let ptr_ty = module.target_config().pointer_type();

                let base_addr =
                    Self::translate_expr(builder, module, var_count, str_count, array, comp, _rt);
                let idx_val =
                    Self::translate_expr(builder, module, var_count, str_count, index, comp, _rt);

                let array_ty = comp.get_expr_type(array);
                let (inner_ty, element_size) = match array_ty {
                    CelesteType::Array(inner) | CelesteType::Pointer(inner) => (*inner.clone(), 8),
                    _ => (CelesteType::Int, 8),
                };

                let scale = builder.ins().iconst(ptr_ty, element_size);
                let offset = builder.ins().imul(idx_val, scale);
                let final_addr = builder.ins().iadd(base_addr, offset);

                let cl_ty = comp.celeste_to_cranelift(&inner_ty);
                builder.ins().load(cl_ty, MemFlags::new(), final_addr, 0)
            }
            Expr::ArrayLiteral(elements) => {
                let ptr_type = module.target_config().pointer_type();
                let element_size = 8;
                let total_size = (elements.len() * element_size) as u32;

                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    total_size,
                ));

                for (i, element_expr) in elements.iter().enumerate() {
                    let val = Self::translate_expr(
                        builder,
                        module,
                        var_count,
                        str_count,
                        element_expr,
                        comp,
                        _rt,
                    );
                    let offset = (i * element_size) as i32;
                    builder.ins().stack_store(val, slot, offset);
                }

                builder.ins().stack_addr(ptr_type, slot, 0)
            }
        }
    }

    pub fn finalize_to_file(self, path: &str) {
        let product = self.module.finish();
        let bytes = product.emit().expect("Failed to emit object bytes");
        let mut file = File::create(path).expect("Could not create output file");
        file.write_all(&bytes).expect("Failed to write object file");
    }
}
