use crate::{ast::{Expr, LiteralValue, Stmt, VarType}, lexer, token::TokenType};
use std::{path::Path};
use std::process::Command;
use std::collections::HashMap;
use crate::error::CompileError;
use crate::token::Token;
use inkwell::{
    AddressSpace, OptimizationLevel, basic_block::BasicBlock, builder::{Builder, BuilderError}, context::Context, module::Module, targets::{
        CodeModel, 
        FileType, 
        InitializationConfig, 
        RelocMode, 
        Target, 
        TargetMachine,
    }, types::{
        BasicMetadataTypeEnum, BasicType, BasicTypeEnum, StructType
    }, values::{
        AggregateValueEnum, BasicMetadataValueEnum, BasicValueEnum, FloatValue, IntValue, PointerValue, ValueKind,
    },
};

impl From<BuilderError> for CompileError {
    fn from(err: BuilderError) -> Self {
        return CompileError{
            span: 0..0,
            message: err.to_string(),
        }
    }
}

pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>, VarType)>,
    struct_types: HashMap<String, (StructType<'ctx>, Vec<(String, VarType)>)>,
}

impl <'ctx>Codegen<'ctx> {
    
    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("module");
        let builder = context.create_builder();
        let variables = HashMap::new();
        let struct_types = HashMap::new();
        Codegen { context, module, builder, variables, struct_types }
    }

    pub fn compile(&mut self, stmts: &[Stmt], path: &str, out_name: &str) -> Result<(), CompileError> {
        for stmt in stmts {
            self.compile_stmt(stmt)?;
        }
        let result = self.module.print_to_string().to_string();
        println!("{}", result);
        match self.module.verify() {
            Ok(()) => {}
            Err(e) => {
                return Err(CompileError { 
                    span: 0..0, 
                    message: e.to_string() 
                });
            }
        }
        self.emit_obj(path, out_name)?;
        Ok(())
    }

    fn compile_expr(&self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, CompileError> {
        match expr {
            Expr::Literal { value, .. } => {
                match value {
                    LiteralValue::Int(n) => {
                        let i64_type = self.context.i64_type();
                        let value_i64 = *n as u64;
                        Ok(i64_type.const_int(value_i64, true).into())
                    }
                    LiteralValue::Float(f) => {
                        let f64_type = self.context.f64_type();
                        Ok(f64_type.const_float(*f).into())
                    }
                    LiteralValue::String(s) => {
                        let str = self.builder.build_global_string_ptr(s, "str")?; 
                        Ok(str.as_pointer_value().into())
                    }
                    LiteralValue::Char(c) => {
                        let char_type = self.context.i8_type();
                        let value_i8 = *c as u64;
                        Ok(char_type.const_int(value_i8, false).into())
                    }
                    LiteralValue::Bool(b) => {
                        let bool_type = self.context.bool_type();
                        let value_bool = *b as u64;
                        Ok(bool_type.const_int(value_bool, false).into())
                    }
                    // todo LiteralValue::Null() => {}
                    _ => return Err(CompileError { 
                        span: 0..0, 
                        message: format!("Literal: {:?}, is not supported in v0.2.", value) 
                    })
                }
            },

            Expr::Binary { left, operator, right } => {
                let mut lhs = self.compile_expr(left)?;
                let mut rhs = self.compile_expr(right)?;
                if lhs.is_int_value() && rhs.is_int_value() {
                    if Self::is_comparison(operator) {
                        Ok(self.compile_int_comparison(lhs.into_int_value(), operator, rhs.into_int_value())?.into())
                    } else {
                        Ok(self.compile_int_binary(lhs.into_int_value(), operator, rhs.into_int_value())?.into())
                    }
                } else {
                    if lhs.is_int_value() {
                        lhs = self.builder.build_signed_int_to_float(
                            lhs.into_int_value(), 
                            self.context.f64_type(), 
                            "casttmp"
                        )?.into();
                    }

                    if rhs.is_int_value() {
                        rhs = self.builder.build_signed_int_to_float(
                            rhs.into_int_value(), 
                            self.context.f64_type(), 
                            "casttmp"
                        )?.into();
                    }

                    if Self::is_comparison(operator) {
                        Ok(self.compile_float_comparison(lhs.into_float_value(), operator, rhs.into_float_value())?.into())
                    } else {
                        Ok(self.compile_float_binary(lhs.into_float_value(), operator, rhs.into_float_value())?.into())
                    }
                }
            }

            Expr::Unary { operator, right } => {
                let result = self.compile_expr(right)?;
                match operator.token_type {
                    TokenType::Minus => {
                        if result.is_int_value() {
                            Ok(self.builder.build_int_neg(result.into_int_value(), "ngi").unwrap().into())
                        } else {
                            Ok(self.builder.build_float_neg(result.into_float_value(), "ngf").unwrap().into())
                        }
                    }

                    TokenType::Bang => {
                        Ok(self.builder.build_not(result.into_int_value(), "unb").unwrap().into())
                    }

                    _ => Err( CompileError { 
                        span: operator.start..operator.end, 
                        message: "This unary operator is not supported in v0.1.".to_string() 
                    })
                    
                }
            }

            Expr::Grouping { expr } => {
                Ok(self.compile_expr(expr)?)
            }

            Expr::Call { callee, arguments, .. } => {
                if let Expr::Variable { name } = &**callee  {
                    let function = self.module.get_function(&name.lexeme).ok_or_else(|| CompileError{
                        span: name.start..name.end,
                        message: format!("Unknown function: {:?}", &name.lexeme)
                    });
                    let mut arg_vec: Vec<BasicMetadataValueEnum> = vec![];
                    for expression in arguments {
                        let result = self.compile_expr(expression)?.into();
                        arg_vec.push(result);
                    }

                    match self.builder.build_call(function?, &arg_vec, &name.lexeme)?.try_as_basic_value() {
                        ValueKind::Basic(v) => {Ok(v)},
                        ValueKind::Instruction(_) => {
                            let i64_type = self.context.i64_type();
                            Ok(i64_type.const_zero().into())
                        }
                    }
                } else {
                    return Err(CompileError { 
                        span: 0..0, 
                        message: format!("Only simple function calls are supported: {:?}.", callee) 
                    });
                }
            }

            Expr::Variable { name } => {
                let (ptr, ty, _) = *self.variables.get(&name.lexeme).ok_or_else(|| CompileError{
                    span: name.start..name.end,
                    message: format!("Variable undefined: {:?}.", &name.lexeme)
                })?;
                Ok(self.builder.build_load(ty, ptr, &name.lexeme)?)
            }

            Expr::StructLit { name, fields } => {
                let (mut agg, names) = match self.struct_types.get(&name.lexeme) {
                    Some((ty, vec)) => {
                        let agg = ty.get_undef();
                        let names = vec.clone();
                        (agg, names)
                    }
                    None => return Err(CompileError { 
                        span: name.start..name.end,
                        message: "Struct not found.".to_string()
                    })
                };

                for (tok, expr) in fields {
                    let value = self.compile_expr(expr)?;
                    let pos = names.iter().position(|(n,_)| n == &tok.lexeme).ok_or_else(|| CompileError {
                        span: tok.start..tok.end,
                        message: format!("Cannot set position to a field {}", tok.lexeme)
                    });
                    let position = pos? as u32;
                    agg = self.builder.build_insert_value(agg, value, position, &tok.lexeme)?.into_struct_value();
                }
                Ok(agg.into())
            }

            Expr::Get { object, field } => {
                let (ptr, vt) = self.compile_lvalue(expr)?;
                let ty = self.var_to_llvm(&vt)?;
                Ok(self.builder.build_load(ty, ptr, &field.lexeme)?)
            }

            Expr::SelfExpr { self_tok } => {
                Err(CompileError { 
                    span: self_tok.start..self_tok.end, 
                    message: "Methods are not supported yet.".to_string() 
                })
            }

            _ => todo!()
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            
            Stmt::Expression { value } => {
                self.compile_expr(value)?;
                Ok(())
            }

            Stmt::Print { value } => { // todo type printing
                let i32_type = self.context.i32_type();
                let ptr = self.context.ptr_type(AddressSpace::default());
                let fn_type = i32_type.fn_type(&[ptr.into()], true);

                let print_fn = match self.module.get_function("printf") {
                    Some(printf) => {printf}
                    None => {
                        self.module.add_function("printf", fn_type, None)
                    }
                };

                let result = self.compile_expr(value)?;                
                let fmt_str = if result.is_pointer_value() {
                    "%s\n"
                } else if result.is_float_value() {
                    "%f\n"
                } else if result.into_int_value().get_type().get_bit_width() == 8 {
                    "%c\n"
                } else {
                    "%ld\n"
                };
                let fmt = self.builder.build_global_string_ptr(fmt_str, "fmt")?;
                self.builder.build_call(print_fn, &[fmt.as_pointer_value().into(), result.into()], "print")?;
                Ok(())
            }

            Stmt::Let { name, value, var_type, .. } => {
                let ty = self.var_to_llvm(var_type)?;
                let ptr = self.builder.build_alloca(ty, &name.lexeme)?;
                let result = self.compile_expr(value)?;
                self.builder.build_store(ptr, result)?;
                self.variables.insert(name.lexeme.clone(), (ptr, ty, var_type.clone()));
                Ok(())
            }

            Stmt::Assign { target, value } => {
                match &**target {
                    Expr::Variable { name } => {
                        let (ptr, ..) = *self.variables.get(&name.lexeme).ok_or_else(|| CompileError{
                            span: name.start..name.end,
                            message: "Failed getting a pointer for assign statement.".to_string()
                        })?;
                        let value = self.compile_expr(value)?;
                        self.builder.build_store(ptr, value)?;
                        Ok(())
                    }

                    Expr::Get { .. } => {
                        let (ptr, _) = self.compile_lvalue(target)?;
                        let value = self.compile_expr(value)?;
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

            Stmt::If { params, then_branch, else_branch } => {
                let c = self.compile_expr(params)?;
                let og_block = self.builder.get_insert_block();
                let fn_val = og_block.unwrap().get_parent().ok_or_else(|| CompileError {
                    span: 0..0, 
                    message: "Cannot get the fn_val for if statement.".to_string()
                })?;

                let then_block = self.context.append_basic_block(fn_val, "then_block");
                let merge = self.context.append_basic_block(fn_val, "merge");

                match else_branch {
                    Some(b) => {
                        let else_block = self.context.append_basic_block(fn_val, "else_block");
                        self.builder.build_conditional_branch(c.into_int_value(), then_block, else_block)?;
                        // * then
                        self.compile_branch(then_block, then_branch, merge)?;
                        // * else
                        self.compile_branch(else_block, b, merge)?;
                    }
                    None => {
                        self.builder.build_conditional_branch(c.into_int_value(), then_block, merge)?;
                        // * then
                        self.compile_branch(then_block, then_branch, merge)?;
                    }
                }

                self.builder.position_at_end(merge);
                Ok(())
            }

            Stmt::While { conditions, statements } => {
                let og_block = self.builder.get_insert_block();
                let fn_val = og_block.unwrap().get_parent().ok_or_else(|| CompileError {
                    span: 0..0,
                    message: "Cannot get the fn_val for while statement.".to_string()
                })?;
                let loop_cond = self.context.append_basic_block(fn_val, "loop_cond");
                let loop_body = self.context.append_basic_block(fn_val, "loop_body");
                let after = self.context.append_basic_block(fn_val, "after");

                self.builder.build_unconditional_branch(loop_cond)?;
                self.builder.position_at_end(loop_cond);
                let c = self.compile_expr(conditions)?;

                self.builder.build_conditional_branch(c.into_int_value(), loop_body, after)?;

                self.compile_branch(loop_body, statements, loop_cond)?;
                self.builder.position_at_end(after);

                Ok(())
            }

            Stmt::For { initializer, condition, 
            increment, statements } => {
                let og_block = self.builder.get_insert_block();
                let fn_val = og_block.unwrap().get_parent().ok_or_else(|| CompileError {
                    span: 0..0,
                    message: "Cannot get the fn_val for while statement.".to_string()
                })?;
                let loop_cond = self.context.append_basic_block(fn_val, "loop_cond");
                let loop_body = self.context.append_basic_block(fn_val, "loop_body");
                let after = self.context.append_basic_block(fn_val, "after");

                if let Some(init) = initializer {
                    self.compile_stmt(init)?;
                }

                self.builder.build_unconditional_branch(loop_cond)?;
                self.builder.position_at_end(loop_cond);
                
                match condition {
                    Some(cond) => { 
                        let c = self.compile_expr(cond)?;
                        self.builder.build_conditional_branch(c.into_int_value(), loop_body, after)?;
                    }
                    None => { self.builder.build_unconditional_branch(loop_body)?; }
                }
                self.builder.position_at_end(loop_body);
                self.compile_stmt(statements)?;

                let br = self.builder.get_insert_block().ok_or_else(|| CompileError{
                    span: 0..0,
                    message: "Builder is not positioned.".to_string()
                })?;
                let terminator = br.get_terminator();
                if terminator.is_none() {
                    if let Some(incr) = increment {
                        self.compile_stmt(incr)?;
                    }
                    self.builder.build_unconditional_branch(loop_cond)?;
                }

                self.builder.position_at_end(after);
                Ok(())
            }

            Stmt::Return { value } => {
                match value {
                    Some(v) => {
                        let result = self.compile_expr(v)?;
                        self.builder.build_return(Some(&result))?;
                    }
                    None => {
                        self.builder.build_return(None)?;
                    }
                }
                Ok(())
            }

            Stmt::Struct { name, fields } => {
                let s_ty = self.context.opaque_struct_type(&name.lexeme);   
                let mut field_types: Vec<BasicTypeEnum> = vec![];
                let mut field_vec: Vec<(String, VarType)> = vec![];
                for (tok, vt) in fields {
                    let llvm_ty = self.var_to_llvm(&vt)?;
                    field_types.push(llvm_ty);
                    field_vec.push((tok.lexeme.clone(), vt.clone()));
                }
                s_ty.set_body(&field_types, false);
                self.struct_types.insert(name.lexeme.clone(), (s_ty, field_vec));
                Ok(())
            }

            Stmt::Function { name, params, statements, return_type } => {
                self.compile_function(name, params, statements, return_type)?;
                Ok(())
            }

            _ => {Err(CompileError { 
                span: 0..0, 
                message: format!("Statement: {:?}, is not supported in v0.1.", stmt)  
            })}
        }
    }

    fn compile_function(&mut self, name: &Token, params: &Vec<(Token, VarType)>, stmts: &Stmt, return_type: &Option<VarType> ) -> Result<(), CompileError> {
        let og_block = self.builder.get_insert_block();
        let og_variables = self.variables.clone();
        let mut param_types: Vec<BasicMetadataTypeEnum> = vec![];
        for (_, var_type) in params {
            param_types.push(self.var_to_llvm(var_type)?.into());
        }

        let fn_type = match return_type {
            Some(r) => {
                 self.var_to_llvm( r)?.fn_type(&param_types, false)
            }
            None => {
                self.context.void_type().fn_type(&param_types, false)
            }
        };

        let fn_val = self.module.add_function(&name.lexeme, fn_type, None);
        let basic_block = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(basic_block);
        self.variables = HashMap::new();

        for (i, (name, var_type)) in params.iter().enumerate() {
            let index = i as u32;
            let ty = self.var_to_llvm(&var_type)?;
            let ptr = self.builder.build_alloca(ty, &name.lexeme)?;
            let result = fn_val.get_nth_param(index).unwrap();
            self.builder.build_store(ptr, result)?;
            self.variables.insert(name.lexeme.clone(), (ptr, ty, var_type.clone()));
        }

        self.compile_stmt(stmts)?;

        let block= self.builder.get_insert_block().ok_or_else(|| CompileError{
            span: 0..0,
            message: "Builder is not positioned.".to_string()
        })?;
        let terminator = block.get_terminator();
        if return_type.is_none() && terminator.is_none() {
            self.builder.build_return(None)?;
        }
        if return_type.is_some() && terminator.is_none() {
            self.builder.build_unreachable()?;
        }
        self.variables = og_variables;
        if let Some(b) = og_block {
            self.builder.position_at_end(b);
        };
        Ok(())
    }

    fn compile_branch(&mut self, basic_block: BasicBlock<'ctx>, 
    branch: &Stmt, destination_block: BasicBlock<'ctx>) -> Result<(), CompileError> {
        self.builder.position_at_end(basic_block);
        self.compile_stmt(branch)?;
        let br = self.builder.get_insert_block().ok_or_else(|| CompileError{
            span: 0..0,
            message: "Builder is not positioned.".to_string()
        })?;
        let terminator = br.get_terminator();
        if terminator.is_none() {
            self.builder.build_unconditional_branch(destination_block)?;
        }

        Ok(())
    }

    fn is_comparison(op: &Token) -> bool {
        match op.token_type {
            TokenType::Less |
            TokenType::LessEqual |
            TokenType::Greater |
            TokenType::GreaterEqual |
            TokenType::BangEqual |
            TokenType::EqualEqual => {true}
            
            _ => false
        }
    }

    fn compile_int_binary(&self, lhs: IntValue<'ctx>, op: &Token, rhs: IntValue<'ctx>) -> Result<IntValue<'ctx>, CompileError> {
        match op.token_type {
            
            TokenType::Plus => {
                Ok(self.builder.build_int_add(lhs, rhs, "add")?)
            }

            TokenType::Minus => {
                Ok(self.builder.build_int_sub(lhs, rhs, "sub")?)
            }

            TokenType::Star => {
                Ok(self.builder.build_int_mul(lhs, rhs, "mul")?)
            }

            TokenType::Slash => {
                Ok(self.builder.build_int_signed_div(lhs, rhs, "div")?)
            }

            TokenType::Percent => {
                Ok(self.builder.build_int_signed_rem(lhs, rhs, "rem")?)
            }

            _ => return Err(CompileError { 
                        span: op.start..op.end, 
                        message: "This binary expression is not supported in v0.1.".to_string() 
                    })
        }
    }

    fn compile_int_comparison(&self, lhs: IntValue<'ctx>, op: &Token, rhs: IntValue<'ctx>) -> Result<IntValue<'ctx>, CompileError> {
        match op.token_type {
            TokenType::Less => {
                Ok(self.builder.build_int_compare(inkwell::IntPredicate::SLT, lhs, rhs, "slt")?)
            }

            TokenType::LessEqual => {
                Ok(self.builder.build_int_compare(inkwell::IntPredicate::SLE, lhs, rhs, "sle")?)
            }

            TokenType::Greater => {
                Ok(self.builder.build_int_compare(inkwell::IntPredicate::SGT, lhs, rhs, "sgt")?)
            }

            TokenType::GreaterEqual => {
                Ok(self.builder.build_int_compare(inkwell::IntPredicate::SGE, lhs, rhs, "sge")?)
            }

            TokenType::BangEqual => {
                Ok(self.builder.build_int_compare(inkwell::IntPredicate::NE, lhs, rhs, "ne")?)
            }   

            TokenType::EqualEqual => {
                Ok(self.builder.build_int_compare(inkwell::IntPredicate::EQ, lhs, rhs, "eq")?)
            }

            _ => return Err(CompileError { 
                        span: op.start..op.end, 
                        message: "This int comparison expression is not supported in v0.1.".to_string() 
                    })
        }
    }

    fn compile_float_binary(&self, lhs: FloatValue<'ctx>, op: &Token, rhs: FloatValue<'ctx>) -> Result<FloatValue<'ctx>, CompileError> {
        match op.token_type {
            
            TokenType::Plus => {
                Ok(self.builder.build_float_add(lhs, rhs, "add")?)
            }

            TokenType::Minus => {
                Ok(self.builder.build_float_sub(lhs, rhs, "sub")?)
            }

            TokenType::Star => {
                Ok(self.builder.build_float_mul(lhs, rhs, "mul")?)
            }

            TokenType::Slash => {
                Ok(self.builder.build_float_div(lhs, rhs, "div")?)
            }

            _ => return Err(CompileError { 
                        span: op.start..op.end, 
                        message: "This float binary expression is not supported in v0.1.".to_string() 
                    })
        }
    }

    fn compile_float_comparison(&self, lhs: FloatValue<'ctx>, op: &Token, rhs: FloatValue<'ctx>) -> Result<IntValue<'ctx>, CompileError> {
        match op.token_type {
            TokenType::Less => {
                    Ok(self.builder.build_float_compare(inkwell::FloatPredicate::OLT, lhs, rhs, "olt")?)
                }

                TokenType::LessEqual => {
                    Ok(self.builder.build_float_compare(inkwell::FloatPredicate::OLE, lhs, rhs, "ole")?)
                }

                TokenType::Greater => {
                    Ok(self.builder.build_float_compare(inkwell::FloatPredicate::OGT, lhs, rhs, "ogt")?)
                }

                TokenType::GreaterEqual => {
                    Ok(self.builder.build_float_compare(inkwell::FloatPredicate::OGE, lhs, rhs, "oge")?)
                }

                TokenType::BangEqual => {
                    Ok(self.builder.build_float_compare(inkwell::FloatPredicate::ONE, lhs, rhs, "one")?)
                }   

                TokenType::EqualEqual => {
                    Ok(self.builder.build_float_compare(inkwell::FloatPredicate::OEQ, lhs, rhs, "oeq")?)
                }

                _ => return Err(CompileError { 
                            span: op.start..op.end, 
                            message: "This float comparison expression is not supported in v0.1.".to_string() 
                        })
        }
    }

    fn var_to_llvm(&self, var_type: &VarType) -> Result<BasicTypeEnum<'ctx>, CompileError> {
        match var_type {
            VarType::Int => {
                Ok(self.context.i64_type().into())
            },
            VarType::Float => {
                Ok(self.context.f64_type().into())
            },
            // todo VarType::Str => {},
            VarType::Char => {
                Ok(self.context.i8_type().into())
            },
            VarType::Bool => {
                Ok(self.context.bool_type().into())
            },
            VarType::Named(tok) => {
                match self.struct_types.get(&tok.lexeme) {
                    Some((sty, _)) => {
                        let struct_type = *sty;
                        Ok(struct_type.into())
                    }
                    None => return Err(CompileError { 
                        span: tok.start..tok.end, 
                        message: "Unknown type.".to_string() 
                    })  
                }
            }
            _ => return Err(CompileError { 
                span: 0..0, 
                message: "VarType to LLVM conversion failed.".to_string() 
            })
        }
    }

    fn compile_lvalue(&self, expr: &Expr) -> Result<(PointerValue<'ctx>, VarType), CompileError> {
        match expr {
            Expr::Variable { name } => {
                let (ptr, _, vt) = self.variables.get(&name.lexeme).ok_or_else(|| CompileError{
                    span: name.start..name.end,
                    message: "Variable undefined.".to_string()
                })?;
                Ok((*ptr, vt.clone()))
            }

            Expr::Get { object, field } => {
                let (obj_ptr, obj_vt) = self.compile_lvalue(object)?;

                match obj_vt {
                    VarType::Named(obj_tok) => {
                        match self.struct_types.get(&obj_tok.lexeme) {
                            Some((st, fields)) => {
                                let pos = fields.iter().position(|(n,_)| n == &field.lexeme).ok_or_else(|| CompileError {
                                        span: field.start..field.end,
                                        message: "Unknown field.".to_string()
                                })?;
                                let position = pos as u32;
                                let field_ptr = self.builder.build_struct_gep(*st, obj_ptr, position, &field.lexeme)?;
                                let field_vt = fields[pos].1.clone();
                                Ok((field_ptr, field_vt))
                            }

                            None => {
                                return Err(CompileError { 
                                    span: obj_tok.start..obj_tok.end, 
                                    message: "Unknown type.".to_string() 
                                })
                            }
                        }
                    }
                    _ => {
                        return Err(CompileError { 
                            span: field.start..field.end, 
                            message: "This type has no fields.".to_string() 
                        })
                    }
                }
            }

            Expr::SelfExpr { self_tok } => {
                Err(CompileError { 
                    span: self_tok.start..self_tok.end, 
                    message: "Methods are not supported yet.".to_string() 
                })
            }

            _ => {
                return Err(CompileError { 
                    span: 0..0, 
                    message: "Wrong expression to compile left value.".to_string() 
                })
            }
        }
    }

    fn emit_obj(&self, path: &str, out_name: &str) -> Result<(), CompileError> {
        match Target::initialize_native(&InitializationConfig::default()) {
            Ok(()) => {}
            Err(e) => {
                return Err(CompileError { 
                        span: 0..0, 
                        message: e.to_string() 
                    })
            }
        }
        let default_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&default_triple).map_err(|e| CompileError{
            span: 0..0,
            message: e.to_string()
        })?;
        let target_machine = target.create_target_machine(
            &default_triple, 
            "generic", 
            "", 
            OptimizationLevel::None,
            RelocMode::PIC,
            CodeModel::Default,
        ).ok_or_else(|| CompileError{
            span: 0..0,
            message: "Target machine creation failed.".to_string()
        })?;
        let out_path = Path::new(path).parent().unwrap_or(Path::new("."));
        let obj_path = out_path.join(format!("{}.o", out_name));
        let exe_path = out_path.join(out_name);

        target_machine.write_to_file(&self.module, FileType::Object, &obj_path).map_err(|e| CompileError{
            span: 0..0,
            message: e.to_string()
        })?;
        let mut cc = Command::new("cc");
        cc.arg(&obj_path);
        cc.arg("-o");
        cc.arg(&exe_path);
        match cc.status() {
            Ok(status) => {
                if status.success() {
                    println!("Compiled to: {}", &exe_path.display());
                    return Ok(())
                } else {
                    return Err(CompileError {
                        span: 0..0, 
                        message: format!("Autolinking failed: {:?}", status.code()) 
                    })
                }
                
            },
            Err(e) => { return Err(CompileError {
                span: 0..0, 
                message: format!("Autolinking failed: {:?} on {}", e, &obj_path.display()) 
            })}
        }
    }
}