use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::frontend::ir::{BinOp, IR, PrefixOp, SExpr};
use crate::frontend::types::Type;

// Represents a function in C.
struct CFunction<'a>
{
    name: String,
    args: Vec<(&'a String, &'a Type)>,
    ret_type: &'a Type,
    code: String,
    last_reference: usize
}

// Represents a structure in C
#[derive(Clone, Debug)]
enum CType
{
    Primative(String, Type),
    Sum(String, Type, HashMap<Type, usize>)
}

impl CType
{
    // get_c_name(&self) -> &String
    // Gets the c name of the type.
    fn get_c_name(&self) -> &String
    {
        match self
        {
            CType::Primative(s, _)
                | CType::Sum(s, _, _)
                => s
        }
    }

    // get_curly_type(&self) -> &Type
    // Returns the Curly IR type.
    fn get_curly_type(&self) -> &Type
    {
        match self
        {
            CType::Primative(_, t)
                | CType::Sum(_, t, _)
                => t
        }
    }

    // get_hashmap(&self) -> Option<&HashMap<Type, usize>>
    // Gets the hashmap associated with a sum type.
    fn get_hashmap(&self) -> Option<&HashMap<Type, usize>>
    {
        match self
        {
            CType::Sum(_, _, v) => Some(v),
            _ => None
        }
    }
}

// get_c_type(&Type, &HashMap<Type, String>) -> &str
// Converts an IR type into a C type.
fn get_c_type<'a>(_type: &Type, types: &'a HashMap<Type, CType>) -> &'a str
{
    match _type
    {
        Type::Int => "int_t",
        Type::Float => "float_t",
        Type::Bool => "char",
        Type::Func(_, _) => "func_t",
        Type::Symbol(_) => types.get(_type).unwrap().get_c_name(),
        Type::Sum(_) => types.get(_type).unwrap().get_c_name(),
        _ => panic!("unsupported type!")
    }
}

// sanitise_symbol(&str) -> String
// Sanitises a symbol.
fn sanitise_symbol(value: &str) -> String
{
    let mut s = value.replace("'", "$$PRIME$$");
    s.push_str("$");
    s
}

// convert_sexpr(&SExpr, &IR, &mut CFunction, &HashMap<Type, String>) -> String
// Converts a s expression into C code.
fn convert_sexpr(sexpr: &SExpr, root: &IR, func: &mut CFunction, types: &HashMap<Type, CType>) -> String
{
    match sexpr
    {
        // Ints
        SExpr::Int(_, n) => {
            // Get name
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str("int_t ");
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(&format!("{}", n));
            func.code.push_str(";\n");

            name
        }

        // Floats
        SExpr::Float(_, n) => {
            // Get name
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str("float_t ");
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(&format!("{}", n));
            func.code.push_str(";\n");

            name
        }

        // Booleans
        SExpr::True(_) => {
            // Get name
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str("char ");
            func.code.push_str(&name);
            func.code.push_str(" = 1;\n");

            name
        }

        SExpr::False(_) => {
            // Get name
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str("char ");
            func.code.push_str(&name);
            func.code.push_str(" = 0;\n");

            name
        }

        // Symbols
        SExpr::Symbol(_, s) => {
            sanitise_symbol(s)
        }

        // Functions
        SExpr::Function(_, s) => {
            // Get name
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            let f = root.funcs.get(s).unwrap();
            func.code.push_str("func_t ");
            func.code.push_str(&name);
            func.code.push_str(" = { 0, (void*) ");
            func.code.push_str(s);
            func.code.push_str("$$FUNC$$, (void*) ");
            func.code.push_str(s);
            func.code.push_str(&format!("$$WRAPPER$$, {}", f.args.len() + f.captured.len()));
            func.code.push_str(", 0, ");
            if f.captured.len() > 0
            {
                let count = f.args.len() + f.captured.len();
                func.code.push_str("calloc(");
                func.code.push_str(&format!("{}", count));
                func.code.push_str(", sizeof(void*)), calloc(");
                func.code.push_str(&format!("{}", count));
                func.code.push_str(", sizeof(void*)) };\n");
            } else
            {
                func.code.push_str("(void*) 0, (void*) 0 };\n");
            }

            // Save captured variables
            for c in f.captured_names.iter()
            {
                // Get type
                let mut v = sanitise_symbol(c);
                let mut _type = f.captured.get(c).unwrap();
                while let Type::Symbol(s) = _type
                {
                    _type = root.types.get(s).unwrap();
                }

                // Fix functions, floats, and sum types
                match _type
                {
                    Type::Float => {
                        let name = format!("$${}", func.last_reference);
                        func.last_reference += 1;
                        func.code.push_str("double_wrapper_t ");
                        func.code.push_str(&name);
                        func.code.push_str(";\n");
                        func.code.push_str(&name);
                        func.code.push_str(".d = ");
                        func.code.push_str(&v);
                        func.code.push_str(";\nvoid* ");
                        v = format!("$${}", func.last_reference);
                        func.last_reference += 1;
                        func.code.push_str(&v);
                        func.code.push_str(" = ");
                        func.code.push_str(&name);
                        func.code.push_str(".v;\n");
                    }

                    Type::Func(_, _) => {
                        v = {
                            let name = format!("$${}", func.last_reference);
                            func.last_reference += 1;
                            func.code.push_str("func_t* ");
                            func.code.push_str(&name);
                            func.code.push_str(" = copy_func_arg(&");
                            func.code.push_str(&v);
                            func.code.push_str(");\n");
                            name
                        };
                        func.code.push_str(&name);
                        func.code.push_str(".args[");
                        func.code.push_str(&name);
                        func.code.push_str(".argc] = (void*) force_free_func;\n");
                    }

                    Type::Sum(_) => {
                        let name = format!("$${}", func.last_reference);
                        func.last_reference += 1;
                        func.code.push_str(get_c_type(&_type, types));
                        func.code.push_str("* ");
                        func.code.push_str(&name);
                        func.code.push_str(" = malloc(sizeof(");
                        func.code.push_str(get_c_type(&_type, types));
                        func.code.push_str("));\n*");
                        func.code.push_str(&name);
                        func.code.push_str(" = ");
                        func.code.push_str(&v);
                        func.code.push_str(";\n");
                        v = name;
                    }

                    _ => ()
                }

                func.code.push_str(&name);
                func.code.push_str(".args[");
                func.code.push_str(&name);
                func.code.push_str(".argc++] = (void*) ");
                func.code.push_str(&v);
                func.code.push_str(";\n");
            }

            name
        }

        // Prefix
        SExpr::Prefix(m, op, v) => {
            // Get name and value
            let val = convert_sexpr(v, root, func, types);
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str(get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(match op
            {
                PrefixOp::Neg => "-",
                PrefixOp::Span => panic!("unsupported operator!")
            });
            func.code.push_str(&val);
            func.code.push_str(";\n");

            name
        }

        // Infix
        SExpr::Infix(m, op, l, r) => {
            // Get name and operands
            let left = convert_sexpr(l, root, func, types);
            let right = convert_sexpr(r, root, func, types);
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str(get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(&left);
            func.code.push(' ');
            func.code.push_str(match op
            {
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::BSL => "<<",
                BinOp::BSR => ">>",
                BinOp::LT => "<",
                BinOp::GT => ">",
                BinOp::LEQ => "<=",
                BinOp::GEQ => ">=",
                BinOp::EQ => "==",
                BinOp::NEQ => "!=",
                BinOp::In => panic!("unsupported operator!"),
                BinOp::And => "&",
                BinOp::Or => "|",
                BinOp::Xor => "^",
                BinOp::BoolXor => "^"
            });
            func.code.push(' ');
            func.code.push_str(&right);
            func.code.push_str(";\n");

            name
        }

        // Boolean and
        SExpr::And(m, l, r) => {
            // Get name and left operand
            let left = convert_sexpr(l, root, func, types);
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str(get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(&left);
            func.code.push_str(";\nif (");
            func.code.push_str(&name);
            func.code.push_str(") {\n");
            let right = convert_sexpr(r, root, func, types);
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(&right);
            func.code.push_str(";\n}\n");

            name
        }

        // Boolean or
        SExpr::Or(m, l, r) => {
            // Get name and left operand
            let left = convert_sexpr(l, root, func, types);
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Generate code
            func.code.push_str(get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(&left);
            func.code.push_str(";\nif (!");
            func.code.push_str(&name);
            func.code.push_str(") {\n");
            let right = convert_sexpr(r, root, func, types);
            func.code.push_str(&name);
            func.code.push_str(" = ");
            func.code.push_str(&right);
            func.code.push_str(";\n}\n");

            name
        }

        // As operator
        SExpr::As(m, v) => {
            // Get value and types
            let value = convert_sexpr(v, root, func, types);

            let mut _type = &m._type;
            while let Type::Symbol(s) = _type
            {
                _type = root.types.get(s).unwrap();
            }

            let mut vtype = &v.get_metadata()._type;
            while let Type::Symbol(s) = vtype
            {
                vtype = root.types.get(s).unwrap();
            }

            // Check if types are equal
            if _type == vtype
            {
                return value;
            }

            // Get name and value
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            func.code.push_str(get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);

            // Check result type
            match &_type
            {
                // Sum types
                Type::Sum(_) => {
                    func.code.push_str(";\n");
                    func.code.push_str(&name);
                    func.code.push_str(".tag = ");
                    let ctype = types.get(&_type).unwrap();
                    let id = ctype.get_hashmap().unwrap().get(&v.get_metadata()._type).unwrap();
                    func.code.push_str(&format!("{}", id));
                    func.code.push_str(";\n");
                    func.code.push_str(&name);
                    func.code.push_str(".values.$$");
                    func.code.push_str(&format!("{}", id));
                    func.code.push_str(" = ");
                    func.code.push_str(&value);
                    func.code.push_str(";\n");
                }

                // Everything else is unsupported
                _ => panic!("unsupported type!")
            }

            name
        }

        // If expressions
        SExpr::If(m, c, b, e) => {
            // Get name
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Declare variable
            func.code.push_str(&get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);
            func.code.push_str(";\n");

            // Get condition
            let cond = convert_sexpr(c, root, func, types);
            func.code.push_str("if (");
            func.code.push_str(&cond);
            func.code.push_str(") {\n");

            // Get types
            let mut mtype = &m._type;
            while let Type::Symbol(s) = mtype
            {
                mtype = root.types.get(s).unwrap();
            }
            let mut btype = &b.get_metadata()._type;
            while let Type::Symbol(s) = btype
            {
                btype = root.types.get(s).unwrap();
            }
            let mut etype = &e.get_metadata()._type;
            while let Type::Symbol(s) = etype
            {
                etype = root.types.get(s).unwrap();
            }

            // Get body
            let body = convert_sexpr(b, root, func, types);

            // Save
            if mtype == btype
            {
                func.code.push_str(&name);
                func.code.push_str(" = ");
                func.code.push_str(&body);
            } else
            {
                let id = types.get(mtype).unwrap().get_hashmap().unwrap().get(&b.get_metadata()._type).unwrap();
                func.code.push_str(&name);
                func.code.push_str(".tag = ");
                func.code.push_str(&format!("{}", id));
                func.code.push_str(";\n");
                func.code.push_str(&name);
                func.code.push_str(".values.$$");
                func.code.push_str(&format!("{}", id));
                func.code.push_str(" = ");
                func.code.push_str(&body);
            }
            func.code.push_str(";\n} else {\n");

            // Get else clause
            let elsy = convert_sexpr(e, root, func, types);

            // Save
            if mtype == etype
            {
                func.code.push_str(&name);
                func.code.push_str(" = ");
                func.code.push_str(&elsy);
            } else if let Type::Sum(_) = etype
            {
                let map = types.get(mtype).unwrap().get_hashmap().unwrap();
                let subtype = types.get(etype).unwrap();
                let submap = subtype.get_hashmap().unwrap();
                func.code.push_str("switch (");
                func.code.push_str(&elsy);
                func.code.push_str(".tag) {\n");

                for s in submap
                {
                    func.code.push_str("case ");
                    func.code.push_str(&format!("{}:\n", s.1));
                    func.code.push_str(&name);
                    func.code.push_str(".tag = ");
                    func.code.push_str(&format!("{};\n", map.get(s.0).unwrap()));
                    func.code.push_str(&name);
                    func.code.push_str(".values.$$");
                    func.code.push_str(&format!("{}", map.get(s.0).unwrap()));
                    func.code.push_str(" = ");
                    func.code.push_str(&elsy);
                    func.code.push_str(".values.$$");
                    func.code.push_str(&format!("{}", s.1));
                    func.code.push_str(";\nbreak;\n");
                }
                func.code.push_str("}\n");
            } else
            {
                let id = types.get(&m._type).unwrap().get_hashmap().unwrap().get(&e.get_metadata()._type).unwrap();
                func.code.push_str(&name);
                func.code.push_str(".tag = ");
                func.code.push_str(&format!("{}", id));
                func.code.push_str(";\n");
                func.code.push_str(&name);
                func.code.push_str(".values.$$");
                func.code.push_str(&format!("{}", id));
                func.code.push_str(" = ");
                func.code.push_str(&elsy);
            }
            func.code.push_str(";\n}\n");

            name
        }

        // Applications
        SExpr::Application(_, l, r) => {
            // Get the list of arguments and the function
            let mut args = vec![r];
            let mut funcs = vec![sexpr];
            let mut f = &**l;
            while let SExpr::Application(_, l, r) = f
            {
                args.insert(0, r);
                funcs.insert(0, f);
                f = l;
            }

            // Quick hack for debug function
            // TODO: Make this not a hack
            if let SExpr::Symbol(_, v) = f
            {
                if v == "debug"
                {
                    let arg = convert_sexpr(&args[0], root, func, types);
                    put_debug_fn(&mut func.code, &arg, &args[0].get_metadata()._type, root, types, true);
                    if args.len() == 1
                    {
                        return arg;
                    } else
                    {
                        panic!("Using debug on multiple arguments is not supported");
                    }
                } else if v == "putch"
                {
                    let arg = convert_sexpr(&args[0], root, func, types);
                    func.code.push_str("printf(\"%c\", (char) ");
                    func.code.push_str(&arg);
                    func.code.push_str(");\n");
                    return arg;
                }
            }

            let mut _type = &f.get_metadata()._type;
            while let Type::Symbol(s) = _type
            {
                _type = root.types.get(s).unwrap();
            }

            let mut ftype = &f.get_metadata()._type;

            let mut unknown_arity = false;
            match _type
            {
                // Functions
                Type::Func(_, _) => {
                    // Get function and args
                    let mut fstr = if let SExpr::Function(m, _) = f
                    {
                        if m.arity == 0 || m.saved_argc.is_none() || m.saved_argc.unwrap() != 0
                        {
                            convert_sexpr(f, root, func, types)
                        } else
                        {
                            String::with_capacity(0)
                        }
                    } else
                    {
                        convert_sexpr(f, root, func, types)
                    };

                    let mut astrs = vec![];
                    let mut name = String::with_capacity(0);
                    for a in args.iter().enumerate()
                    {
                        // Get argument
                        let (n, a) = a;
                        let mut v = convert_sexpr(a, root, func, types);
                        let mut _type = &a.get_metadata()._type;
                        while let Type::Symbol(s) = _type
                        {
                            _type = root.types.get(s).unwrap();
                        }

                        while let Type::Symbol(s) = ftype
                        {
                            ftype = root.types.get(s).unwrap();
                        }

                        let mut arg_type = &**if let Type::Func(a, b) = &ftype
                        {
                            ftype = b;
                            a
                        } else { unreachable!("this is always a function") };
                        while let Type::Symbol(s) = arg_type
                        {
                            arg_type = root.types.get(s).unwrap();
                        }

                        if !f.get_metadata().tailrec
                        {
                            match arg_type
                            {
                                Type::Float => {
                                    let name = format!("$${}", func.last_reference);
                                    func.last_reference += 1;
                                    func.code.push_str("double_wrapper_t ");
                                    func.code.push_str(&name);
                                    func.code.push_str(";\n");
                                    func.code.push_str(&name);
                                    func.code.push_str(".d = ");
                                    func.code.push_str(&v);
                                    func.code.push_str(";\nvoid* ");
                                    v = format!("$${}", func.last_reference);
                                    func.last_reference += 1;
                                    func.code.push_str(&v);
                                    func.code.push_str(" = ");
                                    func.code.push_str(&name);
                                    func.code.push_str(".v;\n");
                                }

                                Type::Func(_, _) => {
                                    v = format!("&{}", &v);
                                }

                                Type::Sum(_) => {
                                    // Autocast argument if not already done so
                                    if _type == arg_type
                                    {
                                        v = format!("&{}", &v);
                                    } else if let Type::Sum(_) = _type
                                    {
                                        let name = format!("$${}", func.last_reference);
                                        func.last_reference += 1;
                                        let map = types.get(arg_type).unwrap().get_hashmap().unwrap();
                                        let subtype = types.get(_type).unwrap();
                                        let submap = subtype.get_hashmap().unwrap();
                                        func.code.push_str(types.get(arg_type).unwrap().get_c_name());
                                        func.code.push(' ');
                                        func.code.push_str(&name);
                                        func.code.push_str(";\n");
                                        func.code.push_str("switch (");
                                        func.code.push_str(&v);
                                        func.code.push_str(".tag) {\n");

                                        for s in submap
                                        {
                                            func.code.push_str("case ");
                                            func.code.push_str(&format!("{}:\n", s.1));
                                            func.code.push_str(&name);
                                            func.code.push_str(".tag = ");
                                            func.code.push_str(&format!("{};\n", map.get(s.0).unwrap()));
                                            func.code.push_str(&name);
                                            func.code.push_str(".values.$$");
                                            func.code.push_str(&format!("{}", map.get(s.0).unwrap()));
                                            func.code.push_str(" = ");
                                            func.code.push_str(&v);
                                            func.code.push_str(".values.$$");
                                            func.code.push_str(&format!("{}", s.1));
                                            func.code.push_str(";\nbreak;\n");
                                        }
                                        func.code.push_str("}\n");
                                        v = format!("&{}", name);
                                    } else
                                    {
                                        let name = format!("$${}", func.last_reference);
                                        func.last_reference += 1;
                                        let arg_ctype = types.get(arg_type).unwrap();
                                        func.code.push_str(arg_ctype.get_c_name());
                                        func.code.push(' ');
                                        func.code.push_str(&name);
                                        func.code.push_str(";\n");
                                        func.code.push_str(&name);
                                        func.code.push_str(".tag = ");
                                        func.code.push_str(&format!("{};\n", arg_ctype.get_hashmap().unwrap().get(_type).unwrap()));
                                        func.code.push_str(&name);
                                        func.code.push_str(".values.");
                                        func.code.push_str(&format!("$${}", arg_ctype.get_hashmap().unwrap().get(_type).unwrap()));
                                        func.code.push_str(" = ");
                                        func.code.push_str(&v);
                                        func.code.push_str(";\n");
                                        v = format!("&{}", name);
                                    }
                                }

                                _ => ()
                            }
                        }

                        // Functions with unknown arity
                        if f.get_metadata().arity == 0 || f.get_metadata().saved_argc.is_none()
                        {
                            // Check for function list if just got into this state of unknown arity
                            if !unknown_arity
                            {
                                unknown_arity = true;

                                // Init list
                                func.code.push_str("if (");
                                func.code.push_str(&fstr);
                                func.code.push_str(".cleaners == (void*) 0)\n");
                                func.code.push_str(&fstr);
                                func.code.push_str(".cleaners = calloc(");
                                func.code.push_str(&fstr);
                                func.code.push_str(".arity, sizeof(void*));\n");
                                func.code.push_str("if (");
                                func.code.push_str(&fstr);
                                func.code.push_str(".args == (void*) 0)\n");
                                func.code.push_str(&fstr);
                                func.code.push_str(".args = calloc(");
                                func.code.push_str(&fstr);
                                func.code.push_str(".arity, sizeof(void*));\n");
                            }

                            if let Type::Func(_, _) = arg_type
                            {
                                let name = format!("$${}", func.last_reference);
                                func.last_reference += 1;
                                func.code.push_str("func_t* ");
                                func.code.push_str(&name);
                                func.code.push_str(" = copy_func_arg(");
                                func.code.push_str(&v);
                                func.code.push_str(");\n");
                                v = name;
                                func.code.push_str(&fstr);
                                func.code.push_str(".cleaners[");
                                func.code.push_str(&fstr);
                                func.code.push_str(".argc] = force_free_func;\n");
                            } else if let Type::Sum(_) = arg_type
                            {
                                let name = format!("$${}", func.last_reference);
                                func.last_reference += 1;
                                let type_name = types.get(_type).unwrap().get_c_name();
                                func.code.push_str(type_name);
                                func.code.push_str("* ");
                                func.code.push_str(&name);
                                func.code.push_str(" = malloc(sizeof(");
                                func.code.push_str(type_name);
                                func.code.push_str("));\n*");
                                func.code.push_str(&name);
                                func.code.push_str(" = ");
                                func.code.push_str(&v[1..]);
                                func.code.push_str(";\n");
                            }

                            // Save argument
                            func.code.push_str(&fstr);
                            func.code.push_str(".args[");
                            func.code.push_str(&fstr);
                            func.code.push_str(".argc++] = (void*) ");
                            func.code.push_str(&v);
                            func.code.push_str(";\n");

                            // Call the function
                            let _type = &funcs[n].get_metadata()._type;
                            name = format!("$${}", func.last_reference);
                            func.last_reference += 1;
                            func.code.push_str(get_c_type(_type, types));
                            func.code.push(' ');
                            func.code.push_str(&name);
                            if let Type::Func(_, _) = _type
                            {
                                func.code.push_str(" = ");
                                func.code.push_str(&fstr);
                            }

                            func.code.push_str(";\nif (");
                            func.code.push_str(&fstr);
                            func.code.push_str(".arity == ");
                            func.code.push_str(&fstr);
                            func.code.push_str(".argc) {\n");
                            func.code.push_str(&name);
                            func.code.push_str(" = ((");
                            func.code.push_str(get_c_type(_type, types));
                            func.code.push_str(" (*)(func_t*))");
                            func.code.push_str(&fstr);
                            func.code.push_str(".wrapper)(&");
                            func.code.push_str(&fstr);
                            func.code.push_str(");\n");

                            // Reset the list of arguments
                            if n != args.len() - 1
                            {
                                fstr = name.clone();
                                func.code.push_str(";\n");
                                func.code.push_str("if (");
                                func.code.push_str(&fstr);
                                func.code.push_str(".args == (void*) 0)\n");
                                func.code.push_str(&fstr);
                                func.code.push_str(".args = calloc(");
                                func.code.push_str(&fstr);
                                func.code.push_str(".arity, sizeof(void*));\n");
                                func.code.push_str("if (");
                                func.code.push_str(&fstr);
                                func.code.push_str(".cleaners == (void*) 0)\n");
                                func.code.push_str(&fstr);
                                func.code.push_str(".cleaners = calloc(");
                                func.code.push_str(&fstr);
                                func.code.push_str(".arity, sizeof(void*));\n");
                            }

                            func.code.push_str("}\n");

                            f = funcs[n];

                        // Functions with known arity and fully applied
                        } else if f.get_metadata().arity <= astrs.len() + 1
                        {
                            // Get name
                            astrs.push((v, arg_type));
                            name = format!("$${}", func.last_reference);
                            func.last_reference += 1;

                            if f.get_metadata().tailrec
                            {
                                for a in func.args.iter().enumerate()
                                {
                                    func.code.push_str(&sanitise_symbol(a.1.0));
                                    func.code.push_str(" = ");
                                    func.code.push_str(&astrs[a.0].0);
                                    func.code.push_str(";\n");
                                }

                                func.code.push_str("$$LOOP$$ = 1;\n");
                                func.code.push_str(get_c_type(ftype, types));
                                func.code.push(' ');
                                func.code.push_str(&name);
                                func.code.push_str(";\n");
                            } else
                            {
                                let saved_argc = f.get_metadata().saved_argc.unwrap();
                                func.code.push_str(get_c_type(ftype, types));
                                func.code.push(' ');
                                func.code.push_str(&name);

                                if fstr == ""
                                {
                                    // Get function name
                                    fstr = format!("{}$FUNC$$", if let SExpr::Function(_, f) = f { sanitise_symbol(f) } else { unreachable!("always a function"); });
                                    func.code.push_str(" = ");
                                    func.code.push_str(&fstr);
                                    func.code.push('(');

                                    // Pass arguments
                                    for i in 0..f.get_metadata().arity
                                    {
                                        if i != 0 || saved_argc > 0
                                        {
                                            func.code.push_str(", ");
                                        }

                                        func.code.push_str(&astrs[i].0);
                                    }

                                    // Close parentheses
                                    func.code.push_str(");\n");
                                } else
                                {
                                    func.code.push_str(" = ((");

                                    // Create function pointer
                                    func.code.push_str(get_c_type(ftype, types));
                                    func.code.push_str(" (*)(");
                                    for i in 0..saved_argc + f.get_metadata().arity
                                    {
                                        if i != 0
                                        {
                                            func.code.push_str(", ");
                                        }
                                        func.code.push_str("void*");
                                    }

                                    // Call the function
                                    func.code.push_str(")) ");
                                    func.code.push_str(&fstr);
                                    func.code.push_str(".func)(");

                                    // Print saved arguments
                                    for i in 0..saved_argc
                                    {
                                        if i != 0
                                        {
                                            func.code.push_str(", ");
                                        }

                                        func.code.push_str(&fstr);
                                        func.code.push_str(&format!(".args[{}]", i));
                                    }

                                    // Pass new arguments
                                    for i in 0..f.get_metadata().arity
                                    {
                                        if i != 0 || saved_argc > 0
                                        {
                                            func.code.push_str(", ");
                                        }

                                        func.code.push_str("(void*) ");
                                        func.code.push_str(&astrs[i].0);
                                    }

                                    // Close parentheses
                                    func.code.push_str(");\n");
                                }
                            }

                            if n < funcs.len()
                            {
                                f = funcs[n];
                            }

                            for a in astrs.iter()
                            {
                                match a.1
                                {
                                    Type::Func(_, _) => {
                                    }

                                    _ => ()
                                }
                            }

                            astrs.clear();

                            if n < args.len() - 1
                            {
                                fstr = name.clone();
                            }
                        } else
                        {
                            astrs.push((v, arg_type));
                        }
                    }

                    // Currying
                    if astrs.len() > 0
                    {
                        if fstr == ""
                        {
                            fstr = convert_sexpr(f, root, func, types)
                        }

                        // Get name
                        name = format!("$${}", func.last_reference);
                        func.last_reference += 1;

                        // Create new function structure
                        func.code.push_str("func_t ");
                        func.code.push_str(&name);
                        func.code.push_str(";\ncopy_func(&");
                        func.code.push_str(&name);
                        func.code.push_str(", &");
                        func.code.push_str(&fstr);
                        func.code.push_str(");\n");

                        // Init list
                        func.code.push_str("if (");
                        func.code.push_str(&name);
                        func.code.push_str(".cleaners == (void*) 0)\n");
                        func.code.push_str(&name);
                        func.code.push_str(".cleaners = calloc(");
                        func.code.push_str(&name);
                        func.code.push_str(".arity, sizeof(void*));\n");
                        func.code.push_str("if (");
                        func.code.push_str(&name);
                        func.code.push_str(".args == (void*) 0)\n");
                        func.code.push_str(&name);
                        func.code.push_str(".args = calloc(");
                        func.code.push_str(&name);
                        func.code.push_str(".arity, sizeof(void*));\n");

                        // Put in new args
                        for arg in astrs.iter_mut()
                        {
                            // If it's a function or sum type allocate space in the heap for it
                            if let Type::Func(_, _) = arg.1
                            {
                                {
                                    let name = format!("$${}", func.last_reference);
                                    func.last_reference += 1;
                                    func.code.push_str("func_t* ");
                                    func.code.push_str(&name);
                                    func.code.push_str(" = copy_func_arg(");
                                    func.code.push_str(&arg.0);
                                    func.code.push_str(");\n");
                                    func.code.push_str(&name);
                                    func.code.push_str("->refc++;\n");
                                    arg.0 = name;
                                }

                                func.code.push_str(&name);
                                func.code.push_str(".cleaners[");
                                func.code.push_str(&name);
                                func.code.push_str(".argc] = force_free_func;\n");
                            } else if let Type::Sum(_) = arg.1
                            {
                                let name = format!("$${}", func.last_reference);
                                func.last_reference += 1;
                                let type_name = types.get(arg.1).unwrap().get_c_name();
                                func.code.push_str(type_name);
                                func.code.push_str("* ");
                                func.code.push_str(&name);
                                func.code.push_str(" = malloc(sizeof(");
                                func.code.push_str(type_name);
                                func.code.push_str("));\n*");
                                func.code.push_str(&name);
                                func.code.push_str(" = ");
                                func.code.push_str(&arg.0[1..]);
                                func.code.push_str(";\n");
                                arg.0 = name;
                            }

                            func.code.push_str(&name);
                            func.code.push_str(".args[");
                            func.code.push_str(&name);
                            func.code.push_str(".argc++] = (void*) ");
                            func.code.push_str(&arg.0);
                            func.code.push_str(";\n");
                        }
                    }

                    name
                }

                // String and list concatenation is unsupported at the moment
                _ => panic!("unsupported application of `{}` on {:?}!", _type, sexpr)
            }
        }

        // Assignments
        SExpr::Assign(m, a, v) => {
            let _type =
                if let Type::Symbol(_) = m._type
                {
                    types.get(&m._type).unwrap().get_curly_type()
                } else
                {
                    &m._type
                };
            let vtype =
                if let Type::Symbol(_) = v.get_metadata()._type
                {
                    types.get(&v.get_metadata()._type).unwrap().get_curly_type()
                } else
                {
                    &v.get_metadata()._type
                };
            let a = &sanitise_symbol(a);

            match _type
            {
                Type::Sum(_) if _type != vtype => {
                    // Get value and generate code
                    let val = convert_sexpr(v, root, func, types);
                    let map = types.get(&m._type).unwrap().get_hashmap().unwrap();
                    func.code.push_str(get_c_type(&m._type, types));
                    func.code.push(' ');
                    func.code.push_str(a);
                    func.code.push_str(";\n");
                    func.code.push_str(a);
                    func.code.push_str(".tag = ");
                    let id = format!("{}", map.get(&v.get_metadata()._type).unwrap());
                    func.code.push_str(&id);
                    func.code.push_str(";\n");
                    func.code.push_str(a);
                    func.code.push_str(".values.$$");
                    func.code.push_str(&id);
                    func.code.push_str(" = ");
                    func.code.push_str(&val);
                    func.code.push_str(";\n");

                    a.clone()
                }

                _ => {
                    // Get value and generate code
                    let val = convert_sexpr(v, root, func, types);
                    func.code.push_str(get_c_type(&m._type, types));
                    func.code.push(' ');
                    func.code.push_str(a);
                    func.code.push_str(" = ");
                    func.code.push_str(&val);
                    func.code.push_str(";\n");

                    // Increment reference counter
                    match m._type
                    {
                        Type::Func(_, _) => {
                            func.code.push_str(&a);
                            func.code.push_str(".refc++;\n");
                        }

                        _ => ()
                    }

                    a.clone()
                }
            }
        }

        // With expressions
        SExpr::With(m, a, b) => {
            // Get name
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;

            // Declare variable
            func.code.push_str(&get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);
            func.code.push_str(";\n{\n");

            // Assignments
            let mut astrs = vec![];
            for a in a
            {
                astrs.push(convert_sexpr(a, root, func, types));
            }

            // Body
            let body = convert_sexpr(b, root, func, types);
            let ptr = format!("$${}", func.last_reference);
            func.last_reference += 1;
            func.code.push_str(&get_c_type(&m._type, types));
            func.code.push_str("* ");
            func.code.push_str(&ptr);
            func.code.push_str(" = &");
            func.code.push_str(&body);
            func.code.push_str(";\n");

            // Increment body reference count
            match m._type
            {
                Type::Func(_, _) => {
                    func.code.push_str(&ptr);
                    func.code.push_str("->refc++;\n");
                }

                _ => ()
            }

            // Decrement assignment reference counts and free if necessary
            for a in a.iter().enumerate()
            {
                match a.1.get_metadata()._type
                {
                    Type::Func(_, _) => {
                        func.code.push_str("refc_func(&");
                        func.code.push_str(&astrs[a.0]);
                        func.code.push_str(");\n");
                    }

                    _ => ()
                }
            }

            // Decrement body reference count
            match m._type
            {
                Type::Func(_, _) => {
                    func.code.push_str(&ptr);
                    func.code.push_str("->refc--;\n");
                }

                _ => ()
            }

            // Copy data from ptr
            func.code.push_str(&name);
            func.code.push_str(" = *");
            func.code.push_str(&ptr);

            // Exit block and return success
            func.code.push_str(";\n}\n");
            name
        }

        SExpr::Match(m, v, a) => {
            // Get value and name
            let value = convert_sexpr(v, root, func, types);
            let name = format!("$${}", func.last_reference);
            func.last_reference += 1;
            let _type = types.get(&v.get_metadata()._type).unwrap();
            let map = _type.get_hashmap().unwrap();

            // Create switch statement
            func.code.push_str(get_c_type(&m._type, types));
            func.code.push(' ');
            func.code.push_str(&name);
            func.code.push_str(";\nswitch (");
            func.code.push_str(&value);
            func.code.push_str(".tag) {\n");

            let mut mtype = &m._type;
            while let Type::Symbol(s) = mtype
            {
                mtype = root.types.get(s).unwrap();
            }

            // Create match arms
            for a in a.iter()
            {
                let mut _type = &a.0;
                while let Type::Symbol(s) = _type
                {
                    _type = root.types.get(s).unwrap();
                }

                if let Type::Sum(_) = _type
                {
                    let subtype = types.get(_type).unwrap();
                    let submap = subtype.get_hashmap().unwrap();
                    for s in submap
                    {
                        func.code.push_str("case ");
                        func.code.push_str(&format!("{}:\n", map.get(&s.0).unwrap()));
                    }

                    func.code.push_str("{\n");
                    func.code.push_str(subtype.get_c_name());
                    func.code.push_str(" $$;\nswitch (");
                    func.code.push_str(&value);
                    func.code.push_str(".tag) {\n");

                    for s in submap
                    {
                        func.code.push_str("case ");
                        func.code.push_str(&format!("{}:\n", map.get(s.0).unwrap()));
                        func.code.push_str("$$.tag = ");
                        func.code.push_str(&format!("{};\n", s.1));
                        func.code.push_str("$$.values.$$");
                        func.code.push_str(&format!("{}", s.1));
                        func.code.push_str(" = ");
                        func.code.push_str(&value);
                        func.code.push_str(".values.$$");
                        func.code.push_str(&format!("{}", map.get(s.0).unwrap()));
                        func.code.push_str(";\nbreak;\n");
                    }
                    func.code.push_str("}\n");
                    if let SExpr::Symbol(_, s) = &**v
                    {
                        func.code.push_str(subtype.get_c_name());
                        func.code.push(' ');
                        func.code.push_str(s);
                        func.code.push_str(" = $$;\n");
                    }
                } else if let Type::Enum(_) = _type
                {
                    let id = map.get(_type).unwrap();
                    func.code.push_str("case ");
                    func.code.push_str(&format!("{}: {{\n", id));
                } else
                {
                    let id = map.get(_type).unwrap();
                    func.code.push_str("case ");
                    func.code.push_str(&format!("{}: {{\n", id));
                    func.code.push_str(get_c_type(&a.0, types));
                    func.code.push_str(" $$MATCHTEMP$$ = ");
                    let name = if let SExpr::Symbol(_, s) = &**v
                    {
                        s
                    } else
                    {
                        "$"
                    };
                    let name = &sanitise_symbol(name);
                    func.code.push_str(&value);
                    func.code.push_str(".values.$$");
                    func.code.push_str(&format!("{}", id));
                    func.code.push_str(";\n");
                    func.code.push_str(get_c_type(&a.0, types));
                    func.code.push(' ');
                    func.code.push_str(name);
                    func.code.push_str(" = $$MATCHTEMP$$;\n");
                }

                let arm = convert_sexpr(&a.1, root, func, types);

                let mut atype = &a.1.get_metadata()._type;
                while let Type::Symbol(s) = atype
                {
                    atype = root.types.get(s).unwrap();
                }

                if atype == mtype
                {
                    func.code.push_str(&name);
                    func.code.push_str(" = ");
                    func.code.push_str(&arm);
                    func.code.push_str(";\n");
                } else
                {
                    let _type = types.get(&m._type).unwrap();
                    let map = _type.get_hashmap().unwrap();
                    let id = map.get(atype).unwrap();
                    func.code.push_str(&name);
                    func.code.push_str(".tag = ");
                    func.code.push_str(&format!("{}", id));
                    func.code.push_str(";\n");
                    func.code.push_str(&name);
                    func.code.push_str(".values.$$");
                    func.code.push_str(&format!("{}", id));
                    func.code.push_str(" = ");
                    func.code.push_str(&arm);
                    func.code.push_str(";\n");
                }

                func.code.push_str("break;\n}\n");
            }
            func.code.push_str("}\n");

            name
        }

        SExpr::MemberAccess(m, a) => {
            let mut _type = &m._type;
            while let Type::Symbol(s) = _type
            {
                _type = root.types.get(s).unwrap();
            }

            if let Type::Sum(_) = &_type
            {
                let name = format!("$${}", func.last_reference);
                func.last_reference += 1;
                let t = types.get(&m._type).unwrap();
                func.code.push_str(t.get_c_name());
                func.code.push(' ');
                func.code.push_str(&name);
                func.code.push_str(";\n");
                func.code.push_str(&name);
                func.code.push_str(".tag = ");
                func.code.push_str(&format!("{}", t.get_hashmap().unwrap().get(&Type::Enum(a[1].clone())).unwrap()));
                func.code.push_str(";\n");

                name
            } else
            {
                panic!("unimplemented member access");
            }
        }

        _ => panic!("unimplemented s expression!")
    }
}

// put_fn_wrapper(&mut String, &str, &CFunction) -> ()
// Puts the wrapper for a given function in the built string.
fn put_fn_wrapper(s: &mut String, name: &str, func: &CFunction, types: &HashMap<Type, CType>)
{
    s.push_str(get_c_type(func.ret_type, types));
    s.push(' ');
    s.push_str(&name);
    s.push_str("$$WRAPPER$$");
    s.push_str("(func_t* f) {\nreturn ((");
    s.push_str(get_c_type(func.ret_type, types));
    s.push_str(" (*) (");

    for i in 0..func.args.len()
    {
        if i != 0
        {
            s.push_str(", ");
        }

        s.push_str("void*");
    }

    s.push_str(")) f->func)(");

    for i in 0..func.args.len()
    {
        if i != 0
        {
            s.push_str(", ");
        }

        s.push_str("f->args[");
        s.push_str(&format!("{}", i));
        s.push_str("]");
    }

    s.push_str(");\n}\n");
}

// put_fn_declaration(&mut String, &CFunction, &HashMap<Type, CType>) -> ()
// Puts a function declaration in the built string.
fn put_fn_declaration(s: &mut String, func: &CFunction, types: &HashMap<Type, CType>)
{
    s.push_str(get_c_type(func.ret_type, types));
    s.push(' ');
    s.push_str(&func.name);
    s.push_str("$FUNC$$");
    s.push('(');

    let mut comma = false;
    for a in func.args.iter()
    {
        let mut _type = a.1;
        if let Type::Symbol(_) =& _type
        {
            _type = types.get(&_type).unwrap().get_curly_type();
        }

        if comma
        {
            s.push_str(", ");
        } else
        {
            comma = true;
        }

        s.push_str(get_c_type(_type, types));
        s.push(' ');
        match _type
        {
            Type::Float
                | Type::Func(_, _)
                | Type::Sum(_)
                => s.push_str("*$$"),
            _ => ()
        }

        s.push_str(&sanitise_symbol(a.0));
    }

    s.push(')');
}

// put_debug_fn(&mut String, &str, &Type, &IR, &HashMap<Type, CType>, bool) -> ()
// Puts a debug function in the built string.
fn put_debug_fn(code: &mut String, v: &str, _type: &Type, ir: &IR, types: &HashMap<Type, CType>, newline: bool)
{
    let original_type = _type;
    let mut _type = _type;
    while let Type::Symbol(v) = _type
    {
        _type = ir.types.get(v).unwrap()
    }

    match _type
    {
        // Print out primatives
        Type::Int => {
            let ptr_size = std::mem::size_of::<&char>();
            code.push_str("printf(\"");
            match ptr_size
            {
                4 => code.push_str("%i\", "),
                8 => code.push_str("%lli\", "),
                _ => panic!("unsupported pointer size {}", ptr_size)
            }
            code.push_str(v);
            code.push_str(");\n");
        }

        Type::Float => {
            code.push_str("printf(\"%.5f\", ");
            code.push_str(v);
            code.push_str(");\n");
        }

        Type::Bool => {
            code.push_str("printf(\"%s\", ");
            code.push_str(v);
            code.push_str(" ? \"true\" : \"false\");\n");
        }

        // Print out aggregate types
        Type::Func(_, _) => {
            code.push_str(&format!("printf(\"<func %p> : {}\", ", original_type));
            code.push_str(v);
            code.push_str(".func);\n");
        }

        Type::Sum(_) => {
            code.push_str("switch (");
            code.push_str(v);
            code.push_str(".tag) {\n");
            let _type = types.get(_type).unwrap();

            if let CType::Sum(_, _, fields) = _type
            {
                for field in fields.iter()
                {
                    code.push_str(&format!("case {}: {{\n", field.1));
                    put_debug_fn(code, &format!("{}.values.$${}", v, field.1), &field.0, ir, types, false);
                    code.push_str("break;\n}\n");
                }
            }
            code.push_str("}\n");
            code.push_str(&format!("printf(\" : {}\");\n", original_type));
        }

        Type::Enum(v) => {
            code.push_str("printf(\"");
            code.push_str(v);
            code.push_str("\");\n");
        }

        _ => panic!("uwu")
    }

    if newline
    {
        code.push_str("printf(\"\\n\");\n");
    }
}

// collect_types(&IR, &mut HashMap<Type, String>, &mut String) -> ()
// Collects user defined types into a string containing all type definitions.
fn collect_types(ir: &IR, types: &mut HashMap<Type, CType>, types_string: &mut String)
{
    // Iterate over every type
    let mut last_reference = 0;
    for _type in ir.types.iter().filter(|v| if let Type::Symbol(_) = v.1 { false } else { true })
    {
        match _type.1
        {
            // Primatives get mapped to old type
            Type::Int => {
                types.insert(Type::Symbol(_type.0.clone()), CType::Primative(String::from("int_t"), _type.1.clone()));
            }

            Type::Float => {
                types.insert(Type::Symbol(_type.0.clone()), CType::Primative(String::from("float_t"), _type.1.clone()));
            }

            Type::Bool => {
                types.insert(Type::Symbol(_type.0.clone()), CType::Primative(String::from("char"), _type.1.clone()));
            }

            Type::Func(_, _) => {
                types.insert(Type::Symbol(_type.0.clone()), CType::Primative(String::from("func_t"), _type.1.clone()));
            }

            // Sum types are tagged unions
            Type::Sum(v) => {
                types_string.push_str(&format!("struct $${} {{\n    unsigned int tag;\n    union {{\n", last_reference));

                let mut field_ref = 0;
                let mut iter: Vec<&Type> = v.0.iter().collect();
                while let Some(t) = iter.get(field_ref)
                {
                    let mut t = *t;
                    while let Type::Symbol(s) = t
                    {
                        t = ir.types.get(s).unwrap();
                    }

                    match t
                    {
                        Type::Int => types_string.push_str("        int_t"),
                        Type::Float => types_string.push_str("        float_t"),
                        Type::Bool => types_string.push_str("        char"),
                        Type::Func(_, _) => types_string.push_str("        func_t"),

                        Type::Enum(_) => {
                            field_ref += 1;
                            continue;
                        }

                        Type::Sum(v) => {
                            for v in v.0.iter()
                            {
                                if !iter.contains(&v)
                                {
                                    iter.push(v);
                                }
                            }
                            iter.remove(field_ref);
                            continue;
                        }

                        Type::Tag(_, t) => {
                            match &**t
                            {
                                Type::Int => types_string.push_str("        int_t"),
                                Type::Float => types_string.push_str("        float_t"),
                                Type::Bool => types_string.push_str("        char"),
                                Type::Func(_, _) => types_string.push_str("        func_t"),
                                _ => panic!("unsupported type!")
                            }
                        }

                        _ => panic!("unsupported type!")
                    }

                    types_string.push_str(&format!(" $${};\n", field_ref));
                    field_ref += 1;
                }

                // Save type definitions
                let map = HashMap::from_iter(iter.into_iter().cloned().enumerate().filter_map(|v| if let Type::Sum(_) = v.1 { None } else { Some((v.1, v.0)) }));
                let ct = CType::Sum(format!("struct $${}", last_reference), _type.1.clone(), map);
                types_string.push_str("    } values;\n};\n");
                types.insert(_type.1.clone(), ct.clone());
                types.insert(Type::Symbol(_type.0.clone()), ct);
                last_reference += 1;
            }

            _ => ()
        }
    }

    // Do symbols
    for _type in ir.types.iter()
    {
        // Symbols get mapped to last type in chain
        if let Type::Symbol(_s) = _type.1
        {
            let mut s = _s;
            let __type = loop
            {
                let _type = ir.types.get(s).unwrap();
                match _type
                {
                    Type::Symbol(v) => s = v,
                    _ => break _type
                }
            };

            types.insert(Type::Symbol(_type.0.clone()), types.get(__type).unwrap().clone());
        }
    }
}

// convert_ir_to_c(&IR, Option<&mut Vec<String>>) -> String
// Converts Curly IR to C code.
pub fn convert_ir_to_c(ir: &IR, repl_vars: Option<&Vec<String>>) -> String
{
    // Create and populate types
    let mut types = HashMap::new();
    let mut types_string = String::with_capacity(0);
    collect_types(ir, &mut types, &mut types_string);

    // Create and populate functions
    let mut funcs = HashMap::new();
    for f in ir.funcs.iter()
    {
        let mut cf = CFunction {
            name: sanitise_symbol(&f.0),
            args: f.1.captured_names.iter().map(|v| (v, f.1.captured.get(v).unwrap())).chain(f.1.args.iter().map(|v| (&v.0, &v.1))).collect(),
            ret_type: &f.1.body.get_metadata()._type,
            code: String::new(),
            last_reference: 0
        };

        // Fix doubles and functions
        for a in cf.args.iter()
        {
            let mut _type = a.1;
            while let Type::Symbol(s) = _type
            {
                _type = ir.types.get(s).unwrap();
            }

            match _type
            {
                Type::Float => {
                    // Get name
                    let name = format!("$${}", cf.last_reference);
                    cf.last_reference += 1;
                    let arg_name = sanitise_symbol(&a.0);

                    // Convert pointer to double
                    cf.code.push_str("double_wrapper_t ");
                    cf.code.push_str(&name);
                    cf.code.push_str(";\n");
                    cf.code.push_str(&name);
                    cf.code.push_str(".v = $$");
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(";\nfloat_t ");
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(" = ");
                    cf.code.push_str(&name);
                    cf.code.push_str(".d;\n");
                }

                Type::Func(_, _) => {
                    // Copy
                    let arg_name = sanitise_symbol(&a.0);
                    cf.code.push_str("func_t ");
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(";\ncopy_func(&");
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(", $$");
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(");\n");
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(".refc++;\n");
                }

                Type::Sum(_) => {
                    // Copy
                    let arg_name = sanitise_symbol(&a.0);
                    let type_name = types.get(&a.1).unwrap().get_c_name();
                    cf.code.push_str(type_name);
                    cf.code.push(' ');
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(" = *$$");
                    cf.code.push_str(&arg_name);
                    cf.code.push_str(";\n");
                }

                _ => ()
            }
        }

        if f.1.body.get_metadata().tailrec
        {
            cf.code.push_str("char $$LOOP$$ = 1;\n");
            cf.code.push_str(get_c_type(&f.1.body.get_metadata()._type, &types));
            cf.code.push_str(" $$RET$$;\nwhile ($$LOOP$$) {\n$$LOOP$$ = 0;\n");
        }

        let last = convert_sexpr(&f.1.body, ir, &mut cf, &types);

        if f.1.body.get_metadata().tailrec
        {
            cf.code.push_str("$$RET$$ = ");
            cf.code.push_str(&last);
            cf.code.push_str(";\n}\n");
        }

        // Deallocate functions
        for a in cf.args.iter()
        {
            match a.1
            {
                Type::Func(_, _) => {
                    // Delete
                    cf.code.push_str("refc_func(&");
                    cf.code.push_str(&sanitise_symbol(&a.0));
                    cf.code.push_str(");\n");
                }

                _ => ()
            }
        }

        // Return statement
        cf.code.push_str("return ");
        if f.1.body.get_metadata().tailrec
        {
            cf.code.push_str("$$RET$$");
        } else
        {
            cf.code.push_str(&last);
        }
        cf.code.push_str(";\n");

        funcs.insert(f.0, cf);
    }

    // Create the main function
    let mut main_func = CFunction {
        name: String::from(""),
        args: Vec::with_capacity(0),
        ret_type: if let Some(v) = ir.sexprs.last()
        {
            &v.get_metadata()._type
        } else
        {
            &Type::Int
        },
        code: String::new(),
        last_reference: 0
    };

    // Populate the main function
    let mut cleanup = vec![];
    for s in ir.sexprs.iter()
    {
        let v = convert_sexpr(s, ir, &mut main_func, &types);

        // Debug print
        if let Some(_) = repl_vars
        {
            put_debug_fn(&mut main_func.code, &v, &s.get_metadata()._type, ir, &types, true);
        }

        // Deallocation
        match s.get_metadata()._type
        {
            Type::Func(_, _) => {
            }

            _ => ()
        }

        cleanup.push(v);
    }

    // Define structures and helper functions
    let ptr_size = std::mem::size_of::<&char>();
    let mut code_string = format!("
typedef {} int_t;
typedef {} float_t;
",
    match ptr_size
    {
        4 => "int",
        8 => "long long",
        _ => panic!("unsupported architecture with pointer size {}", ptr_size)
    },
    match ptr_size
    {
        4 => "float",
        8 => "double",
        _ => panic!("unsupported architecture with pointer size {}", ptr_size)
    });
    code_string.push_str("
typedef struct {
    unsigned int refc;
    void* func;
    void* wrapper;
    unsigned int arity;
    unsigned int argc;
    char (**cleaners)(void*);
    void** args;
} func_t;

typedef union {
    float_t d;
    void* v;
} double_wrapper_t;

int printf(const char*, ...);

void* calloc(long unsigned int, long unsigned int);

void* malloc(long unsigned int);

void free(void*);

char force_free_func(void* _func) {
    // func_t* func = (func_t*) _func;
    // for (int i = 0; i < func->argc; i++) {
        // if (func->cleaners[i] != (void*) 0 && func->cleaners[i](func->args[i]))
            // free(func->args[i]);
    // }

    // free(func->args);
    // free(func->cleaners);
    return (char) 1;
}

char free_func(func_t* func) {
    // if (func->refc == 0) {
    //    return force_free_func(func);
    //}

    return (char) 0;
}

char refc_func(func_t* func) {
    // if (func->refc > 0)
        // func->refc--;
    return free_func(func);
}

void copy_func(func_t* dest, func_t* source) {
    dest->refc = 0;
    dest->func = source->func;
    dest->wrapper = source->wrapper;
    dest->arity = source->arity;
    dest->argc = source->argc;

    if (dest->argc != 0)
    {
        dest->cleaners = calloc(dest->arity, sizeof(void*));
        dest->args = calloc(dest->arity, sizeof(void*));
    } else
    {
        dest->cleaners = (void*) 0;
        dest->args = (void*) 0;
    }

    for (int i = 0; i < dest->argc; i++) {
        dest->cleaners[i] = source->cleaners[i];
        dest->args[i] = source->args[i];
    }
}

func_t* copy_func_arg(func_t* source) {
    func_t* dest = malloc(sizeof(func_t));
    copy_func(dest, source);
    return dest;
}
");
    code_string.push_str(&types_string);

    // Define repl value struct
    if let Some(_) = repl_vars
    {
        code_string.push_str("
typedef struct {
    unsigned int tag;
    union {
        int_t i;
        float_t d;
        char b;
        func_t f;
");

    let mut set = HashSet::with_capacity(0);
    for _type in types.iter()
    {
        let name = _type.1.get_c_name();
        if let Type::Sum(_) = &_type.0
        {
            if !set.contains(name)
            {
                code_string.push_str("        ");
                code_string.push_str(name);
                code_string.push(' ');
                code_string.push_str(&name[7..]);
                code_string.push_str(";\n");
                set.insert(name);
            }
        }
    }
    code_string.push_str(
"    } vals;
} repl_value_t;

");
    }

    // Declare all functions
    for f in funcs.iter()
    {
        put_fn_declaration(&mut code_string, &f.1, &types);
        code_string.push_str(";\n");
        put_fn_wrapper(&mut code_string, f.0, &f.1, &types);
    }

    // Put all function definitions
    for f in funcs
    {
        put_fn_declaration(&mut code_string, &f.1, &types);
        code_string.push_str(" {\n");
        code_string.push_str(&f.1.code);
        code_string.push_str("}\n");
    }

    // Retrieve previous arguments
    if let Some(vec) = &repl_vars
    {
        code_string.push_str(get_c_type(main_func.ret_type, &types));

        code_string.push_str(" __repl_line(repl_value_t** vars) {\n");
        for v in vec.iter().enumerate()
        {
            code_string.push_str(get_c_type(&ir.scope.get_var(v.1).unwrap().0, &types));
            code_string.push(' ');
            code_string.push_str(&sanitise_symbol(&v.1));
            code_string.push_str(" = vars[");
            code_string.push_str(&format!("{}", v.0));
            code_string.push_str("]->vals.");

            let mut _type = &ir.scope.get_var(v.1).unwrap().0;
            while let Type::Symbol(v) = _type
            {
                _type = ir.types.get(v).unwrap();
            }

            let c = match _type
            {
                Type::Int => "i",
                Type::Float => "d",
                Type::Bool => "b",
                Type::Func(_, _) => "f",
                Type::Sum(_) => {
                    code_string.push_str(&format!("{}", &types.get(_type).unwrap().get_c_name()[7..]));
                    ""
                }
                _ => panic!("unsupported type!")
            };
            code_string.push_str(c);

            code_string.push_str(";\n");
        }
    } else
    {
        code_string.push_str("int main() {\n");
    }

    // Main function code
    code_string.push_str(&main_func.code);

    // Deallocate everything
    for v in ir.sexprs.iter().enumerate()
    {
        if let SExpr::Assign(m, _, _) = v.1
        {
            match m._type
            {
                Type::Func(_, _) => {
                    code_string.push_str("if (");
                    code_string.push_str(&cleanup[v.0]);
                    code_string.push_str(".refc != 0) {\n");
                    code_string.push_str(&cleanup[v.0]);
                    code_string.push_str(".refc = 0;\nfree_func(&");
                    code_string.push_str(&cleanup[v.0]);
                    code_string.push_str(");\n}\n");
                }

                _ => ()
            }
        }
    }

    // End main function
    if let Some(_) = repl_vars
    {
        code_string.push_str("return ");
        match cleanup.last()
        {
            Some(v) => code_string.push_str(v),
            None => code_string.push_str("0")
        }
        code_string.push_str(";\n}\n");
    } else
    {
        code_string.push_str("return 0;\n}\n");
    }

    code_string
}
