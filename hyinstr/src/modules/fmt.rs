use crate::{
    modules::{
        Function, Instruction, Module,
        instructions::HyInstr,
        int::{IDiv, IRem, IntegerSignedness, OverflowPolicy},
        meta::ProbOperand,
        operand::{Label, Operand},
        terminator::Terminator,
    },
    types::TypeRegistry,
};

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(f, "label block_{}", self.0)
        } else {
            write!(f, "block_{}", self.0)
        }
    }
}

impl Operand {
    pub fn fmt<'a>(&'a self, module: Option<&'a Module>) -> impl std::fmt::Display + 'a {
        pub struct Fmt<'a> {
            operand: &'a Operand,
            module: Option<&'a Module>,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.operand {
                    Operand::Reg(name) => write!(f, "%{}", name),
                    Operand::Imm(constant) => write!(f, "{}", constant.fmt(self.module)),
                    Operand::Lbl(label) => write!(f, "{:#}", label),
                    Operand::Meta(meta) => write!(f, "M_{}", meta.0),
                }
            }
        }

        Fmt {
            operand: self,
            module,
        }
    }
}

impl HyInstr {
    pub fn fmt<'a>(
        &'a self,
        registry: &'a TypeRegistry,
        module: Option<&'a Module>,
    ) -> impl std::fmt::Display + Copy + 'a {
        #[derive(Clone, Copy)]
        pub struct Fmt<'a> {
            instr: &'a HyInstr,
            registry: &'a TypeRegistry,
            module: Option<&'a Module>,
        }

        impl<'a> Fmt<'a> {
            fn fmt_integer_policy(
                f: &mut std::fmt::Formatter<'_>,
                signesness: IntegerSignedness,
                overflow_policy: OverflowPolicy,
            ) -> std::fmt::Result {
                use IntegerSignedness::*;
                use OverflowPolicy::*;

                match (overflow_policy, signesness) {
                    (Panic, Signed) => write!(f, "panic signed "),
                    (Panic, Unsigned) => write!(f, "panic unsigned "),
                    (Wrap, Signed) => write!(f, "warp signed "),
                    (Wrap, Unsigned) => write!(f, "warp unsigned "),
                    (Saturate, Signed) => write!(f, "saturate signed "),
                    (Saturate, Unsigned) => write!(f, "saturate unsigned "),
                }
            }

            fn specific_fmt(
                &self,
                f: &mut std::fmt::Formatter<'_>,
            ) -> Result<bool, std::fmt::Error> {
                match self.instr {
                    HyInstr::IAdd(iadd) => {
                        Self::fmt_integer_policy(f, iadd.signedness, iadd.overflow)?;
                        Ok(false)
                    }
                    HyInstr::ISub(isub) => {
                        Self::fmt_integer_policy(f, isub.signedness, isub.overflow)?;
                        Ok(false)
                    }
                    HyInstr::IMul(imul) => {
                        Self::fmt_integer_policy(f, imul.signedness, imul.overflow)?;
                        Ok(false)
                    }
                    HyInstr::IDiv(IDiv { signedness, .. })
                    | HyInstr::IRem(IRem { signedness, .. }) => {
                        match signedness {
                            IntegerSignedness::Signed => write!(f, "signed ")?,
                            IntegerSignedness::Unsigned => write!(f, "unsigned ")?,
                        };
                        Ok(false)
                    }
                    HyInstr::ICmp(cmp) => {
                        write!(f, "{} ", cmp.op.to_str())?;
                        Ok(false)
                    }
                    HyInstr::FCmp(cmp) => {
                        write!(f, "{} ", cmp.op.to_str())?;
                        Ok(false)
                    }
                    HyInstr::ISht(isht) => {
                        use crate::modules::int::IShiftOp::*;
                        let op_variant = match isht.op {
                            Lsl => "lsl",
                            Lsr => "lsr",
                            Asr => "asr",
                            Rol => "rol",
                            Ror => "ror",
                        };
                        write!(f, "{} ", op_variant)?;
                        Ok(false)
                    }
                    HyInstr::MLoad(load) => {
                        if load.volatile {
                            write!(f, "volatile ")?;
                        }

                        write!(
                            f,
                            "{} {}",
                            self.registry.fmt(load.ty),
                            load.addr.fmt(self.module),
                        )?;

                        if let Some(ordering) = &load.ordering {
                            write!(f, ", atomic {}", ordering.to_str())?;
                        }

                        if let Some(alignment) = load.alignement {
                            write!(f, ", align {}", alignment)?;
                        }

                        Ok(true)
                    }
                    HyInstr::MStore(store) => {
                        if store.volatile {
                            write!(f, "volatile ")?;
                        }

                        write!(
                            f,
                            "{}, {}",
                            store.addr.fmt(self.module),
                            store.value.fmt(self.module)
                        )?;

                        if let Some(ordering) = &store.ordering {
                            write!(f, ", atomic {}", ordering.to_str())?;
                        }

                        if let Some(alignment) = store.alignment {
                            write!(f, ", align {} ", alignment)?;
                        }

                        Ok(true)
                    }
                    HyInstr::MAlloca(malloca) => {
                        write!(
                            f,
                            "{} {}",
                            self.registry.fmt(malloca.ty),
                            malloca.count.fmt(self.module)
                        )?;

                        if let Some(alignment) = malloca.alignment {
                            write!(f, ", align {} ", alignment)?;
                        }

                        Ok(true)
                    }
                    HyInstr::Phi(phi) => {
                        write!(f, "{} ", self.registry.fmt(phi.ty))?;
                        let mut first = true;
                        for (operand, label) in &phi.values {
                            if first {
                                first = false;
                            } else {
                                write!(f, ", ")?;
                            }
                            write!(f, "[ {}, {} ]", label, operand.fmt(self.module))?;
                        }
                        Ok(true)
                    }
                    HyInstr::MetaProb(prob) => {
                        match &prob.operand {
                            ProbOperand::Probability(operand) => {
                                write!(f, "prob {}", operand.fmt(self.module))?;
                            }
                            ProbOperand::ExpectedValue(operand) => {
                                write!(f, "ev {}", operand.fmt(self.module))?;
                            }
                            ProbOperand::Variance(operand) => {
                                write!(f, "var {}", operand.fmt(self.module))?;
                            }
                            ProbOperand::ProbabilityReachability => write!(f, "rch")?,
                            ProbOperand::ExpectedIterations => write!(f, "eit")?,
                        };
                        Ok(true)
                    }
                    HyInstr::Invoke(invoke) => {
                        if let Some(cconv) = &invoke.cconv {
                            write!(f, "{} ", cconv.to_string())?;
                        }

                        if let Some(ty) = invoke.ty {
                            write!(f, "{} ", self.registry.fmt(ty))?;
                        } else {
                            debug_assert!(invoke.dest.is_none());
                            write!(f, "void ")?;
                        }

                        write!(f, "{}(", invoke.function.fmt(self.module))?;

                        let mut first = true;
                        for arg in &invoke.args {
                            if first {
                                first = false;
                            } else {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", arg.fmt(self.module))?;
                        }
                        write!(f, ")")?;
                        Ok(true)
                    }
                    _ => Ok(false),
                }
            }
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let opname = self.instr.op().opname();

                if let Some(dest) = self.instr.destination() {
                    write!(f, "%{} = ", dest)?;
                }
                write!(f, "{} ", opname)?;

                // Perform specific formatting based on instruction type
                if self.specific_fmt(f)? {
                    return Ok(());
                }

                // Following match arms to be filled in with specific instruction formatting
                if let Some(dest_type) = self.instr.destination_type() {
                    write!(f, "{} ", self.registry.fmt(dest_type))?;
                }

                // Format operands
                let mut first = true;
                for operand in self.instr.operands() {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", operand.fmt(self.module))?;
                }

                Ok(())
            }
        }

        Fmt {
            instr: self,
            registry,
            module,
        }
    }
}

impl Terminator {
    pub fn fmt<'a>(&'a self, module: Option<&'a Module>) -> impl std::fmt::Display + 'a {
        struct Fmt<'a> {
            terminator: &'a Terminator,
            module: Option<&'a Module>,
        }

        impl std::fmt::Display for Fmt<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.terminator {
                    Terminator::CBranch(cbranch) => write!(
                        f,
                        "branch {}, {}, {}",
                        cbranch.cond.fmt(self.module),
                        cbranch.target_true,
                        cbranch.target_false
                    ),
                    Terminator::Jump(jump) => {
                        write!(f, "jump label {}", jump.target)
                    }
                    Terminator::Ret(ret) => {
                        if let Some(value) = &ret.value {
                            write!(f, "ret {:#}", value.fmt(self.module))
                        } else {
                            write!(f, "ret void")
                        }
                    }
                    Terminator::Trap(_) => {
                        write!(f, "trap")
                    }
                }
            }
        }

        Fmt {
            terminator: self,
            module,
        }
    }
}

impl Function {
    pub fn fmt<'a>(
        &'a self,
        type_registry: &'a TypeRegistry,
        module: Option<&'a Module>,
    ) -> impl std::fmt::Display + 'a {
        struct Fmt<'a> {
            function: &'a Function,
            type_registry: &'a TypeRegistry,
            module: Option<&'a Module>,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "define{} {} {}",
                    self.function
                        .cconv
                        .as_ref()
                        .map(|cc| format!(" {}", cc.to_string()))
                        .unwrap_or_default(),
                    self.function
                        .return_type
                        .map(|ty| self.type_registry.fmt(ty).to_string())
                        .unwrap_or("void".to_string()),
                    self.function
                        .name
                        .as_ref()
                        .map(|name| format!("%{}", name))
                        .unwrap_or(format!("%func_{}", self.function.uuid))
                )?;

                write!(f, "(")?;
                let mut first = true;
                for (param_name, param_type) in &self.function.params {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(
                        f,
                        "%{}: {}",
                        param_name,
                        self.type_registry.fmt(*param_type)
                    )?;
                }
                writeln!(f, ") {{")?;

                for (block_label, block) in &self.function.body {
                    writeln!(f, "{}:", block_label)?;
                    for instr in &block.instructions {
                        writeln!(f, "    {}", instr.fmt(self.type_registry, self.module))?;
                    }

                    writeln!(f, "    {}", block.terminator.fmt(self.module))?;
                }

                writeln!(f, "}}")?;
                Ok(())
            }
        }

        Fmt {
            function: self,
            type_registry,
            module,
        }
    }
}
