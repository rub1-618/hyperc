use crate::token::{TokenType, Token};
use crate::ast::{Expr, Stmt, LiteralValue, VarType};
use crate::resolver::TypeInfo;
use crate::error::HypercError;
use crate::support::{expr_span, mangle, stmt_span, var_type_to_type};
use std::{collections::HashMap};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Str,
    Char,
    Bool,
    Unit,
    Named(String),
    Error
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int   => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Str   => write!(f, "str"),
            Type::Char  => write!(f, "char"),
            Type::Bool  => write!(f, "bool"),
            Type::Unit  => write!(f, "()"),
            Type::Named(str) => write!(f, "{}", str),
            Type::Error =>write!(f, "[error]")
        }
    }
}

#[derive(Debug, Clone)]
pub struct FnSig {
    args: Vec<Type>,
    return_type: Type
}

#[derive(Debug, Clone)]
pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    current_return_type: Option<Type>,
    current_self_type: Option<Type>,
    functions: HashMap<String, FnSig>,
    types: HashMap<String, TypeInfo>,
    pub errors: Vec<HypercError>
}

impl TypeChecker {

    pub fn new(types: HashMap<String, TypeInfo>) -> Self {
        TypeChecker {
            scopes: Vec::new(),
            current_return_type: None,
            current_self_type: None,
            functions: HashMap::new(),
            types,
            errors: Vec::new()
        }
    }

    pub fn check(&mut self, stmts: &[Stmt]) {
        self.begin_scope();
        for stmt in stmts {
            self.check_stmt(stmt);
        }
        self.end_scope();
    }

    fn check_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Binary {
                left, operator, right 
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);

                match operator.token_type {
                    TokenType::Plus | TokenType::Minus
                    | TokenType::Star | TokenType::Slash => {
                        if (lt == Type::Int || rt == Type::Float) 
                        && (rt == Type::Int || rt == Type::Float) {
                            if lt == Type::Float || rt == Type::Float {
                                return Type::Float;
                            } else {
                                return Type::Int;
                            }
                        } else {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(expr),
                                message: format!("Both arithmetic operands must be numeric, got: {lt} and {rt}.")
                            });
                            return Type::Error;
                        }
                    }

                    TokenType::Percent => {
                        if lt == Type::Int && rt == Type::Int {
                            return Type::Int;
                        } else {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(expr),
                                message: format!("Both modulo operands must be integers, got: {lt} and {rt}.")
                            });
                            return Type::Error;
                        }
                    }

                    TokenType::Less | TokenType::LessEqual
                    | TokenType::Greater | TokenType::GreaterEqual => {
                        if (lt == Type::Int || lt == Type::Float) 
                        && (rt == Type::Int || rt == Type::Float) {
                            return Type::Bool;
                        } else {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(expr),
                                message: format!("Both comparison operands must be numeric, got: {lt} and {rt}.")
                            });
                            return Type::Error;
                        }
                    }

                    TokenType::EqualEqual | TokenType::BangEqual => {
                        if (lt == Type::Int || lt == Type::Float)
                        && (rt == Type::Int || rt == Type::Float) {
                            return Type::Bool;
                        } else if lt == rt {
                            match &lt {
                                Type::Named(str) => {
                                    match self.types.get(&str.to_string()) {

                                        Some(TypeInfo::Enum { .. }) => return Type::Bool,

                                        Some(TypeInfo::Struct { .. }) => {
                                            self.errors.push(HypercError::TypeError {
                                                span: expr_span(expr),
                                                message: "Cannot compare structs. (yet)".to_string() 
                                            });
                                            return Type::Error;
                                        }

                                        None => {
                                            self.errors.push(HypercError::TypeError {
                                                span: expr_span(expr),
                                                message: "Type not found.".to_string()
                                            });
                                            return Type::Error;
                                        }
                                    }
                                }

                                Type::Char | Type::Bool => return Type::Bool,

                                Type::Str => {
                                    self.errors.push(HypercError::TypeError {
                                        span: expr_span(expr),
                                        message: "String comparison is not supported yet.".to_string()
                                    });
                                    return Type::Error;
                                }

                                _ => {
                                    self.errors.push(HypercError::TypeError {
                                        span: expr_span(expr),
                                        message: format!("Invalid comparison operands, got {lt} and {rt}.")
                                    });
                                    return Type::Error;
                                }
                            }
                        } else {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(expr),
                                message: format!("Invalid comparison operands, got {lt} and {rt}.")
                            });
                            return Type::Error;
                        }
                    }

                    TokenType::And | TokenType::Or => {
                        if lt == Type::Bool && rt == Type::Bool {
                            return Type::Bool;
                        } else {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(expr),
                                message: format!("Both logical operands must be boolean, got: {lt} and {rt}.")
                            });
                            return Type::Error;
                        }
                    }

                    _ => {
                        self.errors.push(HypercError::TypeError {
                            span: expr_span(expr),
                            message: "Unknown binary operation.".to_string()
                        });
                        return Type::Error;
                    }
                }
            }

            Expr::Unary { operator, right } => {
                let rt = self.check_expr(right);
                match operator.token_type {
                    
                    TokenType::Minus => {
                        if rt == Type::Int {
                            return Type::Int;
                        } else if rt == Type::Float {
                            return Type::Float;
                        } else {
                            self.errors.push(HypercError::BuildError {
                                span: expr_span(expr),
                                message: format!("Expected numeric operand after '-', got {rt}.")
                            });
                            return Type::Error;
                        }
                    }
                    
                    TokenType::Bang => {
                        if rt == Type::Bool {
                            return Type::Bool;
                        } else {
                            self.errors.push(HypercError::BuildError {
                                span: expr_span(expr),
                                message: format!("Expected boolean operand after '!', got {rt}.")
                            });
                            return Type::Error;
                        }

                    }

                    _ => {
                        self.errors.push(HypercError::TypeError {
                            span: expr_span(expr),
                            message: "Unknown unary operation.".to_string()
                        });
                        return Type::Error;
                    }
                }    
            }

            Expr::Call { callee, arguments } => {
                match &**callee {
                    
                    Expr::Variable { name } => {
                        let sig = match self.functions.get(&name.lexeme) {
                            Some(s) => s.clone(),
                            None => {
                                self.errors.push(HypercError::TypeError {
                                    span: expr_span(expr),
                                    message: "Function not found.".to_string()
                                });
                                return Type::Error;
                            }
                        };
                        if sig.args.len() != arguments.len() {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(expr),
                                message: "Wrong number of arguments.".to_string()
                            });
                            return Type::Error;
                        }
                        for i in 0..arguments.len() {
                            let arg_ty = self.check_expr(&arguments[i]);
                            if arg_ty != sig.args[i] {
                                self.errors.push(HypercError::TypeError {
                                    span: expr_span(&arguments[i]),
                                    message: format!("Argument type mismatched. Expected: {}, got: {arg_ty}.", sig.args[i])
                                });
                                return Type::Error;
                            }
                        }
                        return sig.return_type.clone();
                    }
                    
                    Expr::Get { object, item } => {
                        match self.check_expr(object) {

                            Type::Named(str) => {
                                let mangled = mangle(&str, &item.lexeme);
                                let sig = match self.functions.get(&mangled) {
                                    Some(s) => s.clone(),
                                    None => {
                                        self.errors.push(HypercError::TypeError {
                                            span: expr_span(expr),
                                            message: "Method not found.".to_string()
                                        });
                                        return Type::Error;
                                    }
                                };
                                    if sig.args.len() != arguments.len() {
                                        self.errors.push(HypercError::TypeError {
                                            span: expr_span(object),
                                            message: "Wrong number of arguments.".to_string()
                                        });
                                        return Type::Error;
                                    }
                                    for i in 0..arguments.len() {
                                        let arg_ty = self.check_expr(&arguments[i]);
                                        if arg_ty != sig.args[i] {
                                            self.errors.push(HypercError::TypeError {
                                                span: expr_span(&arguments[i]),
                                                message: format!("Argument type mismatched. Expected: {}, got: {arg_ty}.", sig.args[i])
                                            });
                                            return Type::Error;
                                        }
                                    }
                                    return sig.return_type.clone();
                            }
                            
                            _ => {
                                self.errors.push(HypercError::TypeError {
                                    span: expr_span(expr),
                                    message: "This type has no implementations.".to_string()
                                });
                                return Type::Error;
                            }
                        }
                    }

                    Expr::Path { type_name, item } => {
                        let mangled = mangle(&type_name.lexeme, &item.lexeme);
                        let sig = match self.functions.get(&mangled) {
                            Some(s) => s.clone(),
                            None => {
                                self.errors.push(HypercError::TypeError {
                                    span: expr_span(expr),
                                    message: "Type or its method not found.".to_string()
                                });
                                return Type::Error;
                            }
                        };
                        if sig.args.len() != arguments.len() {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(expr),
                                message: "Wrong number of arguments.".to_string()
                            });
                            return Type::Error;
                        }
                        for i in 0..arguments.len() {
                            let arg_ty = self.check_expr(&arguments[i]);
                            if arg_ty != sig.args[i] {
                                self.errors.push(HypercError::TypeError {
                                    span: expr_span(&arguments[i]),
                                    message: format!("Argument type mismatched. Expected: {}, got: {arg_ty}.", sig.args[i])
                                });
                                return Type::Error;
                            }
                        }
                        return sig.return_type.clone();
                    }
                    
                    _ => {
                        self.errors.push(HypercError::TypeError {
                            span: expr_span(expr),
                            message: "Function not found.".to_string()
                        });
                        return Type::Error;
                    }
                }
            }

            Expr::Literal { value, .. } => {
                match value {
                    LiteralValue::Int(_)    => Type::Int,
                    LiteralValue::Float(_)  => Type::Float,
                    LiteralValue::String(_) => Type::Str,
                    LiteralValue::Char(_)   => Type::Char,
                    LiteralValue::Bool(_)   => Type::Bool,
                    LiteralValue::None      => Type::Unit
                }
            }

            Expr::Grouping { expr } => self.check_expr(expr),

            Expr::Variable { name } => {
                match self.get_type(&name.lexeme) {
                    Some(ty) => return ty,
                    _ => {
                        self.errors.push(HypercError::TypeError {
                            span: expr_span(expr),
                            message: "Unable to get the variable type.".to_string()
                        });
                        return Type::Error;
                    }
                }
            }

            Expr::StructLit { name, fields } => {
                let expected_fields = match self.types.get(&name.lexeme) {
                    Some(TypeInfo::Struct { fields }) => fields.clone(),
                    _ => unreachable!()
                };
                if expected_fields.len() != fields.len() {
                    self.errors.push(HypercError::TypeError {
                        span: expr_span(expr),
                        message: format!("Wrong number of fields. Expected: {}, got: {}.",
                            expected_fields.len(), fields.len()
                        )
                    });
                }
                for (tok, expr) in fields {
                    match expected_fields.iter().find(|(n,_)| n == &tok.lexeme) {
                        Some((_, vt)) => {
                            let expected = var_type_to_type(vt);
                            let got = self.check_expr(expr);
                            if got != expected {
                                self.errors.push(HypercError::TypeError {
                                    span: tok.start..tok.end,
                                    message: format!("Mismatched types. Expected: {expected}, got: {got}.")
                                });
                                return Type::Error;
                            }
                        }
                        None => {
                            self.errors.push(HypercError::TypeError {
                                span: tok.start..tok.end,
                                message: "Unknown field.".to_string()
                            });
                            return Type::Error;
                        }
                    }
                }
                return Type::Named(name.lexeme.clone())
            }

            Expr::Get { object, item } => {
                let ty = self.check_expr(object);
                match ty {

                    Type::Named(str) => {
                        match self.types.get(&str) {
                            
                            Some(TypeInfo::Enum { .. }) => {
                                self.errors.push(HypercError::TypeError {
                                    span: expr_span(object),
                                    message: format!("Type {str} has no fields, only variants.")
                                });
                                return Type::Error;
                            }
                            
                            Some(TypeInfo::Struct { fields }) => {
                                match fields.iter().find(|(n,_)| n == &item.lexeme ) {
                                    
                                    Some((_, vt)) => {
                                        return var_type_to_type(vt);
                                    }
                                    
                                    None => {
                                        self.errors.push(HypercError::TypeError {
                                            span: item.start..item.end,
                                            message: "Unknown field.".to_string()
                                        });
                                        return Type::Error;
                                    }
                                }
                            }
                            
                            None => {
                                self.errors.push(HypercError::TypeError {
                                    span: expr_span(object),
                                    message: "Type not found.".to_string()
                                });
                                return Type::Error;
                            }
                        }
                    }

                    _ => {
                        self.errors.push(HypercError::TypeError {
                            span: expr_span(object),
                            message: format!("Type {ty} has no fields.")
                        });
                        return Type::Error;
                    }
                }
            }

            Expr::Path { type_name, .. } => Type::Named(type_name.lexeme.clone()),

            Expr::SelfExpr { .. } => {
                match &self.current_self_type {
                    Some(expected) => expected.clone(),
                    None => {
                        self.errors.push(HypercError::TypeError {
                            span: expr_span(expr),
                            message: "'self' is outside of a method.".to_string()
                        });
                        return Type::Error;
                    }
                }
            
            }
            
            _ => unreachable!()
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {

            Stmt::Expression { value } => {self.check_expr(value);},

            Stmt::Print { value } => {
                let result = self.check_expr(value);
                match result {
                    Type::Named(str) => {
                        match self.types.get(&str) {
                            Some(TypeInfo::Struct { .. }) => self.errors.push(HypercError::TypeError {
                                span: expr_span(value),
                                message: "Cannot print structs. (yet)".to_string() 
                            }),
                            Some(TypeInfo::Enum   { .. }) => {}
                            None => self.errors.push(HypercError::TypeError {
                                span: expr_span(value),
                                message: "Unknown type.".to_string()
                            })
                        }
                    }
                    _ => {}
                }
            }

            Stmt::Let { 
                name, value, 
                var_type, .. 
            } => {
                let expected = var_type_to_type(var_type);
                let got = self.check_expr(value);
                if got == expected {
                    self.declare(name, got);
                } else {
                    self.errors.push(HypercError::TypeError {
                        span: expr_span(value),
                        message: format!("Mismatched types. Expected: {expected}, got: {got}.")
                    })
                }
            }

            Stmt::Assign { target, value } => {
                let expected = self.check_expr(target);
                let got = self.check_expr(value);
                if got != expected {
                    self.errors.push(HypercError::TypeError {
                        span: expr_span(value),
                        message: format!("Mismatched types. Expected: {expected}, got: {got}.")
                    })
                }
            }
            
            Stmt::Block { statements } => {
                self.begin_scope();
                let result = ( || {
                    for stmt in statements {
                        self.check_stmt(stmt);
                    }
                })();
                self.end_scope();
                result
            }
            
            Stmt::If { 
                condition, 
                then_branch, 
                else_branch 
            } => {
                let cond = self.check_expr(condition);
                if cond == Type::Bool {
                    self.check_stmt(then_branch);
                    if let Some(else_stmt) = else_branch {
                        self.check_stmt(else_stmt);
                    }
                } else {
                    self.errors.push(HypercError::TypeError {
                        span: expr_span(condition),
                        message: format!("If condition must be boolean, got: {cond}")
                    })
                }
            }

            Stmt::While { condition, statements } => {
                let cond = self.check_expr(condition);
                if cond == Type::Bool {
                    self.check_stmt(statements);
                } else {
                    self.errors.push(HypercError::TypeError {
                        span: expr_span(condition),
                        message: format!("While condition must be boolean, got: {cond}")
                    })
                }
            }

            Stmt::For { 
                initializer, condition, 
                increment, statements 
            } => {
                self.begin_scope();
                let result = ( || {
                    if let Some(init) = initializer { self.check_stmt(init); }
                    if let Some(cond) = condition {
                        let got = self.check_expr(cond);
                        if got == Type::Bool {
                            self.check_stmt(statements);
                        } else {
                            self.errors.push(HypercError::TypeError {
                                span: expr_span(cond),
                                message: format!("While condition must be boolean, got: {got}")
                            })
                        }
                    }
                    if let Some(incr) = increment { self.check_stmt(incr); }
                    self.check_stmt(statements);
                })();
                self.end_scope();
                result
            }

            Stmt::Func { 
                name, args, 
                statements, return_type
            } => {
                self.check_func_body(
                    name, args, 
                    statements, return_type, 
                    false, None
                );
            }
            
            Stmt::Return { value, .. } => {
                let ret_type = match value {
                    Some(v) => self.check_expr(v),
                    None => Type::Unit
                };
                match &self.current_return_type {
                    Some(expected) => {
                        if &ret_type != expected {
                            self.errors.push(HypercError::TypeError {
                                span: stmt_span(stmt),
                                message: format!("Mismatched return type. Expected: {expected}, got: {ret_type}.")
                            });
                        }
                    }
                    None => self.errors.push(HypercError::TypeError {
                        span: stmt_span(stmt),
                        message: "Return must be inside a function.".to_string()
                    })
                }
            }
            
            Stmt::Struct { .. } => {}
            
            Stmt::Impl { name, methods } => {
                let named = Some(name);
                self.current_self_type = Some(Type::Named(name.lexeme.clone()));
                for method in methods {
                    match method {
                        Stmt::Func { 
                            name, args, 
                            statements, return_type 
                        } => {
                            self.check_func_body(
                                name, args, 
                                statements, return_type, 
                                true, named
                            );
                        }
                        _ => unreachable!()
                    }
                }
                self.current_self_type = None;
            }
            
            Stmt::Enum { .. } => {}
        }
    }

    fn check_func_body(&mut self,
    name: &Token, args: &Vec<(Token, VarType)>, statements: &Box<Stmt>,
    return_type: &Option<VarType>, is_method: bool, named: Option<&Token>
    ) {
        let prev_st = self.current_self_type.take();
        let prev_rt = self.current_return_type.take();
        self.current_return_type = if is_method {
            prev_st.clone()
        } else { None };
        let key_owner = match named {
            Some(ty) => mangle(&ty.lexeme, &name.lexeme),
            None => name.lexeme.clone()
        };
        let prev_fn_rt = match return_type {
            Some(vt) => var_type_to_type(vt),
            None => Type::Unit
        };
        self.current_return_type = Some(prev_fn_rt.clone());
        let mut arg_types = vec![];
        for (_, vt) in args {
            arg_types.push(var_type_to_type(vt));
        }
        self.functions.insert(key_owner, FnSig { 
            args: arg_types, return_type: prev_fn_rt 
        });
        self.begin_scope();
        let result = ( || {
            for (tok, vt) in args {
                self.declare(tok, var_type_to_type(vt));
            }
            self.check_stmt(statements);
            match return_type {
                Some(_) => {
                    if !Self::has_return(statements) {
                        self.errors.push(HypercError::TypeError {
                            span: name.start..name.end,
                            message: "Not all paths return.".to_string()
                        })
                    }
                }
                None => {}
            }
        })();
        self.end_scope();
        self.current_return_type = prev_rt;
        self.current_self_type = prev_st;
        result
    }

    fn declare(&mut self, name: &Token, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme.clone(), ty);
        }
    }

    fn has_return(stmt: &Stmt) -> bool {
        match stmt {

            Stmt::Return { .. } => true,

            Stmt::Block { statements } => {
                let check = statements.iter().any(|s: &Stmt| Self::has_return(s));
                if check { true } else { false }
            }

            Stmt::If { 
                then_branch, 
                else_branch, .. 
            } => {
                match else_branch {
                    Some(s) => {
                        if Self::has_return(then_branch) && Self::has_return(s) {
                            true
                        } else { false }
                    }
                    None => false
                }
            }

            _ => false
        }
    }

    fn get_type(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }
}