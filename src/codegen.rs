use crate::token::{TokenType, Token};
use crate::ast::{Expr, Stmt, LiteralValue, VarType};
use crate::error::HypercError;
use crate::support::{expr_span, stmt_span, is_comparison, mangle};
use std::path::Path;
use std::{
    process::Command,
    collections::HashMap
};
use inkwell::{
    AddressSpace, 
    OptimizationLevel,
    basic_block::BasicBlock,
    builder::{Builder, BuilderError},
    context::Context,
    module::Module,
    targets::{
        CodeModel, FileType,
        InitializationConfig,
        RelocMode, Target,
        TargetMachine,
    }, types:: {
        BasicType, 
        BasicTypeEnum, StructType
    }, values::{
        BasicValueEnum, 
        IntValue, FloatValue,
        PointerValue, ValueKind
    }
};

impl From<BuilderError> for HypercError {
    fn from(value: BuilderError) -> Self {
        return HypercError::CompileError {
            span: 0..0,
            message: value.to_string()
        }
    }
}

pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, 
        (PointerValue<'ctx>, BasicTypeEnum<'ctx>, VarType)>,
    struct_types: HashMap<String, 
        (StructType<'ctx>, Vec<(String, VarType)>)>,
    enum_types: HashMap<String, Vec<String>>
} 

impl <'ctx>Codegen<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("module");
        let builder = context.create_builder();
        let variables = HashMap::new();
        let struct_types = HashMap::new();
        let enum_types = HashMap::new();
        Codegen {
            context, module,
            builder, variables,
            struct_types, enum_types
        }
    }

    pub fn compile(&mut self, 
        stmts: &[Stmt], path: &str, 
        out: &str, is_debug: bool
    ) -> Result<(), HypercError> {
        for stmt in stmts {
            self.compile_stmt(stmt)?;
        }
        if is_debug {
            println!("{}", self.module.print_to_string().to_string())
        }
        match self.module.verify() {
            Ok(()) => {}
            Err(e) => {
                return Err(HypercError::CompileError {
                    span: 0..0,
                    message: e.to_string()
                })
            }
        }
        self.emit_obj(path, out)?;
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), HypercError> {
        match stmt {
            
            Stmt::Expression { value } => {
                self.compile_expr(value)?;
                Ok(())
            }

            Stmt::Print { value } => {
                let i32_type = self.context.i32_type();
                let ptr = self.context.ptr_type(AddressSpace::default());
                let fn_type = i32_type.fn_type(
                    &[ptr.into()],
                    true
                );

                let print_fn = match self.module.get_function("printf") {
                    Some(printf) => printf,
                    None => self.module.add_function("printf", fn_type, None)
                };

                let result = self.compile_expr(value)?;
                let fmt_str = if result.is_pointer_value() {
                    "%s\n"
                } else if result.is_float_value() {
                    "%f\n"
                } else if result.into_int_value().get_type().get_bit_width() == 1 {
                    "%b\n"
                } else if result.into_int_value().get_type().get_bit_width() == 8 {
                    "%c\n"
                } else {
                    "%ld\n"
                };

                if fmt_str == "%b\n" {
                    let ptr_true = self.builder
                     .build_global_string_ptr("true\n", "true_ptr")?;
                    let ptr_false = self.builder
                     .build_global_string_ptr("false\n", "false_ptr")?;
                    let select = self.builder.build_select(
                        result.into_int_value(), 
                        ptr_true, 
                        ptr_false, 
                        "boolstr"
                    )?;
                    self.builder.build_call(
                        print_fn, 
                        &[select.into()], 
                        "print"
                    )?;
                    return Ok(());
                }

                let fmt = self.builder.build_global_string_ptr(
                    fmt_str, 
                    "fmt"
                )?;
                self.builder.build_call(
                    print_fn, 
                    &[fmt.as_pointer_value().into(), result.into()], 
                    "print"
                )?;
                Ok(())
            }

            Stmt::Let { name, value, var_type, .. } => {
                let ty = self
                 .var_to_llvm(var_type, name.clone())?;
                let ptr = self.builder.build_alloca(ty, &name.lexeme)?;
                let result = self.compile_expr(value)?;
                self.builder.build_store(ptr, result)?;
                self.variables.insert(name.lexeme.clone(), (ptr, ty, var_type.clone()));
                Ok(())
            }

            Stmt::Assign { target, value } => {
                let value = self.compile_expr(value)?;
                match &**target {
                    Expr::Variable { name } => {
                        let (ptr, ..) = self.variables.get(&name.lexeme)
                        .ok_or_else(|| HypercError::CompileError {
                            span: expr_span(target),
                            message: "Variable not found.".to_string()
                        })?;
                        self.builder.build_store(*ptr, value)?;
                        Ok(())
                    }

                    Expr::Get { .. } => {
                        let (ptr, _) = self.compile_lvalue(target)?;
                        self.builder.build_store(ptr, value)?;
                        Ok(())
                    }

                    _ => unreachable!()
                }
            }

            Stmt::Block { statements } => {
                for stmt in statements {
                    self.compile_stmt(stmt)?;
                }
                Ok(())
            }

            Stmt::If { 
                condition,
                then_branch,
                else_branch
            } => {
                let cond = self.compile_expr(condition)?;
                let og_block = self.builder.get_insert_block().unwrap();
                let fn_val = og_block.get_parent()
                 .ok_or_else(|| HypercError::CompileError {
                    span: stmt_span(stmt),
                    message: "Unable to get the fn_val for if statement.".to_string()
                })?;

                let then_block = self.context
                 .append_basic_block(fn_val, "then_block");
                let merge_block = self.context
                 .append_basic_block(fn_val, "merge-block");

                match else_branch {
                    Some(b) => {
                        let else_block = self.context
                         .append_basic_block(fn_val, "else-block");
                        self.builder.build_conditional_branch(
                            cond.into_int_value(), 
                            then_block, else_block
                        )?;
                        // * then
                        self.compile_branch(then_block, 
                         then_branch, merge_block)?;
                        // * else
                        self.compile_branch(else_block, b, merge_block)?;
                    }
                    None => {
                        self.builder.build_conditional_branch(
                            cond.into_int_value(), 
                            then_block, merge_block
                        )?;
                        // * then
                        self.compile_branch(then_block, 
                         then_branch, merge_block)?;
                    }
                }
                self.builder.position_at_end(merge_block);
                Ok(())
            }

            Stmt::While { condition, statements } => {
                let og_block = self.builder.get_insert_block().unwrap();
                let fn_val = og_block.get_parent()
                 .ok_or_else(|| HypercError::CompileError {
                    span: stmt_span(stmt),
                    message: "Unable to get the fn_val for while statement.".to_string()
                })?;
                let loop_cond_block = self.context
                 .append_basic_block(fn_val, "loop-cond-block");
                let loop_body_block = self.context
                 .append_basic_block(fn_val, "loop-body-block");
                let after_loop_block = self.context
                 .append_basic_block(fn_val, "after-loop-blocv");

                self.builder.build_unconditional_branch(loop_cond_block)?;
                self.builder.position_at_end(loop_cond_block);
                let cond = self.compile_expr(condition)?;

                self.builder.build_conditional_branch(
                    cond.into_int_value(),
                    loop_body_block, after_loop_block
                )?;
                self.compile_branch(loop_body_block, 
                    statements, loop_cond_block)?;
                self.builder.position_at_end(after_loop_block);
                Ok(())
            }

            Stmt::For {
                initializer,
                condition,
                increment,
                statements
            } => {
                let og_block = self.builder.get_insert_block().unwrap();
                let fn_val = og_block.get_parent()
                 .ok_or_else(|| HypercError::CompileError {
                    span: stmt_span(stmt),
                    message: "Unable to get the fn_val for for statement.".to_string()
                })?;
                let loop_cond_block = self.context
                 .append_basic_block(fn_val, "loop-cond-block");
                let loop_body_block = self.context
                 .append_basic_block(fn_val, "loop-body-block");
                let after_loop_block = self.context
                 .append_basic_block(fn_val, "after-loop-blocv");

                if let Some(init) = initializer {
                    self.compile_stmt(init)?;
                }

                self.builder.build_unconditional_branch(loop_cond_block)?;
                self.builder.position_at_end(loop_cond_block);

                match condition {
                    Some(cond) => {
                        let c = self.compile_expr(cond)?;
                        self.builder.build_conditional_branch(c.into_int_value(), 
                            loop_body_block, after_loop_block)?;
                    }
                    None => {
                        self.builder.build_unconditional_branch(
                            loop_body_block
                        )?;
                    }
                }
                self.builder.position_at_end(loop_body_block);
                self.compile_stmt(statements)?;

                let br = self.builder.get_insert_block()
                 .ok_or_else(|| HypercError::CompileError {
                    span: stmt_span(statements),
                    message: "Builder isnot positioned.".to_string()
                })?;
                let terminator = br.get_terminator();
                
                if terminator.is_none() {
                    if let Some(incr) = increment {
                        self.compile_stmt(incr)?;
                    }
                    self.builder.build_unconditional_branch(loop_cond_block)?;
                }

                self.builder.position_at_end(after_loop_block);
                Ok(())
            }

            Stmt::Func { 
                name, args, 
                statements, return_type 
            } => {
                self.compile_function(name.clone(), args, 
                 statements, return_type.clone())
            }

            Stmt::Return { value, .. } => {
                match value {
                    
                    Some(v) => {
                        let value = self.compile_expr(v)?;
                        self.builder.build_return(Some(&value))?;
                    }

                    None => {
                        self.builder.build_return(None)?;
                    }
                }
                Ok(())
            }

            Stmt::Struct { name, fields } => {
                let st = self.context.opaque_struct_type(&name.lexeme);
                let mut field_types = vec![];
                let mut str_fields = vec![];
                for (tok, vt) in fields {
                    let llvm_ty = self.var_to_llvm(vt, tok.clone())?;
                    field_types.push(llvm_ty);
                    str_fields.push((tok.lexeme.clone(), vt.clone()));
                }
                st.set_body(&field_types, false);
                self.struct_types.insert(name.lexeme.clone(), (st, str_fields));
                Ok(())
            }

            Stmt::Impl { name, methods } => {
                let named = name.clone();
                for method in methods {
                    match method {
                        Stmt::Func { 
                            name, args, 
                            statements, return_type
                        } => {
                            self.compile_method(
                                name.clone(), args, 
                                statements, return_type.clone(), named.clone()
                            )?;
                        }                        
                        _ => unreachable!()
                    }
                }
                Ok(())
            }

            Stmt::Enum { name, variants } => {
                let mut variants_vec = vec![];
                for variant in variants {
                    variants_vec.push(variant.lexeme.clone());
                }
                self.enum_types.insert(name.lexeme.clone(), variants_vec);
                Ok(())
            }
        }
    }

    fn compile_expr(&self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, HypercError> {
        match expr {

            Expr::Binary { left, operator, right } => {
                let mut lhs = self.compile_expr(left)?;
                let mut rhs = self.compile_expr(right)?;
                if lhs.is_int_value() && rhs.is_int_value() {
                    if is_comparison(operator) {
                        Ok(self.compile_int_comparison(
                            lhs.into_int_value(), 
                            operator, 
                            rhs.into_int_value()
                        ).unwrap().into())
                    } else {
                        Ok(self.compile_int_binary(lhs.into_int_value(), operator, rhs.into_int_value()).unwrap().into())
                    }
                } else {
                    if lhs.is_int_value() {
                        lhs = self.builder.build_signed_int_to_float(
                            lhs.into_int_value(), 
                            self.context.f64_type(), 
                            "casttmp"
                        )?.into()
                    }

                    if rhs.is_int_value() {
                        rhs = self.builder.build_signed_int_to_float(
                            rhs.into_int_value(), 
                            self.context.f64_type(), 
                            "casttmp"
                        )?.into()
                    }

                    if is_comparison(operator) {
                        Ok(self.compile_float_comparison(
                            lhs.into_float_value(),
                            operator,
                            rhs.into_float_value()
                        ).unwrap().into())
                    } else {
                        Ok(self.compile_float_binary(
                            lhs.into_float_value(),
                            operator,
                            rhs.into_float_value()
                        ).unwrap().into())
                    }
                }
            }
            
            Expr::Unary { operator, right } => {
                let value = self.compile_expr(right)?;
                match operator.token_type {
                    TokenType::Minus => {
                        if value.is_int_value() {
                            Ok(self.builder.build_int_neg(value.into_int_value(), "ngi").unwrap().into())
                        } else {
                            Ok(self.builder.build_float_neg(value.into_float_value(), "ngf").unwrap().into())
                        }
                    }
                    TokenType::Bang => {
                        Ok(self.builder.build_int_neg(value.into_int_value(), "ngb").unwrap().into())
                    }
                    _ => unreachable!()
                }
            }

            Expr::Call { callee, arguments } => {


                if let Expr::Variable { name } = &**callee {
                    let function = self.module
                     .get_function(&name.lexeme)
                     .ok_or_else(|| HypercError::CompileError {
                        span: expr_span(expr),
                        message: format!("Function '{}' not found.", name.lexeme)
                    })?;
                    let mut args = vec![];
                    for expr in arguments {
                        args.push(self.compile_expr(expr)?.into());
                    }
                    match self.builder.build_call(function, &args, &name.lexeme)?
                     .try_as_basic_value() {
                        ValueKind::Basic(v) => Ok(v),
                        ValueKind::Instruction(_) => {
                            let i64_type = self.context.i64_type();
                            Ok(i64_type.const_zero().into())
                        }
                    }


                } else if let Expr::Get { object, item } = &**callee {
                    let (ptr, vt) = self.compile_lvalue(&object)?;
                    let result = match vt {
                        VarType::Named(tok) => {
                            mangle(&tok.lexeme, &item.lexeme)
                        }

                        _ => return Err(HypercError::CompileError {
                            span: expr_span(expr),
                            message: "Expected a named type.".to_string()
                        })
                    };
                    let function = self.module.get_function(&result)
                     .ok_or_else(|| HypercError::CompileError {
                        span: expr_span(expr),
                        message: "Unknown method.".to_string()
                    })?;
                    let mut args = vec![ptr.into()];
                    for expr in arguments {
                        args.push(self.compile_expr(expr)?.into());
                    }
                    match self.builder.build_call(function, &args, &result)?.try_as_basic_value() {
                        ValueKind::Basic(v) => Ok(v),
                        ValueKind::Instruction(_) => {
                            let i64_type = self.context.i64_type();
                            Ok(i64_type.const_zero().into())
                        }
                    }


                } else if let Expr::Path { type_name, item } = &**callee {
                    match self.struct_types.get(&type_name.lexeme) {
                        Some(_) => {
                            let result = mangle(&type_name.lexeme, &item.lexeme);
                            let function = self.module.get_function(&result)
                             .ok_or_else(|| HypercError::CompileError {
                                span: expr_span(expr),
                                message: "Method not found.".to_string()
                            });
                            let mut args = vec![];
                            for expr in arguments {
                                args.push(self.compile_expr(expr)?.into());
                            }
                            match self.builder.build_call(function?, &args, &item.lexeme)?
                            .try_as_basic_value() {
                                ValueKind::Basic(v) => Ok(v),
                                ValueKind::Instruction(_) => {
                                    let i64_type = self.context.i64_type();
                                    Ok(i64_type.const_zero().into())
                                }
                            }
                        }

                        None => return Err(HypercError::CompileError {
                            span: expr_span(expr),
                            message: "Type not found.".to_string()
                        })
                    }
                } else {
                    return Err(HypercError::CompileError {
                        span: expr_span(expr),
                        message: "Unknown function.".to_string()
                    })
                }
            }

            Expr::Literal { value, span } => {
                match value {
                    LiteralValue::Int(i64_val) => {
                        let i64_type = self.context.i64_type();
                        let value = *i64_val as u64;
                        Ok(i64_type.const_int(value, true).into())
                    }
                    LiteralValue::Float(value) => {
                        let f64_type = self.context.f64_type();
                        Ok(f64_type.const_float(*value).into())
                    }
                    LiteralValue::String(str) => {
                        let str_as_ptr = self.builder.build_global_string_ptr(
                            str, "str"
                        )?;
                        Ok(str_as_ptr.as_pointer_value().into())
                    }
                    LiteralValue::Char(char) => {
                        let char_type = self.context.i8_type();
                        let value = *char as u64;
                        Ok(char_type.const_int(value, false).into())
                    }
                    LiteralValue::Bool(bool) => {
                        let bool_type = self.context.bool_type();
                        let value = *bool as u64;
                        Ok(bool_type.const_int(value, false).into())
                    }

                    _ => return Err(HypercError::CompileError {
                        span: span.clone(),
                        message: format!("Unknown value: {value:?}.")
                    })
                }
            }

            Expr::Grouping { expr } => self.compile_expr(expr),

            Expr::Variable { name } => {
                let (ptr, ty, _ ) = self.variables.get(&name.lexeme)
                 .ok_or_else(|| HypercError::CompileError {
                    span: expr_span(expr),
                    message: "Variable not found.".to_string()
                })?;
                Ok(self.builder.build_load(*ty, *ptr, &name.lexeme)?)
            }

            Expr::StructLit { name, fields } => {
                let (mut agg, names) = match self.struct_types
                 .get(&name.lexeme) {
                    
                    Some((st, items)) => {
                        let ptr = st.get_undef();
                        (ptr, items.clone())
                    }

                    None => return Err(HypercError::CompileError {
                        span: expr_span(expr),
                        message: "Type not found.".to_string()
                    })
                };

                for (tok, expr) in fields {
                    let value = self.compile_expr(expr)?;
                    let pos = names.iter().position(|(n,_)|
                     n == &tok.lexeme).ok_or_else(|| HypercError::CompileError {
                        span: tok.start..tok.end,
                        message: format!("Field '{}' not found.", &tok.lexeme)
                    })?;
                    let position = pos as u32;
                    agg = self.builder.build_insert_value(
                        agg, 
                        value, 
                        position, 
                        &tok.lexeme
                    )?.into_struct_value();
                }
                Ok(agg.into())
            }

            Expr::Get { object, item } => {
                let (ptr, vt) = self.compile_lvalue(object)?;
                let pointee_ty = self.var_to_llvm(&vt, item.clone())?;
                Ok(self.builder.build_load(pointee_ty, ptr, &item.lexeme)?)
            }

            Expr::Path { type_name, item } => {
                let variants = self.enum_types.get(&type_name.lexeme)
                 .ok_or_else(|| HypercError::CompileError {
                    span: expr_span(expr),
                    message: "Enum not found".to_string()
                })?;
                let pos = variants.iter().position(|n| n == &item.lexeme)
                 .ok_or_else(|| HypercError::CompileError {
                    span: item.start..item.end,
                    message: "Variant not found.".to_string()
                })?;
                let position = pos as u64;
                let i64_type = self.context.i64_type();
                Ok(i64_type.const_int(position, false).into())
            }

            Expr::SelfExpr { .. } => return Err(HypercError::CompileError {
                span: expr_span(expr),
                message: "'self' is not a value.".to_string()
            }),

            _ => return Err(HypercError::CompileError {
                span: expr_span(expr),
                message: format!("Unknown expression: {expr:?}.")
            })
        }
    }

    fn compile_branch(&mut self, 
        branch_block: BasicBlock<'ctx>, stmt: &Stmt,
        destination_block: BasicBlock<'ctx>
    ) -> Result<(), HypercError> {
        self.builder.position_at_end(branch_block);
        self.compile_stmt(stmt)?;
        let br = self.builder.get_insert_block()
         .ok_or_else(|| HypercError::CompileError {
            span: stmt_span(stmt),
            message: "Builder is not positioned.".to_string()
        })?;
        let terminator = br.get_terminator();
        if terminator.is_none() {
            self.builder.build_unconditional_branch(destination_block)?;
        }
        Ok(())
    }

    fn compile_function(&mut self, 
        name: Token, args: &Vec<(Token, VarType)>,
        statements: &Stmt,return_type: Option<VarType>
    ) -> Result<(), HypercError> {
        let og_block = self.builder.get_insert_block();
        let og_variables = self.variables.clone();
        let mut arg_types = vec![];
        for (name, vt) in args {
            arg_types.push(self.var_to_llvm(vt, name.clone())?.into());
        }

        let fn_type = match &return_type {
  
            Some(ret) => self.var_to_llvm(&ret, name.clone())?
                     .fn_type(&arg_types, false),
  
            None => self.context.void_type()
                     .fn_type(&arg_types, false)
        };

        let fn_val = self.module
         .add_function(&name.lexeme, fn_type, None);
        let entry_block = self.context
         .append_basic_block(fn_val, "entry-block");
        self.builder.position_at_end(entry_block);
        self.variables = HashMap::new();

        for (i, (tok, vt)) in args.iter().enumerate() {
            let index = i as u32;
            let ty = self.var_to_llvm(vt, tok.clone())?;
            let ptr = self.builder.build_alloca(ty, &tok.lexeme)?;
            let result = fn_val.get_nth_param(index).unwrap();
            self.builder.build_store(ptr, result)?;
            self.variables.insert(tok.lexeme.clone(), (ptr, ty, vt.clone()));
        }

        self.compile_stmt(statements)?;

        let block = self.builder.get_insert_block()
         .ok_or_else(|| HypercError::CompileError {
            span: stmt_span(statements),
            message: "Builder is not positioned.".to_string()
        })?;

        let terminator = block.get_terminator();
        if return_type.is_none() && terminator.is_none() {
            self.builder.build_return(None)?;
        } else if return_type.is_some() && terminator.is_none() {
            self.builder.build_unreachable()?;
        }

        self.variables = og_variables;
        if let Some(og_bb) = og_block {
            self.builder.position_at_end(og_bb);
        }

        Ok(())
    }

    fn compile_method(&mut self, 
        name: Token, args: &Vec<(Token, VarType)>,
        statements: &Stmt,return_type: Option<VarType>, named: Token
    ) -> Result<(), HypercError> {
        let mangled = mangle(&named.lexeme, &name.lexeme);
        let og_block = self.builder.get_insert_block();
        let og_variables = self.variables.clone();
        let ptr = self.context.ptr_type(AddressSpace::default());
        
        let mut arg_types = vec![];
        arg_types.push(ptr.into());
        for (name, vt) in args {
            arg_types.push(self.var_to_llvm(vt, name.clone())?.into());
        }

        let fn_type = match &return_type {
  
            Some(ret) => self.var_to_llvm(&ret, name.clone())?
                     .fn_type(&arg_types, false),
  
            None => self.context.void_type()
                     .fn_type(&arg_types, false)
        };

        let fn_val = self.module
         .add_function(&mangled, fn_type, None);
        let entry_block = self.context
         .append_basic_block(fn_val, "entry-block");
        self.builder.position_at_end(entry_block);
        self.variables = HashMap::new();
        
        let ptr_res = fn_val.get_nth_param(0).unwrap();
        let ptr_val = ptr_res.into_pointer_value();
        self.variables.insert(
            "self".to_string(), 
            (
                ptr_val, 
                self.var_to_llvm(&VarType::Named(named.clone()), name.clone())?, 
                VarType::Named(named.clone())
            )
        );

        for (i, (tok, vt)) in args.iter().enumerate() {
            let index = (i + 1) as u32;
            let ty = self.var_to_llvm(vt, tok.clone())?;
            let ptr = self.builder.build_alloca(ty, &tok.lexeme)?;
            let result = fn_val.get_nth_param(index).unwrap();
            self.builder.build_store(ptr, result)?;
            self.variables.insert(tok.lexeme.clone(), (ptr, ty, vt.clone()));
        }  

        self.compile_stmt(statements)?;

        let block = self.builder.get_insert_block()
         .ok_or_else(|| HypercError::CompileError {
            span: stmt_span(statements),
            message: "Builder is not positioned.".to_string()
        })?;

        let terminator = block.get_terminator();
        if return_type.is_none() && terminator.is_none() {
            self.builder.build_return(None)?;
        } else if return_type.is_some() && terminator.is_none() {
            self.builder.build_unreachable()?;
        }

        self.variables = og_variables;
        if let Some(og_bb) = og_block {
            self.builder.position_at_end(og_bb);
        }

        Ok(())
    }

    fn compile_int_binary(&self, 
        lhs: IntValue<'ctx>, op: &Token, rhs: IntValue<'ctx>
    ) -> Result<IntValue<'ctx>, HypercError> {
        match op.token_type {
            
            TokenType::Plus => {
                Ok(self.builder.build_int_add(lhs, rhs, "add")?)
            }

            TokenType::Minus => {
                Ok(self.builder.build_int_sub(lhs, rhs, "sub")?)
            }
            
            TokenType::Star => {
                Ok(self.builder.build_int_mul(lhs, rhs, "mut")?)
            }

            TokenType::Slash => {
                Ok(self.builder.build_int_signed_div(lhs, rhs, "div")?)
            }

            TokenType::Percent => {
                Ok(self.builder.build_int_signed_rem(lhs, rhs, "rem")?)
            }

            _ => unreachable!()
        }
    }

    fn compile_int_comparison(&self,
        lhs: IntValue<'ctx>, op: &Token, rhs: IntValue<'ctx>
    ) -> Result<IntValue<'ctx>, HypercError> {
        match op.token_type {

            TokenType::Less         => {
                Ok(self.builder.build_int_compare(
                    inkwell::IntPredicate::SLT, lhs, rhs, "sge"
                )?)
            }

            TokenType::Greater      => {
                Ok(self.builder.build_int_compare(
                    inkwell::IntPredicate::SGT, lhs, rhs, "sgt"
                )?)
            }

            TokenType::LessEqual    => {
                Ok(self.builder.build_int_compare(
                    inkwell::IntPredicate::SLE, lhs, rhs, "sle"
                )?)
            }
            TokenType::GreaterEqual => {
                Ok(self.builder.build_int_compare(
                    inkwell::IntPredicate::SGT, lhs, rhs, "sgt"
                )?)
            }

            TokenType::EqualEqual   => {
                Ok(self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ, lhs, rhs, "eq"
                )?)
            }

            TokenType::BangEqual    => {
                Ok(self.builder.build_int_compare(
                    inkwell::IntPredicate::NE, lhs, rhs, "ne"
                )?)
            }

            _ => unreachable!()
        }
    }

    fn compile_float_binary(&self, 
        lhs: FloatValue<'ctx>, op: &Token, rhs: FloatValue<'ctx>
    ) -> Result<FloatValue<'ctx>, HypercError> {
        match op.token_type {
            
            TokenType::Plus => {
                Ok(self.builder.build_float_add(lhs, rhs, "add")?)
            }

            TokenType::Minus => {
                Ok(self.builder.build_float_sub(lhs, rhs, "sub")?)
            }
            
            TokenType::Star => {
                Ok(self.builder.build_float_mul(lhs, rhs, "mut")?)
            }

            TokenType::Slash => {
                Ok(self.builder.build_float_div(lhs, rhs, "div")?)
            }

            _ => unreachable!()
        }
    }

    fn compile_float_comparison(&self,
        lhs: FloatValue<'ctx>, op: &Token, rhs: FloatValue<'ctx>
    ) -> Result<IntValue<'ctx>, HypercError> {
        match op.token_type {

            TokenType::Less         => {
                Ok(self.builder.build_float_compare(
                    inkwell::FloatPredicate::OLT, lhs, rhs, "oge"
                )?)
            }

            TokenType::Greater      => {
                Ok(self.builder.build_float_compare(
                    inkwell::FloatPredicate::OGT, lhs, rhs, "ogt"
                )?)
            }

            TokenType::LessEqual    => {
                Ok(self.builder.build_float_compare(
                    inkwell::FloatPredicate::OLE, lhs, rhs, "ole"
                )?)
            }
            TokenType::GreaterEqual => {
                Ok(self.builder.build_float_compare(
                    inkwell::FloatPredicate::OGT, lhs, rhs, "ogt"
                )?)
            }

            TokenType::EqualEqual   => {
                Ok(self.builder.build_float_compare(
                    inkwell::FloatPredicate::OEQ, lhs, rhs, "oeq"
                )?)
            }

            TokenType::BangEqual    => {
                Ok(self.builder.build_float_compare(
                    inkwell::FloatPredicate::ONE, lhs, rhs, "one"
                )?)
            }

            _ => unreachable!()
        }
    }

    fn var_to_llvm(&self, vt: &VarType, tok: Token) -> Result<BasicTypeEnum<'ctx>, HypercError> {
        match vt {

            VarType::Int    => Ok(self.context.i64_type().into()),

            VarType::Float  => Ok(self.context.f64_type().into()),

            // todo: VarType::Str    => {}

            VarType::Char   => Ok(self.context.i8_type().into()),

            VarType::Bool   => Ok(self.context.bool_type().into()),

            VarType::Named(tok) => {
                match self.struct_types.get(&tok.lexeme) {

                    Some((st, _)) => {
                        let struct_type = *st;
                        Ok(struct_type.into())
                    },

                    None => return Err(HypercError::CompileError {
                        span: tok.start..tok.end,
                        message: "Type not found.".to_string()
                    })
                }
            }

            _ => return Err(HypercError::CompileError {
                span: tok.start..tok.end,
                message: "Type not found or is not supported yet.".to_string()
            })
        }
    }

    fn compile_lvalue(&self, expr: &Expr
    ) -> Result<(PointerValue<'ctx>, VarType), HypercError> {
        match expr {
    
            Expr::Variable { name } => {
                let (ptr, _, vt) = self.variables.get(&name.lexeme)
                 .ok_or_else(|| HypercError::CompileError {
                    span: expr_span(expr),
                    message: "Vatiable not found.".to_string()
                })?;
                Ok((*ptr, vt.clone()))
            }
    
            Expr::Get { object, item } => {
                let (obj_ptr, obj_vt) = self.compile_lvalue(object)?;
                
                match obj_vt {
                    
                    VarType::Named(tok) => {
                        match self.struct_types.get(&tok.lexeme) {
                            
                            Some((st, items)) => {
                                let pos = items.iter().position(|(n,_)|
                                 n == &item.lexeme).ok_or_else(|| HypercError::CompileError {
                                    span: expr_span(expr),
                                    message: "Item not found.".to_string()
                                })?;
                                let position = pos as u32;
                                let item_ptr = self.builder.build_struct_gep(
                                    *st,
                                    obj_ptr,
                                    position,
                                    &item.lexeme
                                )?;
                                let (_, item_vt) = items[pos].clone();
                                Ok((item_ptr, item_vt))
                            }
                            
                            None => return Err(HypercError::CompileError {
                                span: expr_span(expr),
                                message: "Type not found.".to_string()
                            })
                        }
                    }

                    _ => return Err(HypercError::CompileError {
                        span: expr_span(expr),
                        message: "This type has no fields or methods.".to_string()
                    })
                }
            }
    
            Expr::SelfExpr { .. } => {
                let (ptr, _, vt) = self.variables.get("self").ok_or_else(||
                 HypercError::CompileError {
                    span: expr_span(expr),
                    message: "'self' is outside of a method.".to_string()
                })?;
                Ok((*ptr, vt.clone()))
            }
    
            _ => return Err(HypercError::CompileError {
                span: expr_span(expr),
                message: "Wrong expression to compile left value.".to_string()
            })
        }




    }

    fn emit_obj(&self, path: &str, out: &str) -> Result<(), HypercError> {
        match Target::initialize_native(&InitializationConfig::default()) {
            Ok(_) => {}
            Err(e) => return Err(HypercError::BuildError {
                message: e.to_string()
            })
        }
        let triple = TargetMachine::get_default_triple();
        let target = match Target::from_triple(&triple) {
            Ok(t) => t,
            Err(e) => return Err(HypercError::BuildError {
                message: e.to_string()
            })
        };
        let cpu = "generic";
        let features = "";
        let target_machine = target.create_target_machine(
            &triple,
            cpu,
            features,
            OptimizationLevel::None,
            RelocMode::PIC,
            CodeModel::Default,
        ).ok_or_else(|| HypercError::BuildError {
            message: "TargetMachine creation failed.".to_string()
        })?;
        let out_path = Path::new(path).parent().unwrap_or(Path::new("."));
        let obj_path = out_path.join(format!("{}.o", out));
        let bin_path = out_path.join(out);
        let file_type = FileType::Object;

        // todo: assembly: .s

        target_machine.write_to_file(&self.module, file_type, &obj_path)
         .map_err(|e| HypercError::BuildError {
            message: e.to_string()
        })?;
        let mut cmd = Command::new("cc");
        cmd.args([
            &obj_path.to_str().unwrap(),
            "-o",
            &bin_path.to_str().unwrap()
        ]);
        match cmd.status() {
            Ok(status) => {
                if status.success() {
                    println!("Compiled to: {}\n", &bin_path.display());
                    return Ok(());
                } else {
                    return Err(HypercError::BuildError {
                        message: format!(
                            "Autolinking failed with exit code: {:?}", 
                            status.code()
                        )
                    })
                }
            }
            Err(e) => return Err(HypercError::BuildError {
                message: format!("Autolinking failed with code: {:?} on {}.", 
                    e, &obj_path.display()
                )
            })
        }
    }
}