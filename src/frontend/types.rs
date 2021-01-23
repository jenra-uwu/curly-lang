use logos::Span;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Error, Formatter};
use std::hash::{Hash, Hasher};

use super::ir::IR;
use super::parser::AST;

#[derive(Clone, Debug)]
pub struct HashSetWrapper<T>(pub HashSet<T>);

impl<T: Hash + Eq> PartialEq for HashSetWrapper<T>
{
    fn eq(&self, other: &HashSetWrapper<T>) -> bool
    {
        self.0 == other.0
    }
}

impl<T: Hash + Eq> Eq for HashSetWrapper<T> { }

impl<T> Hash for HashSetWrapper<T>
{
    fn hash<H: Hasher>(&self, _: &mut H) { }
}

// Represents a type.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Type
{
    Error,
    ConversionError(Span),
    Unknown,
    Int,
    Float,
    Bool,
    String,
    Symbol(String),
    Func(Box<Type>, Box<Type>),
    Sum(HashSetWrapper<Type>),
    Enum(String)
}

impl Display for Type
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error>
    {
        match self
        {
            // Errors
            Type::Error => { write!(f, "TypeError")?; }
            Type::ConversionError(_) => { write!(f, "ConversionError")?; }
            Type::Unknown => { write!(f, "UnknownType")?; }

            // Primatives
            Type::Int => { write!(f, "Int")?; }
            Type::Float => { write!(f, "Float")?; }
            Type::Bool => { write!(f, "Bool")?; }
            Type::String => { write!(f, "String")?; }
            Type::Symbol(s) => { write!(f, "{}", s)?; }
            Type::Enum(e) => { write!(f, "enum {}", e)?; }

            // Fuction types
            Type::Func(func, a) => {
                if let Type::Func(_, _) = **func
                {
                    write!(f, "({})", **func)?;
                } else
                {
                    write!(f, "{}", **func)?;
                }
                write!(f, " -> {}", a)?;
            }

            // Sum types
            Type::Sum(fields) => {
                let mut bar = false;
                for field in fields.0.iter()
                {
                    if bar
                    {
                        write!(f, " | ")?;
                    } else
                    {
                        bar = true;
                    }

                    if let Type::Func(_, _) = field
                    {
                        write!(f, "({})", field)?;
                    } else
                    {
                        write!(f, "{}", field)?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Type
{
    // is_subtype(&self, &Type, &IR) -> bool
    // Returns true if self is a valid subtype in respect to the passed in type.
    pub fn is_subtype(&self, supertype: &Type, ir: &IR) -> bool
    {
        let mut _type = self;
        while let Type::Symbol(s) = _type
        {
            _type = ir.types.get(s).unwrap();
        }

        let mut supertype = supertype;
        while let Type::Symbol(s) = supertype
        {
            supertype = ir.types.get(s).unwrap();
        }

        if _type == supertype
        {
            return true;
        }

        println!("uwu: {} < {}", _type, supertype);

        match supertype
        {
            // Primatives
            Type::Int => *_type == Type::Int,
            Type::Float => *_type == Type::Float,
            Type::Bool => *_type == Type::Bool,
            Type::String => *_type == Type::String,

            // Functions
            Type::Func(sf, sa) =>
                if let Type::Func(f, a) = _type
                {
                    f == sf && a == sa
                } else
                {
                    false
                }

            // Sum types
            Type::Sum(types) => {
                for t in types.0.iter()
                {
                    if _type.is_subtype(t, ir)
                    {
                        return true;
                    }
                }

                false
            }

            // Enums
            Type::Enum(se) =>
                if let Type::Enum(e) = _type
                {
                    se == e
                } else
                {
                    false
                }

            // Everything else is to be ignored
            _ => false
        }
    }
}

// convert_ast_to_type(AST, &IR) -> Type
// Converts an ast node into a type.
pub fn convert_ast_to_type(ast: AST, types: &HashMap<String, Type>) -> Type
{
    match ast
    {
        // Symbols
        AST::Symbol(s, v) => {
            match v.as_str()
            {
                // Primatives
                "Int" => Type::Int,
                "Float" => Type::Float,
                "Bool" => Type::Bool,

                // Check if registered in IR
                _ =>
                    if let Some(_) = types.get(&v)
                    {
                        Type::Symbol(v)
                    } else
                    {
                        Type::ConversionError(s)
                    }
            }
        }

        // Enums
        AST::Prefix(_, op, v) if op == "enum" =>
            if let AST::Symbol(_, v) = *v
            {
                Type::Enum(v)
            } else
            {
                unreachable!("enum always has a symbol");
            }

        // Sum types
        AST::Infix(_, op, l, r) if op == "|" => {
            let mut fields = HashSet::new();
            fields.insert(convert_ast_to_type(*r, types));
            let mut acc = *l;

            loop 
            {
                match acc
                {
                    AST::Infix(_, op, l, r) if op == "|" => {
                        let v = convert_ast_to_type(*r, types);
                        if let Type::Sum(v) = v
                        {
                            for v in v.0
                            {
                                fields.insert(v);
                            }
                        } else
                        {
                            fields.insert(v);
                        }

                        acc = *l;
                    }

                    _ => break
                }
            }

            for f in fields.iter()
            {
                if let Type::ConversionError(s) = f
                {
                    return Type::ConversionError(s.clone());
                }
            }

            fields.insert(convert_ast_to_type(acc, types));
            if fields.len() == 1
            {
                fields.into_iter().next().unwrap()
            } else
            {
                Type::Sum(HashSetWrapper(fields))
            }
        }

        // Function types
        AST::Infix(_, op, l, r) if op == "->" =>
        {
            let l = convert_ast_to_type(*l, types);
            let r = convert_ast_to_type(*r, types);

            if let Type::ConversionError(s) = l
            {
                Type::ConversionError(s)
            } else if let Type::ConversionError(s) = r
            {
                Type::ConversionError(s)
            } else
            {
                Type::Func(Box::new(l), Box::new(r))
            }
        }

        // Parenthesised types
        AST::Prefix(_, op, v) if op == "" =>
            convert_ast_to_type(*v, types),

        // Error
        _ => Type::ConversionError(ast.get_span())
    }
}

