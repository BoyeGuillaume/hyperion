//! Pretty-print helpers for Hy instructions, terminators, functions, and modules.
use crate::{
    analysis::{AnalysisStatistic, TerminationScope},
    modules::{
        Function, Module,
        instructions::{
            HyInstr, Instruction,
            int::{IDiv, IRem},
            meta::MetaProbOperand,
        },
        operand::{Label, Operand},
        terminator::HyTerminator,
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
    /// Build a formatting helper that renders the operand using the given module for context.
    pub fn fmt_with<'a>(
        &'a self,
        registry: Option<&'a TypeRegistry>,
        module: Option<&'a Module>,
    ) -> impl std::fmt::Display + 'a {
        pub struct Fmt<'a> {
            operand: &'a Operand,
            registry: Option<&'a TypeRegistry>,
            module: Option<&'a Module>,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.operand {
                    Operand::Reg(name) => write!(f, "{}", name),
                    Operand::Imm(constant) => write!(f, "{}", constant.fmt(self.module)),
                    Operand::Undef(ty) => {
                        if let Some(registry) = self.registry {
                            write!(f, "{} undef", registry.fmt(*ty))
                        } else {
                            write!(f, "undef")
                        }
                    }
                }
            }
        }

        Fmt {
            operand: self,
            registry,
            module,
        }
    }

    /// Backward compatible helper that only supplies module context.
    pub fn fmt<'a>(&'a self, module: Option<&'a Module>) -> impl std::fmt::Display + 'a {
        self.fmt_with(None, module)
    }
}

impl HyInstr {
    /// Build a formatting helper that renders the instruction using the supplied registries.
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
            fn specific_fmt(
                &self,
                f: &mut std::fmt::Formatter<'_>,
            ) -> Result<bool, std::fmt::Error> {
                match self.instr {
                    HyInstr::IAdd(iadd) => {
                        write!(f, ".{} ", iadd.variant.to_str())?;
                        Ok(false)
                    }
                    HyInstr::ISub(isub) => {
                        write!(f, ".{} ", isub.variant.to_str())?;
                        Ok(false)
                    }
                    HyInstr::IMul(imul) => {
                        write!(f, ".{} ", imul.variant.to_str())?;
                        Ok(false)
                    }
                    HyInstr::IDiv(IDiv { signedness, .. })
                    | HyInstr::IRem(IRem { signedness, .. }) => {
                        write!(f, ".{} ", signedness.to_str())?;
                        Ok(false)
                    }
                    HyInstr::ICmp(cmp) => {
                        write!(f, ".{} ", cmp.variant.to_str())?;
                        Ok(false)
                    }
                    HyInstr::FCmp(cmp) => {
                        write!(f, ".{} ", cmp.variant.to_str())?;
                        Ok(false)
                    }
                    HyInstr::ISht(isht) => {
                        write!(f, ".{} ", isht.variant.to_str())?;
                        Ok(false)
                    }
                    HyInstr::MGetElementPtr(element_ptr) => {
                        write!(f, " {}, ", self.registry.fmt(element_ptr.in_ty),)?;
                        Ok(false)
                    }
                    HyInstr::MLoad(load) => {
                        if load.volatile {
                            write!(f, "volatile ")?;
                        }

                        write!(
                            f,
                            " {}",
                            load.addr.fmt_with(Some(self.registry), self.module),
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
                            store.addr.fmt_with(Some(self.registry), self.module),
                            store.value.fmt_with(Some(self.registry), self.module)
                        )?;

                        if let Some(ordering) = &store.ordering {
                            write!(f, ", atomic {}", ordering.to_str())?;
                        }

                        if let Some(alignment) = store.alignement {
                            write!(f, ", align {} ", alignment)?;
                        }

                        Ok(true)
                    }
                    HyInstr::MAlloca(malloca) => {
                        write!(
                            f,
                            "{} {}",
                            self.registry.fmt(malloca.ty),
                            malloca.count.fmt_with(Some(self.registry), self.module)
                        )?;

                        if let Some(alignment) = malloca.alignement {
                            write!(f, ", align {} ", alignment)?;
                        }

                        Ok(true)
                    }
                    HyInstr::InsertValue(insert) => {
                        write!(
                            f,
                            " {}, {}",
                            insert.aggregate.fmt_with(Some(self.registry), self.module),
                            insert.value.fmt_with(Some(self.registry), self.module)
                        )?;

                        for idx in &insert.indices {
                            write!(f, ", i32 {}", idx)?;
                        }

                        Ok(true)
                    }
                    HyInstr::ExtractValue(extract) => {
                        write!(
                            f,
                            " {}",
                            extract.aggregate.fmt_with(Some(self.registry), self.module)
                        )?;

                        for idx in &extract.indices {
                            write!(f, ", i32 {}", idx)?;
                        }

                        Ok(true)
                    }
                    HyInstr::Phi(phi) => {
                        write!(f, " ")?;
                        let mut first = true;
                        for (operand, label) in &phi.values {
                            if first {
                                first = false;
                            } else {
                                write!(f, ", ")?;
                            }
                            write!(
                                f,
                                "[{}, {}]",
                                operand.fmt_with(Some(self.registry), self.module),
                                label
                            )?;
                        }
                        Ok(true)
                    }
                    HyInstr::MetaProb(prob) => {
                        match &prob.operand {
                            MetaProbOperand::Probability(_) => write!(f, ".prob")?,
                            MetaProbOperand::ExpectedValue(_) => write!(f, ".ev")?,
                            MetaProbOperand::Variance(_) => write!(f, ".var")?,
                        };
                        Ok(false)
                    }
                    HyInstr::MetaAnalysisStat(mas) => match &mas.statistic {
                        AnalysisStatistic::ExecutionCount => {
                            write!(f, ".excnt")?;
                            Ok(true)
                        }
                        AnalysisStatistic::InstructionCount(flags) => {
                            write!(f, ".icnt i32 0x{:x}", flags.bits())?;
                            Ok(true)
                        }
                        AnalysisStatistic::TerminationBehavior(scope) => {
                            match scope {
                                TerminationScope::BlockExit => {
                                    write!(f, ".term.blockexit")?;
                                }
                                TerminationScope::FunctionExit => {
                                    write!(f, ".term.funcexit")?;
                                }
                                TerminationScope::ReachAny(labels) => {
                                    write!(f, ".term.reach ")?;
                                    let mut first = true;
                                    for label in labels {
                                        if first {
                                            first = false;
                                        } else {
                                            write!(f, ", ")?;
                                        }
                                        write!(f, "{}", label)?;
                                    }
                                }
                            }
                            Ok(true)
                        }
                    },
                    HyInstr::Invoke(invoke) => {
                        if let Some(cconv) = &invoke.cconv {
                            write!(f, " {}", cconv.to_string())?;
                        }

                        write!(
                            f,
                            " {}",
                            invoke.function.fmt_with(Some(self.registry), self.module)
                        )?;

                        for arg in &invoke.args {
                            write!(f, ", {}", arg.fmt_with(Some(self.registry), self.module))?;
                        }
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
                    let ty = self.instr.destination_type().unwrap();
                    write!(f, "{}: {} = ", dest, self.registry.fmt(ty))?;
                }
                write!(f, "{}", opname)?;

                // Perform specific formatting based on instruction type
                if self.specific_fmt(f)? {
                    return Ok(());
                }

                write!(f, " ")?;

                // Format operands
                let mut first = true;
                for operand in self.instr.operands() {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", operand.fmt_with(Some(self.registry), self.module))?;
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

impl HyTerminator {
    /// Build a formatting helper that renders the terminator using the supplied module for context.
    pub fn fmt<'a>(
        &'a self,
        registry: Option<&'a TypeRegistry>,
        module: Option<&'a Module>,
    ) -> impl std::fmt::Display + 'a {
        struct Fmt<'a> {
            terminator: &'a HyTerminator,
            registry: Option<&'a TypeRegistry>,
            module: Option<&'a Module>,
        }

        impl std::fmt::Display for Fmt<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.terminator {
                    HyTerminator::Branch(cbranch) => write!(
                        f,
                        "branch {}, {}, {}",
                        cbranch.cond.fmt_with(self.registry, self.module),
                        cbranch.target_true,
                        cbranch.target_false
                    ),
                    HyTerminator::Jump(jump) => {
                        write!(f, "jump label {}", jump.target)
                    }
                    HyTerminator::Ret(ret) => {
                        if let Some(value) = &ret.value {
                            write!(f, "ret {:#}", value.fmt_with(self.registry, self.module))
                        } else {
                            write!(f, "ret void")
                        }
                    }
                    HyTerminator::Trap(_) => {
                        write!(f, "trap")
                    }
                }
            }
        }

        Fmt {
            terminator: self,
            registry,
            module,
        }
    }
}

impl Function {
    /// Build a formatting helper that renders the function in textual form.
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
                    "define{} {} {}{}",
                    self.function
                        .cconv
                        .as_ref()
                        .map(|cc| format!(" {}", cc.to_string()))
                        .unwrap_or_default(),
                    self.function
                        .return_type
                        .map(|ty| self.type_registry.fmt(ty).to_string())
                        .unwrap_or("void".to_string()),
                    if self.function.meta_function { "!" } else { "" },
                    self.function
                        .name
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap_or(format!("@{}", self.function.uuid))
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
                        writeln!(f, "  {}", instr.fmt(self.type_registry, self.module))?;
                    }

                    writeln!(
                        f,
                        "  {}",
                        block.terminator.fmt(Some(self.type_registry), self.module)
                    )?;
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

impl Module {
    /// Build a formatting helper that renders every function within the module.
    pub fn fmt<'a>(&'a self, type_registry: &'a TypeRegistry) -> impl std::fmt::Display + 'a {
        struct Fmt<'a> {
            module: &'a Module,
            type_registry: &'a TypeRegistry,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for function in self.module.functions.values() {
                    writeln!(f, "{}", function.fmt(self.type_registry, Some(self.module)))?;
                }
                Ok(())
            }
        }

        Fmt {
            module: self,
            type_registry,
        }
    }
}
