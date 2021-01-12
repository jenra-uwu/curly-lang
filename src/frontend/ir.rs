use logos::Span;
use std::collections::HashMap;
use std::iter::FromIterator;

use super::parser::AST;
use super::scopes::Scope;
use super::types;
use super::types::Type;

// Represents a prefix operator.
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum PrefixOp
{
    Neg,
    Span,
}

// Represents an infix operator.
#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum BinOp
{
    Mul,
    Div,
    Mod,
    Add,
    Sub,
    BSL,
    BSR,
    LT,
    GT,
    LEQ,
    GEQ,
    EQ,
    NEQ,
    In,
    And,
    Or,
    Xor,
    BoolAnd,
    BoolOr,
    BoolXor,
}

// Represents metadata associated with sexpressions.
#[derive(Debug)]
pub struct SExprMetadata
{
    pub span: Span,
    pub _type: Type
}

// Represents an s expression
#[derive(Debug)]
pub enum SExpr
{
    // Ints
    Int(SExprMetadata, i64),

    // Floats
    Float(SExprMetadata, f64),

    // Booleans
    True(SExprMetadata),
    False(SExprMetadata),

    // Symbols
    Symbol(SExprMetadata, String),

    // Strings
    String(SExprMetadata, String),

    // Functions
    Function(SExprMetadata, String),

    // Prefix expression
    Prefix(SExprMetadata, PrefixOp, Box<SExpr>),

    // Infix expression
    Infix(SExprMetadata, BinOp, Box<SExpr>, Box<SExpr>),

    // If expression
    If(SExprMetadata, Box<SExpr>, Box<SExpr>, Box<SExpr>),

    // Function application
    Application(SExprMetadata, Box<SExpr>, Box<SExpr>),

    // Assignment
    Assign(SExprMetadata, String, Box<SExpr>),

    // Scoping
    With(SExprMetadata, Vec<SExpr>, Box<SExpr>),
}

impl SExpr
{
    // get_metadata(&SExpr) -> &SExprMetadata
    // Returns an immutable reference to the metadata.
    pub fn get_metadata(&self) -> &SExprMetadata
    {
        match self
        {
            Self::Int(m, _)
                | Self::Float(m, _)
                | Self::True(m)
                | Self::False(m)
                | Self::Symbol(m, _)
                | Self::String(m, _)
                | Self::Function(m, _)
                | Self::Prefix(m, _, _)
                | Self::Infix(m, _, _, _)
                | Self::If(m, _, _, _)
                | Self::Application(m, _, _)
                | Self::Assign(m, _, _)
                | Self::With(m, _, _)
                => m
        }
    }

    // get_mutable_metadata(&mut SExpr) -> &mut SExprMetadata
    // Returns a mutable reference to the metadata.
    pub fn get_mutable_metadata(&mut self) -> &mut SExprMetadata
    {
        match self
        {
            Self::Int(m, _)
                | Self::Float(m, _)
                | Self::True(m)
                | Self::False(m)
                | Self::Symbol(m, _)
                | Self::String(m, _)
                | Self::Function(m, _)
                | Self::Prefix(m, _, _)
                | Self::Infix(m, _, _, _)
                | Self::If(m, _, _, _)
                | Self::Application(m, _, _)
                | Self::Assign(m, _, _)
                | Self::With(m, _, _)
                => m
        }
    }
}

#[derive(Debug)]
pub struct IRFunction
{
    pub args: Vec<(String, Type)>,
    pub body: SExpr,
    pub global: bool
}

#[derive(Debug)]
pub struct IRMetadata
{
    pub scope: Scope,
}

impl IRMetadata
{
    // push_scope(&mut self) -> ()
    // Pushes a new scope to the top of the scope stack.
    pub fn push_scope(&mut self)
    {
        use std::mem::swap;

        let mut scope = Scope::new();

        swap(&mut scope, &mut self.scope);
        self.scope.parent = Some(Box::new(scope));
    }

    // pop_scop(&mut self) -> ()
    // Pops a scope from the stack if a parent scope exists.
    pub fn pop_scope(&mut self)
    {
        use std::mem::swap;

        if let Some(v) = &mut self.scope.parent
        {
            let mut scope = Scope::new();

            swap(&mut scope, v);
            swap(&mut self.scope, &mut scope);
        }
    }
}

// Represents the ir.
#[derive(Debug)]
pub struct IR
{
    pub metadata: IRMetadata,
    pub funcs: HashMap<String, IRFunction>,
    pub sexprs: Vec<SExpr>
}

impl IR
{
    // new() -> IR
    // Creates a new root IR.
    pub fn new() -> IR
    {
        IR {
            metadata: IRMetadata {
                scope: Scope::new().init_builtins()
            },
            funcs: HashMap::with_capacity(0),
            sexprs: vec![]
        }
    }

    // clear(&mut self) -> ()
    // Clears the root of any sexpressions.
    pub fn clear(&mut self)
    {
        self.sexprs.clear();
    }
}

// convert_node(AST, &mut HashMap<String, IRFunction>, bool) -> SExpr
// Converts an ast node into an sexpression.
fn convert_node(ast: AST, funcs: &mut HashMap<String, IRFunction>, global: bool, seen_funcs: &mut HashMap<String, usize>) -> SExpr
{
    match ast
    {
        // Int
        AST::Int(span, n) => SExpr::Int(SExprMetadata {
            span,
            _type: Type::Int
        }, n),

        // Float
        AST::Float(span, n) => SExpr::Float(SExprMetadata {
            span,
            _type: Type::Float
        }, n),

        // True
        AST::True(span) => SExpr::True(SExprMetadata {
            span,
            _type: Type::Bool
        }),
 
        // False
        AST::False(span) => SExpr::False(SExprMetadata {
            span,
            _type: Type::Bool
        }),

        // Symbol
        AST::Symbol(span, s) => SExpr::Symbol(SExprMetadata {
            span,
            _type: Type::Error
        }, s),

        // String
        AST::String(span, s) => SExpr::String(SExprMetadata {
            span,
            _type: Type::String
        }, s),

        // Prefix
        AST::Prefix(span, op, v) => {
            let op = match op.as_str()
            {
                "-" => PrefixOp::Neg,
                "*" => PrefixOp::Span,
                _ => panic!("Invalid operator"),
            };

            SExpr::Prefix(SExprMetadata {
                span,
                _type: Type::Error
            }, op, Box::new(convert_node(*v, funcs, global, seen_funcs)))
        }

        // Infix
        AST::Infix(span, op, l, r) => {
            // Get operator
            let op = match op.as_str()
            {
                "*" => BinOp::Mul,
                "/" => BinOp::Div,
                "%" => BinOp::Mod,
                "+" => BinOp::Add,
                "-" => BinOp::Sub,
                "<<" => BinOp::BSL,
                ">>" => BinOp::BSR,
                "<" => BinOp::LT,
                ">" => BinOp::GT,
                "<=" => BinOp::LEQ,
                ">=" => BinOp::GEQ,
                "==" => BinOp::EQ,
                "!=" => BinOp::NEQ,
                "in" => BinOp::In,
                "&" => BinOp::And,
                "|" => BinOp::Or,
                "^" => BinOp::Xor,
                "and" => BinOp::BoolAnd,
                "or" => BinOp::BoolOr,
                "xor" => BinOp::BoolXor,
                _ => panic!("Invalid operator"),
            };

            // Return
            SExpr::Infix(SExprMetadata {
                span,
                _type: Type::Error
            }, op, Box::new(convert_node(*l, funcs, global, seen_funcs)), Box::new(convert_node(*r, funcs, global, seen_funcs)))
        }

        // If expression
        AST::If(span, cond, then, elsy) => SExpr::If(SExprMetadata {
            span,
            _type: Type::Error
        }, Box::new(convert_node(*cond, funcs, global, seen_funcs)), Box::new(convert_node(*then, funcs, global, seen_funcs)), Box::new(convert_node(*elsy, funcs, global, seen_funcs))),

        // Application
        AST::Application(span, l, r) => SExpr::Application(SExprMetadata {
            span,
            _type: Type::Error
        }, Box::new(convert_node(*l, funcs, global, seen_funcs)), Box::new(convert_node(*r, funcs, global, seen_funcs))),

        // Assignment
        AST::Assign(span, name, val) => SExpr::Assign(SExprMetadata {
            span,
            _type: Type::Error
        }, name, Box::new(convert_node(*val, funcs, global, seen_funcs))),

        // Assignment with types
        AST::AssignTyped(span, name, _type, val) => SExpr::Assign(SExprMetadata {
            span,
            _type: types::convert_ast_to_type(*_type)
        }, name, Box::new(convert_node(*val, funcs, global, seen_funcs))),

        // Assigning functions
        AST::AssignFunction(span, name, args, val) => {
            // Get function id
            let func_name = if seen_funcs.contains_key(&name)
            {
                let seen = seen_funcs.get_mut(&name).unwrap();
                *seen += 1;
                format!("{}.{}", name, seen)
            } else
            {
                seen_funcs.insert(name.clone(), 0);
                name.clone()
            };

            let func_id = SExpr::Function(SExprMetadata {
                span: val.get_span(),
                _type: Type::Error
            }, func_name.clone());

            // Create the function
            let func = IRFunction {
                args: args.into_iter().map(|v| (v.0, types::convert_ast_to_type(v.1))).collect(),
                body: convert_node(*val, funcs, false, seen_funcs),
                global
            };

            // Return assigning to the function id
            funcs.insert(func_name, func);
            SExpr::Assign(SExprMetadata {
                span,
                _type: Type::Error
            }, name, Box::new(func_id))
        }

        // With expressions
        AST::With(span, a, v) => {
            let v = convert_node(*v, funcs, false, seen_funcs);
            SExpr::With(SExprMetadata {
                span,
                _type: v.get_metadata()._type.clone()
            }, a.into_iter().map(|a| convert_node(a, funcs, false, seen_funcs)).collect(), Box::new(v))
        }
    }
}

// convert_ast_to_ir(Vec<AST>) -> IR
// Converts a list of asts into ir.
pub fn convert_ast_to_ir(asts: Vec<AST>, ir: &mut IR)
{
    let mut seen_funcs = HashMap::from_iter(ir.funcs.iter().map(|v| (v.0.clone(), 0usize)));
    println!("{:?}", seen_funcs);
    for ast in asts
    {
        ir.sexprs.push(convert_node(ast, &mut ir.funcs, true, &mut seen_funcs));
    }
}

