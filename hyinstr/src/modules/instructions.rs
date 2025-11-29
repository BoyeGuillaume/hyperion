use auto_enums::auto_enum;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumTryAs};

use crate::{
    modules::{
        Instruction, Module, fp,
        int::{self, IntegerSignedness, OverflowPolicy},
        mem, meta, misc,
        operand::Operand,
    },
    types::TypeRegistry,
};

/// Discriminated union covering all public instruction kinds.
///
/// Use this enum to store heterogeneous instruction streams and to pattern‑match
/// on specific operations. The generated `HyInstrKind` discriminant (via
/// `strum`) can be helpful for fast classification.
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumIs, EnumTryAs, EnumDiscriminants)]
#[strum_discriminants(name(HyInstrOp))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HyInstr {
    // Integer instructions
    IAdd(int::IAdd),
    ISub(int::ISub),
    IMul(int::IMul),
    IDiv(int::IDiv),
    IRem(int::IRem),
    ICmp(int::ICmp),
    ISht(int::ISht),
    INeg(int::INeg),

    // Bitwise instructions
    IAnd(int::IAnd),
    IOr(int::IOr),
    IXor(int::IXor),
    INot(int::INot),
    IImplies(int::IImplies),
    IEquiv(int::IEquiv),

    // Floating-point instructions
    FAdd(fp::FAdd),
    FSub(fp::FSub),
    FMul(fp::FMul),
    FDiv(fp::FDiv),
    FRem(fp::FRem),
    FCmp(fp::FCmp),
    FNeg(fp::FNeg),

    // Memory instructions
    MLoad(mem::MLoad),
    MStore(mem::MStore),
    MAlloca(mem::MAlloca),
    MGetElementPtr(mem::MGetElementPtr),

    // Misc instructions
    Invoke(misc::Invoke),
    Phi(misc::Phi),
    Select(misc::Select),

    // Meta instructions
    MetaAssert(meta::MetaAssert),
    MetaAssume(meta::MetaAssume),
    MetaProb(meta::MetaProb),
}

impl HyInstrOp {
    /// Returns true if this instruction is "simple", weaker that has side-effects.
    ///
    /// A simple instruction is one that does not have side-effects except for potential
    /// trap due to overflow or invalid operations (e.g., division by zero).
    ///
    /// The intuition behind is that it could be freely duplicated (e.g., inlining) and
    /// that removing duplicated simple instructions would not change the program semantics.
    /// It cannot be fully removed as trapping behavior must be preserved.
    ///
    /// 1. Memory instructions are *never* "simple" even if technically non-volatile loads
    ///    could be considered as such.
    /// 2. meta assert/assume/prob are considered simple as they can be duplicated without
    ///    changing semantics.
    /// 3. Invoke instructions are not simple as they may have side-effects.
    /// 4. Phi instructions are considered simple as they are just SSA value selectors.
    /// 5. All arithmetic and logical instructions are considered simple.
    /// 6. Select instructions are considered simple as they are just SSA value selectors.
    pub fn is_simple(&self) -> bool {
        match self {
            HyInstrOp::MLoad | HyInstrOp::MStore | HyInstrOp::MAlloca | HyInstrOp::Invoke => false,
            _ => true,
        }
    }
}

impl HyInstr {
    fn fmt_arith_iop(
        opname: &'static str,
        f: &mut std::fmt::Formatter<'_>,
        signesness: IntegerSignedness,
        overflow_policy: OverflowPolicy,
    ) -> std::fmt::Result {
        use IntegerSignedness::*;
        use OverflowPolicy::*;

        match (overflow_policy, signesness) {
            (Panic, Signed) => {
                write!(f, "{} nsw ", opname)
            }
            (Panic, Unsigned) => {
                write!(f, "{} nuw ", opname)
            }
            (Wrap, _) => write!(f, "{} ", opname),
            (Saturate, _) => {
                write!(f, "{} nsat ", opname)
            }
        }
    }

    pub fn op(&self) -> HyInstrOp {
        self.into()
    }

    /// Returns true if this instruction is "simple", weaker that has side-effects.
    ///
    /// A simple instruction is one that does not have side-effects except for potential
    /// trap due to overflow or invalid operations (e.g., division by zero).
    ///
    /// The intuition behind is that it could be freely duplicated (e.g., inlining) and
    /// that removing duplicated simple instructions would not change the program semantics.
    /// It cannot be fully removed as trapping behavior must be preserved.
    ///
    /// 1. Memory instructions are *never* "simple" even if technically non-volatile loads
    ///    could be considered as such.
    /// 2. meta assert/assume/prob are considered simple as they can be duplicated without
    ///    changing semantics.
    /// 3. Invoke instructions are not simple as they may have side-effects.
    /// 4. Phi instructions are considered simple as they are just SSA value selectors.
    /// 5. All arithmetic and logical instructions are considered simple.
    /// 6. Select instructions are considered simple as they are just SSA value selectors.
    pub fn is_simple(&self) -> bool {
        self.op().is_simple()
    }

    pub fn fmt<'a>(
        &'a self,
        registry: &'a TypeRegistry,
        module: Option<&'a Module>,
    ) -> impl std::fmt::Display + 'a {
        pub struct Fmt<'a> {
            instr: &'a HyInstr,
            registry: &'a TypeRegistry,
            module: Option<&'a Module>,
        }

        impl<'a> std::fmt::Display for Fmt<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.instr {
                    HyInstr::IAdd(iadd) => {
                        write!(f, "%{} = ", iadd.dest)?;

                        // overflow handling
                        HyInstr::fmt_arith_iop("add", f, iadd.signedness, iadd.overflow)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(iadd.ty),
                            iadd.lhs.fmt(self.module),
                            iadd.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::ISub(isub) => {
                        write!(f, "%{} = ", isub.dest)?;

                        // overflow handling
                        HyInstr::fmt_arith_iop("sub", f, isub.signedness, isub.overflow)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(isub.ty),
                            isub.lhs.fmt(self.module),
                            isub.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::IMul(imul) => {
                        write!(f, "%{} = ", imul.dest)?;

                        // overflow handling
                        HyInstr::fmt_arith_iop("mul", f, imul.signedness, imul.overflow)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(imul.ty),
                            imul.lhs.fmt(self.module),
                            imul.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::IDiv(idiv) => {
                        write!(f, "%{} = ", idiv.dest)?;

                        // signedness
                        match idiv.signedness {
                            IntegerSignedness::Signed => write!(f, "sdiv")?,
                            IntegerSignedness::Unsigned => write!(f, "udiv")?,
                        }

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(idiv.ty),
                            idiv.lhs.fmt(self.module),
                            idiv.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::IRem(irem) => {
                        write!(f, "%{} = ", irem.dest)?;

                        // signedness
                        match irem.signedness {
                            IntegerSignedness::Signed => write!(f, "srem")?,
                            IntegerSignedness::Unsigned => write!(f, "urem")?,
                        }

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(irem.ty),
                            irem.lhs.fmt(self.module),
                            irem.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::ICmp(icmp) => {
                        write!(f, "%{} = ", icmp.dest)?;

                        // comparison operation
                        let op_str = match icmp.op {
                            int::ICmpOp::Eq => "eq",
                            int::ICmpOp::Ne => "ne",
                            int::ICmpOp::Ugt => "ugt",
                            int::ICmpOp::Uge => "uge",
                            int::ICmpOp::Ult => "ult",
                            int::ICmpOp::Ule => "ule",
                            int::ICmpOp::Sgt => "sgt",
                            int::ICmpOp::Sge => "sge",
                            int::ICmpOp::Slt => "slt",
                            int::ICmpOp::Sle => "sle",
                        };
                        write!(f, "icmp {} ", op_str)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(icmp.ty),
                            icmp.lhs.fmt(self.module),
                            icmp.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::ISht(isht) => {
                        write!(f, "%{} = ", isht.dest)?;

                        // shift operation
                        let op_str = match isht.op {
                            int::IShiftOp::Lsl => "shl",
                            int::IShiftOp::Lsr => "lshr",
                            int::IShiftOp::Asr => "ashr",
                            int::IShiftOp::Rol => "rol",
                            int::IShiftOp::Ror => "ror",
                        };
                        write!(f, "{} ", op_str)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(isht.ty),
                            isht.value.fmt(self.module),
                            isht.shift.fmt(self.module)
                        )
                    }
                    HyInstr::INeg(ineg) => {
                        write!(
                            f,
                            "%{} = neg {} {}",
                            ineg.dest,
                            self.registry.fmt(ineg.ty),
                            ineg.value.fmt(self.module)
                        )
                    }
                    HyInstr::IAnd(iand) => {
                        write!(f, "%{} = and ", iand.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(iand.ty),
                            iand.lhs.fmt(self.module),
                            iand.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::IOr(ior) => {
                        write!(f, "%{} = or ", ior.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(ior.ty),
                            ior.lhs.fmt(self.module),
                            ior.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::IXor(ixor) => {
                        write!(f, "%{} = xor ", ixor.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(ixor.ty),
                            ixor.lhs.fmt(self.module),
                            ixor.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::INot(inot) => {
                        write!(
                            f,
                            "%{} = not {} {}",
                            inot.dest,
                            self.registry.fmt(inot.ty),
                            inot.value.fmt(self.module)
                        )
                    }
                    HyInstr::IImplies(iimplies) => {
                        write!(
                            f,
                            "%{} = implies {} {}, {}",
                            iimplies.dest,
                            self.registry.fmt(iimplies.ty),
                            iimplies.premise.fmt(self.module),
                            iimplies.conclusion.fmt(self.module)
                        )
                    }
                    HyInstr::IEquiv(iequiv) => {
                        write!(
                            f,
                            "%{} = equiv {} {}, {}",
                            iequiv.dest,
                            self.registry.fmt(iequiv.ty),
                            iequiv.lhs.fmt(self.module),
                            iequiv.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::FAdd(fadd) => {
                        write!(f, "%{} = fadd ", fadd.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(fadd.ty),
                            fadd.lhs.fmt(self.module),
                            fadd.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::FSub(fsub) => {
                        write!(f, "%{} = fsub ", fsub.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(fsub.ty),
                            fsub.lhs.fmt(self.module),
                            fsub.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::FMul(fmul) => {
                        write!(f, "%{} = fmul ", fmul.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(fmul.ty),
                            fmul.lhs.fmt(self.module),
                            fmul.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::FDiv(fdiv) => {
                        write!(f, "%{} = fdiv ", fdiv.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(fdiv.ty),
                            fdiv.lhs.fmt(self.module),
                            fdiv.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::FRem(frem) => {
                        write!(f, "%{} = frem ", frem.dest)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(frem.ty),
                            frem.lhs.fmt(self.module),
                            frem.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::FCmp(fcmp) => {
                        write!(f, "%{} = fcmp ", fcmp.dest)?;

                        // comparison operation
                        let op_str = match fcmp.op {
                            fp::FCmpOp::Oeq => "oeq",
                            fp::FCmpOp::Ogt => "ogt",
                            fp::FCmpOp::Oge => "oge",
                            fp::FCmpOp::Olt => "olt",
                            fp::FCmpOp::Ole => "ole",
                            fp::FCmpOp::One => "one",
                            fp::FCmpOp::Ueq => "ueq",
                            fp::FCmpOp::Ugt => "ugt",
                            fp::FCmpOp::Uge => "uge",
                            fp::FCmpOp::Ult => "ult",
                            fp::FCmpOp::Ule => "ule",
                            fp::FCmpOp::Une => "une",
                            fp::FCmpOp::Ord => "ord",
                        };
                        write!(f, "{} ", op_str)?;

                        write!(
                            f,
                            "{} {}, {}",
                            self.registry.fmt(fcmp.ty),
                            fcmp.lhs.fmt(self.module),
                            fcmp.rhs.fmt(self.module)
                        )
                    }
                    HyInstr::FNeg(fneg) => {
                        write!(
                            f,
                            "%{} = fneg {} {}",
                            fneg.dest,
                            self.registry.fmt(fneg.ty),
                            fneg.value.fmt(self.module)
                        )
                    }

                    HyInstr::MLoad(mload) => {
                        write!(f, "%{} = load ", mload.dest)?;
                        if mload.ordering.is_some() {
                            write!(f, "atomic ")?;
                        }
                        if mload.volatile {
                            write!(f, "volatile ")?;
                        }

                        write!(
                            f,
                            "{}, ptr {}",
                            self.registry.fmt(mload.ty),
                            mload.addr.fmt(self.module)
                        )?;

                        if let Some(ordering) = &mload.ordering {
                            write!(f, " {}", ordering.to_string())?;
                        }

                        if let Some(alignment) = mload.alignment {
                            write!(f, ", align {}", alignment)?;
                        }

                        Ok(())
                    }
                    HyInstr::MStore(mstore) => {
                        write!(f, "store ")?;
                        if mstore.ordering.is_some() {
                            write!(f, "atomic ")?;
                        }
                        if mstore.volatile {
                            write!(f, "volatile ")?;
                        }

                        write!(
                            f,
                            "{}, ptr {}",
                            mstore.value.fmt(self.module),
                            mstore.addr.fmt(self.module)
                        )?;

                        if let Some(ordering) = &mstore.ordering {
                            write!(f, " {}", ordering.to_string())?;
                        }

                        if let Some(alignment) = mstore.alignment {
                            write!(f, ", align {}", alignment)?;
                        }

                        Ok(())
                    }
                    HyInstr::MAlloca(malloca) => {
                        write!(f, "%{} = alloca ", malloca.dest)?;

                        write!(
                            f,
                            "{}, {}",
                            self.registry.fmt(malloca.ty),
                            malloca.count.fmt(self.module)
                        )?;

                        if let Some(alignment) = malloca.alignment {
                            write!(f, ", align {}", alignment)?;
                        }

                        Ok(())
                    }
                    HyInstr::MGetElementPtr(mgep) => {
                        write!(f, "%{} = getelementptr ", mgep.dest)?;

                        write!(f, "{}", self.registry.fmt(mgep.ty))?;

                        for index in mgep.indices.iter() {
                            write!(f, ", {}", index.fmt(self.module))?;
                        }

                        Ok(())
                    }
                    HyInstr::Invoke(invoke) => {
                        let args_str = invoke
                            .args
                            .iter()
                            .map(|arg| arg.fmt(self.module).to_string())
                            .collect::<Vec<_>>()
                            .join(", ");

                        let name_str = invoke.function.fmt(self.module).to_string();

                        if let Some(dest) = invoke.dest {
                            if let Some(ret_ty) = &invoke.ty {
                                write!(
                                    f,
                                    "%{} = invoke {} {} ({})",
                                    dest,
                                    self.registry.fmt(*ret_ty),
                                    name_str,
                                    args_str,
                                )
                            } else {
                                write!(f, "invoke void({})", args_str)
                            }
                        } else {
                            write!(f, "invoke void({})", args_str)
                        }
                    }
                    HyInstr::Phi(phi) => {
                        write!(f, "%{} = phi {} ", phi.dest, self.registry.fmt(phi.ty))?;

                        let incoming_str = phi
                            .values
                            .iter()
                            .map(|(value, label)| {
                                format!("[ {}, {} ]", value.fmt(self.module), label)
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        write!(f, "{}", incoming_str)
                    }
                    HyInstr::Select(select) => {
                        write!(f, "%{} = select ", select.dest)?;
                        write!(
                            f,
                            "{} {}, {}, {}",
                            self.registry.fmt(select.ty),
                            select.condition.fmt(self.module),
                            select.true_value.fmt(self.module),
                            select.false_value.fmt(self.module)
                        )
                    }
                    HyInstr::MetaAssert(assert) => {
                        write!(f, "meta-assert {}", assert.condition.fmt(self.module))
                    }
                    HyInstr::MetaAssume(assume) => {
                        write!(f, "meta-assume {}", assume.condition.fmt(self.module))
                    }
                    HyInstr::MetaProb(prob) => {
                        write!(f, "%{} = meta-prob ", prob.dest)?;

                        write!(f, "{} ", self.registry.fmt(prob.ty))?;

                        match &prob.operand {
                            meta::ProbOperand::Probability(op) => {
                                write!(f, "P({})", op.fmt(self.module))
                            }
                            meta::ProbOperand::ExpectedValue(op) => {
                                write!(f, "E({})", op.fmt(self.module))
                            }
                            meta::ProbOperand::Variance(operand) => {
                                write!(f, "Var({})", operand.fmt(self.module))
                            }
                            meta::ProbOperand::ProbabilityReachability => {
                                write!(f, "P(#L >= 1)")
                            }
                            meta::ProbOperand::ExpectedIterations => {
                                write!(f, "E(#L)")
                            }
                        }
                    }
                }
            }
        }

        Fmt {
            instr: self,
            registry,
            module,
        }
    }
}

macro_rules! define_instr_any_instr {
    (
        $($variant:ident),* $(,)?
    ) => {
        impl Instruction for HyInstr {
            fn is_meta_instruction(&self) -> bool {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.is_meta_instruction(),
                    )*
                }
            }

            #[auto_enum(Iterator)]
            fn operands(&self) -> impl Iterator<Item = &Operand> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.operands(),
                    )*
                }
            }

            fn destination(&self) -> Option<super::operand::Name> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.destination(),
                    )*
                }
            }

            #[auto_enum(Iterator)]
            fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.operands_mut(),
                    )*
                }
            }

            fn set_destination(&mut self, name: super::operand::Name) {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.set_destination(name),
                    )*
                }
            }

            #[auto_enum(Iterator)]
            fn referenced_types(&self) -> impl Iterator<Item = crate::types::Typeref> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.referenced_types(),
                    )*
                }
            }

            fn destination_type(&self) -> Option<crate::types::Typeref> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.destination_type(),
                    )*
                }
            }
        }
    };
}

define_instr_any_instr! {
    IAdd,
    ISub,
    IMul,
    IDiv,
    IRem,
    ICmp,
    ISht,
    INeg,
    IAnd,
    IOr,
    IXor,
    INot,
    IImplies,
    IEquiv,
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
    FCmp,
    FNeg,
    MLoad,
    MStore,
    MAlloca,
    MGetElementPtr,
    Invoke,
    Phi,
    Select,
    MetaAssert,
    MetaAssume,
    MetaProb,
}

macro_rules! define_hyinstr_from {
    ($typ:ty, $variant:ident) => {
        impl From<$typ> for HyInstr {
            fn from(inst: $typ) -> Self {
                HyInstr::$variant(inst)
            }
        }
    };
}

define_hyinstr_from!(int::IAdd, IAdd);
define_hyinstr_from!(int::ISub, ISub);
define_hyinstr_from!(int::IMul, IMul);
define_hyinstr_from!(int::IDiv, IDiv);
define_hyinstr_from!(int::IRem, IRem);
define_hyinstr_from!(int::ICmp, ICmp);
define_hyinstr_from!(int::ISht, ISht);
define_hyinstr_from!(int::INeg, INeg);
define_hyinstr_from!(int::IAnd, IAnd);
define_hyinstr_from!(int::IOr, IOr);
define_hyinstr_from!(int::IXor, IXor);
define_hyinstr_from!(int::INot, INot);
define_hyinstr_from!(int::IImplies, IImplies);
define_hyinstr_from!(int::IEquiv, IEquiv);

define_hyinstr_from!(fp::FAdd, FAdd);
define_hyinstr_from!(fp::FSub, FSub);
define_hyinstr_from!(fp::FMul, FMul);
define_hyinstr_from!(fp::FDiv, FDiv);
define_hyinstr_from!(fp::FRem, FRem);
define_hyinstr_from!(fp::FCmp, FCmp);
define_hyinstr_from!(fp::FNeg, FNeg);

define_hyinstr_from!(mem::MLoad, MLoad);
define_hyinstr_from!(mem::MStore, MStore);
define_hyinstr_from!(mem::MAlloca, MAlloca);
define_hyinstr_from!(mem::MGetElementPtr, MGetElementPtr);

define_hyinstr_from!(misc::Invoke, Invoke);
define_hyinstr_from!(misc::Phi, Phi);
define_hyinstr_from!(misc::Select, Select);

define_hyinstr_from!(meta::MetaAssert, MetaAssert);
define_hyinstr_from!(meta::MetaAssume, MetaAssume);
define_hyinstr_from!(meta::MetaProb, MetaProb);
