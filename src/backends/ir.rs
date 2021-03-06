use std::collections::HashMap;
use std::fmt::Display;

use super::super::frontend::ir::{self, ArityInfo, SExpr, SExprMetadata};

/// An instruction in the low level intermediate representation.
#[derive(Copy, Clone)]
pub enum IrInstruction {
    /// Returns an optional parameter from a function.
    Ret,

    /// Loads a function or argument parameter into a local.
    Load,

    /// Applies a list of arguments to a function pointer or closure struct to form a new closure
    /// struct. If passed in a closure struct, it allocates a new closure struct if the passed in
    /// closure struct has a reference count greater than 1.
    Apply,

    /// Calls a function, function pointer, or closure struct and passes the return value into a
    /// new local value. True if the arity is known at compile time, false otherwise.
    Call(bool),

    /// Increments the reference counter for a closure struct.
    RcInc,

    /// Decrements the reference counter for a closure struct and deallocates and decrements child
    /// nodes if the reference counter reaches 0.
    RcFuncFree,
}

impl Display for IrInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use IrInstruction::*;
        match self {
            Ret => write!(f, "ret"),
            Load => write!(f, "load"),
            Apply => write!(f, "apply"),
            Call(true) => write!(f, "call"),
            Call(false) => write!(f, "call?"),
            RcInc => write!(f, "rcinc"),
            RcFuncFree => write!(f, "rcfuncfree"),
        }
    }
}

/// An argument passed into an instruction in the low level intermediate representation.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum IrArgument {
    /// A local value.
    Local(usize),

    /// An argument passed into the function that contains the instruction. Closed values are also
    /// considered arguments.
    Argument(usize),

    /// A function address.
    Function(String),
}

impl Display for IrArgument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use IrArgument::*;
        match self {
            Local(l) => write!(f, "%{}", l),
            Argument(a) => write!(f, "${}", a),
            Function(g) => write!(f, "@{}", g),
        }
    }
}

/// Represents a single instruction in the lower level intermediate representation.
pub struct IrSsa {
    /// The local value the instruction is assigned to.
    pub local: Option<usize>,

    /// The lifetime of the local assigned in this statement.
    pub local_lifetime: usize,

    /// The register the local assigned to in this instruction is allocated in.
    pub local_register: usize,

    /// The instruction (ie opcode) being executed in this instruction.
    pub instr: IrInstruction,

    /// The arguments passed into the instruction.
    pub args: Vec<IrArgument>,
}

impl Display for IrSsa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(l) = self.local {
            write!(f, "%{} = ", l)?;
        }

        write!(f, "{}", self.instr)?;
        for a in self.args.iter() {
            write!(f, " {}", a)?;
        }
        Ok(())
    }
}

/// A function in the lower level intermediate representation.
pub struct IrFunction {
    /// The name of the function.
    pub name: String,

    /// The number of arguments (including closed over values) that the function takes in.
    pub argc: usize,

    /// The list of all SSAs associated with this function.
    /// TODO: Replace with basic blocks.
    pub ssas: Vec<IrSsa>,
}

impl Display for IrFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({}):", self.name, self.argc)?;
        for ssa in self.ssas.iter() {
            write!(f, "\n    {}", ssa)?;
        }
        Ok(())
    }
}

impl IrFunction {
    fn get_last_local(&self) -> Option<usize> {
        for ssa in self.ssas.iter().rev() {
            if let Some(l) = ssa.local {
                return Some(l);
            }
        }
        None
    }

    fn get_next_local(&self) -> usize {
        for ssa in self.ssas.iter().rev() {
            if let Some(l) = ssa.local {
                return l + 1;
            }
        }
        0
    }
}

/// A module in lower level intermediate representation.
/// TODO: Have a higher level data structure that represents the list of all modules in the code.
pub struct IrModule {
    /// The list of all functions in the module.
    pub funcs: Vec<IrFunction>,
}

impl Display for IrModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for func in self.funcs.iter() {
            write!(f, "{}\n\n", func)?;
        }
        Ok(())
    }
}

fn get_arg_if_applicable<'a>(
    args_map: &HashMap<String, usize>,
    sexpr: &'a SExpr,
    map: &HashMap<String, Vec<String>>,
) -> Result<IrArgument, &'a SExpr> {
    match sexpr {
        SExpr::Symbol(_, s) => {
            if let Some(a) = args_map.get(s) {
                Ok(IrArgument::Argument(*a))
            } else {
                todo!("symbols that aren't arguments");
            }
        }

        SExpr::Function(_, f) if map.get(f).unwrap().is_empty() => {
            Ok(IrArgument::Function(f.clone()))
        }

        _ => Err(sexpr),
    }
}

fn conversion_helper(
    args_map: &HashMap<String, usize>,
    func: &mut IrFunction,
    sexpr: &SExpr,
    map: &HashMap<String, Vec<String>>,
) -> Option<usize> {
    match get_arg_if_applicable(args_map, sexpr, map) {
        Ok(v) => {
            let local = Some(func.get_next_local());
            func.ssas.push(IrSsa {
                local,
                local_lifetime: 0,
                local_register: 0,
                instr: IrInstruction::Load,
                args: vec![v],
            });
            local
        }

        Err(SExpr::Empty(_)) => todo!(),
        Err(SExpr::TypeAlias(_, _)) => todo!(),

        Err(SExpr::ExternalFunc(_, _, _)) => todo!(),
        Err(SExpr::Chain(_, _, _)) => todo!(),

        Err(SExpr::Function(_, f)) => {
            use std::iter::once;
            let local = Some(func.get_next_local());
            let args = map.get(f).unwrap().iter().map(|v| {
                get_arg_if_applicable(
                    args_map,
                    &SExpr::Symbol(SExprMetadata::empty(), v.clone()),
                    map,
                )
                .unwrap()
            });
            func.ssas.push(IrSsa {
                local,
                local_lifetime: 0,
                local_register: 0,
                instr: IrInstruction::Apply,
                args: once(IrArgument::Function(f.clone())).chain(args).collect(),
            });
            local
        }

        Err(SExpr::Application(m, f, a)) => {
            let f = match get_arg_if_applicable(args_map, &**f, map) {
                Ok(v) => v,
                Err(e) => IrArgument::Local(conversion_helper(args_map, func, e, map).unwrap()),
            };

            let args: Vec<_> = a
                .iter()
                .map(|a| match get_arg_if_applicable(args_map, a, map) {
                    Ok(v) => v,
                    Err(e) => IrArgument::Local(conversion_helper(args_map, func, e, map).unwrap()),
                })
                .collect();

            use std::iter::once;
            let local = Some(func.get_next_local());
            if matches!(m.arity, ArityInfo::Known(v) if v != 0) {
                func.ssas.push(IrSsa {
                    local,
                    local_lifetime: 0,
                    local_register: 0,
                    instr: IrInstruction::Apply,
                    args: once(f).chain(args.into_iter()).collect(),
                });
            } else {
                func.ssas.push(IrSsa {
                    local,
                    local_lifetime: 0,
                    local_register: 0,
                    instr: IrInstruction::Call(matches!(m.arity, ArityInfo::Known(_))),
                    args: once(f).chain(args.into_iter()).collect(),
                });
            }

            local
        }

        Err(SExpr::Assign(_, _, _)) => todo!(),
        Err(SExpr::With(_, _, _)) => todo!(),
        Err(SExpr::Match(_, _, _)) => todo!(),

        Err(SExpr::Symbol(_, _)) => unreachable!(),
    }
}

fn calculate_lifetimes(func: &mut IrFunction) {
    let mut iter = func.ssas.iter_mut();
    let mut i = 0;
    while let Some(ssa) = iter.next() {
        if ssa.local.is_none() {
            continue;
        }
        let local = ssa.local.unwrap();

        let mut j = i + 1;
        for next in iter.as_slice() {
            for arg in next.args.iter() {
                if let IrArgument::Local(l) = arg {
                    if *l == local {
                        ssa.local_lifetime = j - i;
                        break;
                    }
                }
            }

            j += 1;
        }

        i += 1;
    }
}

fn insert_rc_instructions(func: &mut IrFunction) {
    let mut i = 0;
    let mut local_lifetimes: HashMap<IrArgument, usize> = HashMap::new();
    while let Some(mut ssa) = func.ssas.get(i) {
        if let IrInstruction::Apply = ssa.instr {
            let mut inserts = vec![];
            for arg in ssa.args.iter().skip(1) {
                if !matches!(arg, IrArgument::Function(_)) {
                    inserts.push(IrSsa {
                        local: None,
                        local_lifetime: 0,
                        local_register: 0,
                        instr: IrInstruction::RcInc,
                        args: vec![arg.clone()],
                    });
                }
            }

            for insert in inserts {
                func.ssas.insert(i, insert);
                i += 1;
            }

            ssa = func.ssas.get(i).unwrap();
            if let Some(local) = ssa.local {
                local_lifetimes.insert(IrArgument::Local(local), ssa.local_lifetime + 1);
            }
        } else if let IrInstruction::Call(_) = ssa.instr {
            if let Some(local) = ssa.local {
                local_lifetimes.insert(IrArgument::Local(local), ssa.local_lifetime + 1);
            }
        }

        if let IrInstruction::Call(false) = ssa.instr {
            let mut befores = vec![];
            let mut afters = vec![];
            for arg in ssa.args.iter().skip(1) {
                if !matches!(arg, IrArgument::Function(_)) {
                    befores.push(IrSsa {
                        local: None,
                        local_lifetime: 0,
                        local_register: 0,
                        instr: IrInstruction::RcInc,
                        args: vec![arg.clone()],
                    });
                    afters.push(IrSsa {
                        local: None,
                        local_lifetime: 0,
                        local_register: 0,
                        instr: IrInstruction::RcFuncFree,
                        args: vec![arg.clone()],
                    });
                }
            }

            let i_inc = afters.len();
            for (before, after) in befores.into_iter().zip(afters.into_iter()) {
                func.ssas.insert(i, before);
                i += 1;
                func.ssas.insert(i + 1, after);
            }
            i += i_inc;
        }

        for local in local_lifetimes.keys().cloned().collect::<Vec<_>>() {
            if i == func.ssas.len() - 1 {
                break;
            }

            let lifetime = local_lifetimes.get_mut(&local).unwrap();
            *lifetime -= 1;
            if *lifetime == 0 {
                local_lifetimes.remove(&local);
                func.ssas.insert(
                    i + 1,
                    IrSsa {
                        local: None,
                        local_lifetime: 0,
                        local_register: 0,
                        instr: IrInstruction::RcFuncFree,
                        args: vec![local],
                    },
                );
                i += 1;
            }
        }

        i += 1;
    }
}

/// Converts the frontend IR language to the backend IR language.
pub fn convert_frontend_ir_to_backend_ir(module: &ir::IrModule) -> IrModule {
    let mut new = IrModule { funcs: vec![] };

    let map: HashMap<_, _> = module
        .funcs
        .iter()
        .map(|v| (v.0.clone(), v.1.captured_names.clone()))
        .collect();
    for func in module.funcs.iter() {
        let mut f = IrFunction {
            name: func.1.name.clone(),
            argc: func.1.args.len() + func.1.captured.len(),
            ssas: vec![],
        };
        let args_map: HashMap<String, usize> = func
            .1
            .captured_names
            .iter()
            .cloned()
            .enumerate()
            .chain(func.1.args.iter().map(|v| v.0.clone()).enumerate())
            .map(|v| (v.1, v.0))
            .collect();

        conversion_helper(&args_map, &mut f, &func.1.body, &map);
        f.ssas.push(IrSsa {
            local: None,
            local_lifetime: 0,
            local_register: 0,
            instr: IrInstruction::Ret,
            args: if let Some(l) = f.get_last_local() {
                vec![IrArgument::Local(l)]
            } else {
                vec![]
            },
        });

        calculate_lifetimes(&mut f);
        insert_rc_instructions(&mut f);

        new.funcs.push(f);
    }

    new
}
