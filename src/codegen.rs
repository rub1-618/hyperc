use core::panic;
use crate::{ast::{Expr, LiteralValue, Stmt, VarType}, checker::Type, token::TokenType};
use std::{ops::Deref, path::Path};
use std::collections::HashMap;
use inkwell::{
    OptimizationLevel, builder::Builder, context::Context, module::Module, targets::{
        CodeModel, 
        FileType, 
        InitializationConfig, 
        RelocMode, 
        Target, 
        TargetMachine,
    }, types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum}, values::{
        BasicValueEnum, 
        FloatValue, 
        IntValue,
        PointerValue,
    }
};

use crate::token::Token;

pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>
}

impl <'ctx>Codegen<'ctx> {
    
    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("module");
        let builder = context.create_builder();
        let variables = HashMap::new();
        Codegen { context, module, builder, variables }
    }

    pub fn compile(&mut self, stmts: &[Stmt]) {
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");

        self.builder.position_at_end(basic_block);

        for stmt in stmts {
            self.compile_stmt(stmt);
        }

        let value = i64_type.const_int(0, false);
        self.builder.build_return(Some(&value)).unwrap();

        let result = self.module.print_to_string().to_string();
        println!("{}", result);
        match self.module.verify() {
            Ok(()) => {}
            Err(e) => {
                panic!("Error: {}", &e)
            }
        }

        self.emit_obj();
    }

    fn compile_expr(&self, expr: &Expr) -> BasicValueEnum<'ctx> {
        match expr {
            Expr::Literal { value, .. } => {
                match value {
                    LiteralValue::Int(n) => {
                        let i64_type = self.context.i64_type();
                        let value_i64 = *n as u64;
                        i64_type.const_int(value_i64, true).into()
                    }
                    LiteralValue::Float(f) => {
                        let f64_type = self.context.f64_type();
                        f64_type.const_float(*f).into()
                    }
                    // todo LiteralValue::String(s) => {}
                    // todo LiteralValue::Char(c) => {}
                    LiteralValue::Bool(b) => {
                        let bool_type = self.context.bool_type();
                        let value_bool = *b as u64;
                        bool_type.const_int(value_bool, false).into()
                    }
                    // todo LiteralValue::Null() => {}
                    _ => todo!()
                }
            },

            Expr::Binary { left, operator, right } => {
                let mut lhs = self.compile_expr(left);
                let mut rhs = self.compile_expr(right);
                if lhs.is_int_value() && rhs.is_int_value() {
                    self.compile_int_binary(lhs.into_int_value(), operator, rhs.into_int_value()).into()
                } else {
                    if lhs.is_int_value() {
                        lhs = self.builder.build_signed_int_to_float(
                            lhs.into_int_value(), 
                            self.context.f64_type(), 
                            "casttmp"
                        ).unwrap().into();
                    }

                    if rhs.is_int_value() {
                        rhs = self.builder.build_signed_int_to_float(
                            rhs.into_int_value(), 
                            self.context.f64_type(), 
                            "casttmp"
                        ).unwrap().into();
                    }

                    self.compile_float_binary(lhs.into_float_value(), operator, rhs.into_float_value()).into()
                }
            }

            Expr::Variable { name } => {
                let (ptr, ty) = *self.variables.get(&name.lexeme).unwrap();
                self.builder.build_load(ty, ptr, &name.lexeme).unwrap()
            }

            _ => todo!(),
        
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            
            Stmt::Expression { value } => {
                self.compile_expr(value);
            }

            Stmt::Let { name, value, var_type, .. } => {
                let ty = self.var_to_llvm(var_type);
                let ptr = self.builder.build_alloca(ty, &name.lexeme).unwrap();
                let result = self.compile_expr(value);
                self.builder.build_store(ptr, result).unwrap();
                self.variables.insert(name.lexeme.clone(), (ptr, ty));
            }

            Stmt::Assign { name, value } => {
                let (ptr, ..) = *self.variables.get(&name.lexeme).unwrap();
                let result = self.compile_expr(value);
                self.builder.build_store(ptr, result).unwrap();
            }

            Stmt::Block { statements } => {
                for stmt in statements {
                    self.compile_stmt(stmt);
                }
            }

            Stmt::Function { name, params, statements, return_type } => {
                self.compile_function(name, params, statements, return_type);
            }



            _ => todo!()
        }
    }

    fn compile_function(&mut self, name: &Token, params: &Vec<(Token, VarType)>, stmts: &Stmt, return_type: &Option<VarType> ) {
        let og_block = self.builder.get_insert_block();
        let og_variables = self.variables.clone();
        let mut param_types: Vec<BasicMetadataTypeEnum> = vec![];
        for (_, var_type) in params {
            param_types.push(self.var_to_llvm(var_type).into());
        }

        let fn_type = match return_type {
            Some(r) => {
                 self.var_to_llvm( r).fn_type(&param_types, false)
            }
            None => {
                self.context.void_type().fn_type(&param_types, false)
            }
        };

        let fn_val = self.module.add_function(&name.lexeme, fn_type, None);
        let basic_block = self.context.append_basic_block(fn_val, "entry");
        self. builder.position_at_end(basic_block);

        self.variables = HashMap::new();
        self.compile_stmt(stmts);
        self.variables = og_variables;
        if let Some(b) = og_block {
            self.builder.position_at_end(b);
        };
    }

    fn compile_int_binary(&self, lhs: IntValue<'ctx>, op: &Token, rhs: IntValue<'ctx>) -> IntValue<'ctx> {
        match op.token_type {
            
            TokenType::Plus => {
                self.builder.build_int_add(lhs, rhs, "add").unwrap()
            }

            TokenType::Minus => {
                self.builder.build_int_sub(lhs, rhs, "sub").unwrap()
            }

            TokenType::Star => {
                self.builder.build_int_mul(lhs, rhs, "mul").unwrap()
            }

            TokenType::Slash => {
                self.builder.build_int_signed_div(lhs, rhs, "div").unwrap()
            }

            TokenType::Percent => {
                self.builder.build_int_signed_rem(lhs, rhs, "rem").unwrap()
            }

            _ => todo!()
        }
    }

    fn compile_float_binary(&self, lhs: FloatValue<'ctx>, op: &Token, rhs: FloatValue<'ctx>) -> FloatValue<'ctx> {
        match op.token_type {
            
            TokenType::Plus => {
                self.builder.build_float_add(lhs, rhs, "add").unwrap()
            }

            TokenType::Minus => {
                self.builder.build_float_sub(lhs, rhs, "sub").unwrap()
            }

            TokenType::Star => {
                self.builder.build_float_mul(lhs, rhs, "mul").unwrap()
            }

            TokenType::Slash => {
                self.builder.build_float_div(lhs, rhs, "div").unwrap()
            }

            _ => todo!()
        }
    }

    fn emit_obj(&self) {
        match Target::initialize_native(&InitializationConfig::default()) {
            Ok(()) => {}
            Err(e) => {
                panic!("Error: {}", &e);
            }
        }
        let default_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&default_triple).unwrap();
        let target_machine = target.create_target_machine(
            &default_triple, 
            "generic", 
            "", 
            OptimizationLevel::None,
            RelocMode::Default,
            CodeModel::Default,
        ).unwrap();
        let path = Path::new("/mnt/work_ssd/work/hyperrust/target/out.o");
        target_machine.write_to_file(&self.module, FileType::Object, path).unwrap();
    }

    fn var_to_llvm(&self, var_type: &VarType) -> BasicTypeEnum<'ctx> {
        match var_type {
            VarType::Int => {
                self.context.i64_type().into()
            },
            VarType::Float => {
                self.context.f64_type().into()
            },
            // todo VarType::Str => {},
            // todo VarType::Char => {},
            VarType::Bool => {
                self.context.bool_type().into()
            },
            _ => todo!()
        }
    }
}