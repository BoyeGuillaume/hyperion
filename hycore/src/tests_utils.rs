use std::vec;

use hyinstr::{
    consts::int::IConst,
    modules::{
        BasicBlock, Function, Visibility,
        int::{ICmp, ICmpOp, IMul, ISub, IntegerSignedness, OverflowPolicy},
        misc::Phi,
        operand::{Label, Name, Operand},
        terminator::{CBranch, Ret},
    },
    types::{TypeRegistry, primary::IType},
};
use uuid::Uuid;

/// Example function for testing purposes (function pow(x, n) = x^n)
pub fn example_a(registry: &mut TypeRegistry) -> Function {
    let arg_x: Name = 0;
    let arg_n: Name = 1;

    let entry_cmp: Name = 2;

    let loop_n: Name = 3;
    let loop_n_inc: Name = 4;
    let loop_result: Name = 5;
    let loop_result_inc: Name = 6;
    let loop_cmp: Name = 7;

    let ret_phi: Name = 8;

    let i64_ty = registry.search_or_insert(IType::I64.into());
    let i1_ty = registry.search_or_insert(IType::I1.into());
    let one_i64 = Operand::Imm(IConst::from(1u64).into());
    let zero_i64 = Operand::Imm(IConst::from(0u64).into());

    let entry_block = BasicBlock {
        label: Label::NIL,
        instructions: vec![
            ICmp {
                dest: entry_cmp,
                ty: i1_ty,
                lhs: Operand::Reg(arg_n),
                rhs: zero_i64.clone(),
                op: ICmpOp::Eq,
            }
            .into(),
        ],
        terminator: CBranch {
            cond: Operand::Reg(entry_cmp),
            target_true: Label(1),
            target_false: Label(2),
        }
        .into(),
    };

    let loop_block = BasicBlock {
        label: Label(1),
        instructions: vec![
            Phi {
                dest: loop_n,
                ty: i64_ty,
                values: vec![
                    (Operand::Reg(arg_n), Label::NIL),
                    (Operand::Reg(loop_n_inc), Label(1)),
                ],
            }
            .into(),
            Phi {
                dest: loop_result,
                ty: i64_ty,
                values: vec![
                    (one_i64.clone(), Label::NIL),
                    (Operand::Reg(loop_result_inc), Label(1)),
                ],
            }
            .into(),
            ISub {
                dest: loop_n_inc,
                ty: i64_ty,
                lhs: Operand::Reg(loop_n),
                rhs: one_i64.clone(),
                overflow: OverflowPolicy::Panic,
                signedness: IntegerSignedness::Unsigned,
            }
            .into(),
            IMul {
                dest: loop_result_inc,
                ty: i64_ty,
                lhs: Operand::Reg(loop_result),
                rhs: Operand::Reg(arg_x),
                overflow: OverflowPolicy::Saturate,
                signedness: IntegerSignedness::Unsigned,
            }
            .into(),
            ICmp {
                dest: loop_cmp,
                ty: i1_ty,
                lhs: Operand::Reg(loop_n_inc),
                rhs: zero_i64.clone(),
                op: ICmpOp::Eq,
            }
            .into(),
        ],
        terminator: CBranch {
            cond: Operand::Reg(loop_cmp),
            target_true: Label(2),
            target_false: Label(1),
        }
        .into(),
    };

    let block_ret = BasicBlock {
        label: Label(2),
        instructions: vec![
            Phi {
                dest: ret_phi,
                ty: i64_ty,
                values: vec![
                    (one_i64.clone(), Label::NIL),
                    (Operand::Reg(loop_result), Label(1)),
                ],
            }
            .into(),
        ],
        terminator: Ret {
            value: Some(Operand::Reg(ret_phi)),
        }
        .into(),
    };

    let func = Function {
        uuid: Uuid::new_v4(),
        name: Some("pow".to_string()),
        params: vec![(arg_x, i64_ty), (arg_n, i64_ty)],
        return_type: Some(i64_ty),
        body: vec![entry_block, loop_block, block_ret]
            .into_iter()
            .map(|bb| (bb.label, bb))
            .collect(),
        visibility: Some(Visibility::Default),
        cconv: Some(hyinstr::modules::CallingConvention::C),
        wildcard_types: Default::default(),
        meta_function: false,
    };

    func
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_a_sounds() {
        let mut registry = TypeRegistry::new([0; 6]);
        let func = example_a(&mut registry);
        assert!(func.verify().is_ok());
        assert_eq!(func.name.unwrap(), "pow");
        assert_eq!(func.params.len(), 2);
        assert_eq!(
            func.return_type.unwrap(),
            registry.search_or_insert(IType::I64.into())
        );
        assert_eq!(func.body.len(), 3);
    }
}
