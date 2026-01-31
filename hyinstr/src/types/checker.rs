use std::collections::BTreeMap;

use strum::IntoEnumIterator;

use crate::{
    consts::AnyConst,
    modules::{
        instructions::{HyInstr, Instruction},
        operand::{Name, Operand},
        terminator::HyTerminator,
    },
    types::{
        AnyType, TypeRegistry, Typeref,
        aggregate::ArrayType,
        primary::{FType, IType, PrimaryBasicType, PrimaryType, PtrType, VcSize, VcType},
    },
};

pub fn type_check<'a>(
    type_registry: &'a TypeRegistry,
    initiator: impl Iterator<Item = (Name, Typeref)>,
    instruction_iterator: impl Iterator<Item = &'a HyInstr> + Clone,
    terminator_iterator: impl Iterator<Item = &'a HyTerminator>,
    return_type: Option<Typeref>,
) -> Result<(), crate::utils::Error> {
    // First, iterate over all instructions once to discover all types
    // and ensure SSA names are unique
    let mut name_type_map: BTreeMap<Name, Typeref> = initiator.collect();
    for instruction in instruction_iterator.clone() {
        if let Some(dest_name) = instruction.destination() {
            // Retrieve destination type
            let dest_type = instruction.destination_type().unwrap();

            if name_type_map.insert(dest_name, dest_type).is_some() {
                return Err(crate::utils::Error::DuplicateSSAName {
                    duplicate: dest_name,
                });
            }
        }
    }

    // Define utility function to get the type of an operand
    let get_operand_type = |operand: &Operand| -> Result<Typeref, crate::utils::Error> {
        match operand {
            Operand::Reg(name) => {
                if let Some(typeref) = name_type_map.get(name) {
                    Ok(*typeref)
                } else {
                    Err(crate::utils::Error::UndefinedSSAName { undefined: *name })
                }
            }
            Operand::Imm(any_const) => Ok(any_const.typeref(type_registry)),
            Operand::Undef(typeref) => Ok(*typeref),
        }
    };

    // Now, perform type checking using the discovered types
    for instruction in instruction_iterator {
        use crate::modules::instructions::HyInstrOp::*;

        match instruction.op() {
            IAdd | ISub | IMul | IDiv | IRem | ISht | INeg | IAnd | IOr | IXor | INot
            | IImplies | IEquiv => {
                // All of the operands must be of the same integer/vectorized integer type
                let dest_type = instruction.destination_type().unwrap();
                let ty = type_registry.get(dest_type).unwrap();
                match *ty {
                    AnyType::Primary(PrimaryType::Int(_))
                    | AnyType::Primary(PrimaryType::Vc(VcType {
                        ty: PrimaryBasicType::Int(_),
                        ..
                    })) => {}
                    _ => {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: "iN or <N x iN>".to_string(),
                            found: type_registry.fmt(dest_type).to_string(),
                        });
                    }
                }

                // Check operand types
                for operand in instruction.operands() {
                    let operand_type = get_operand_type(operand)?;
                    if operand_type != dest_type {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: type_registry.fmt(dest_type).to_string(),
                            found: type_registry.fmt(operand_type).to_string(),
                        });
                    }
                }
            }
            FAdd | FSub | FMul | FDiv | FNeg | FRem => {
                // All of the operands must be of the same integer/vectorized fp type
                let dest_type = instruction.destination_type().unwrap();
                let ty = type_registry.get(dest_type).unwrap();
                match *ty {
                    AnyType::Primary(PrimaryType::Float(_))
                    | AnyType::Primary(PrimaryType::Vc(VcType {
                        ty: PrimaryBasicType::Float(_),
                        ..
                    })) => {}
                    _ => {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: "fp or <N x fp>".to_string(),
                            found: type_registry.fmt(dest_type).to_string(),
                        });
                    }
                }

                // Check operand types
                for operand in instruction.operands() {
                    let operand_type = get_operand_type(operand)?;
                    if operand_type != dest_type {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: type_registry.fmt(dest_type).to_string(),
                            found: type_registry.fmt(operand_type).to_string(),
                        });
                    }
                }
            }
            ICmp | FCmp => {
                // Destination type must be i1 or <N x i1>
                let dest_type = instruction.destination_type().unwrap();
                let ty = type_registry.get(dest_type).unwrap();
                let mut vc_size: Option<VcSize> = None;
                match *ty {
                    AnyType::Primary(PrimaryType::Int(i_type)) if i_type.num_bits() == 1 => {}
                    AnyType::Primary(PrimaryType::Vc(VcType {
                        ty: PrimaryBasicType::Int(i_type),
                        size,
                    })) if i_type.num_bits() == 1 => {
                        vc_size = Some(size);
                    }
                    _ => {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: "i1 or <N x i1>".to_string(),
                            found: type_registry.fmt(dest_type).to_string(),
                        });
                    }
                }

                // Both operands must be of the same integer/vectorized integer type
                let mut operands_iterator = instruction.operands();
                let op_a = operands_iterator.next().unwrap();
                let op_b = operands_iterator.next().unwrap();

                let type_a = get_operand_type(op_a)?;
                let type_b = get_operand_type(op_b)?;

                if type_a != type_b {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: type_registry.fmt(type_a).to_string(),
                        found: type_registry.fmt(type_b).to_string(),
                    });
                }

                // Ensure operand type is integer/vectorized integer
                let ty = type_registry.get(type_a).unwrap();
                if instruction.op() == ICmp {
                    match (&*ty, vc_size.is_some()) {
                        (AnyType::Primary(PrimaryType::Int(_)), false) => {}
                        (
                            AnyType::Primary(PrimaryType::Vc(VcType {
                                ty: PrimaryBasicType::Int(_),
                                size,
                            })),
                            true,
                        ) if *size == vc_size.unwrap() => {}
                        _ => {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: if let Some(size) = vc_size {
                                    format!("<{} x iN>", size)
                                } else {
                                    "iN".to_string()
                                },
                                found: type_registry.fmt(type_a).to_string(),
                            });
                        }
                    }
                } else {
                    match (&*ty, vc_size.is_some()) {
                        (AnyType::Primary(PrimaryType::Float(_)), false) => {}
                        (
                            AnyType::Primary(PrimaryType::Vc(VcType {
                                ty: PrimaryBasicType::Float(_),
                                size,
                            })),
                            true,
                        ) if *size == vc_size.unwrap() => {}
                        _ => {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: if let Some(size) = vc_size {
                                    format!("<{} x fp>", size)
                                } else {
                                    "fp".to_string()
                                },
                                found: type_registry.fmt(type_a).to_string(),
                            });
                        }
                    }
                }
            }
            MLoad => {
                // Pointer type must be a pointer to the destination type
                let mut operands_iterator = instruction.operands();
                let ptr_operand = operands_iterator.next().unwrap();
                let ptr_type = get_operand_type(ptr_operand)?;
                let ty = type_registry.get(ptr_type).unwrap();
                if !ty.is_primary() || !ty.try_as_primary_ref().unwrap().is_ptr() {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "ptr".to_string(),
                        found: type_registry.fmt(ptr_type).to_string(),
                    });
                }
            }
            MStore => {
                // Pointer type must be a pointer to the value type
                // Value operand can be of any type (not checked here)
                let mut operands_iterator = instruction.operands();
                let ptr_operand = operands_iterator.next().unwrap();
                let ptr_type = get_operand_type(ptr_operand)?;
                let ty = type_registry.get(ptr_type).unwrap();
                if !ty.is_primary() || !ty.try_as_primary_ref().unwrap().is_ptr() {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "ptr".to_string(),
                        found: type_registry.fmt(ptr_type).to_string(),
                    });
                }
            }
            MAlloca => {
                // First operand must be number of elements (therefore integer)
                let mut operands_iterator = instruction.operands();
                let num_elements_operand = operands_iterator.next().unwrap();
                let num_elements_type = get_operand_type(num_elements_operand)?;
                let ty = type_registry.get(num_elements_type).unwrap();
                if !ty.is_primary() || !ty.try_as_primary_ref().unwrap().is_int() {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "iN".to_string(),
                        found: type_registry.fmt(num_elements_type).to_string(),
                    });
                }
            }
            MGetElementPtr => {
                let element_ptr = instruction.try_as_m_get_element_ptr_ref().unwrap();

                // Destination and base pointer types must be ptr
                let ptr_type = type_registry.search_or_insert(PtrType.into());
                if element_ptr.ty != ptr_type {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: type_registry.fmt(ptr_type).to_string(),
                        found: type_registry.fmt(element_ptr.ty).to_string(),
                    });
                }

                let base_ptr_type = get_operand_type(&element_ptr.base)?;
                if base_ptr_type != ptr_type {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: type_registry.fmt(ptr_type).to_string(),
                        found: type_registry.fmt(base_ptr_type).to_string(),
                    });
                }

                // Ensure all indices are of integer type
                for index in &element_ptr.indices {
                    let operand_ty = get_operand_type(index)?;
                    let operand_ty_info = type_registry.get(operand_ty).unwrap();

                    if !matches!(&*operand_ty_info, AnyType::Primary(PrimaryType::Int(_))) {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: "iN".to_string(),
                            found: format!("{}", index.fmt(None)),
                        });
                    }
                }

                // Typeref of the current element
                // We skip the first index as it is for the base pointer indexing (when pointing to an array)
                let mut elem = element_ptr.in_ty;
                for index in element_ptr.indices.iter().skip(1) {
                    let ty = type_registry.get(elem).unwrap();
                    match &*ty {
                        AnyType::Primary(PrimaryType::Vc(VcType { ty, .. })) => {
                            // Cannot check index bounds, just update elem
                            elem =
                                type_registry.search_or_insert(AnyType::Primary(ty.clone().into()));
                        }
                        AnyType::Array(ArrayType { ty, .. }) => {
                            // Cannot check index bounds, just update elem
                            elem = *ty;
                        }
                        AnyType::Struct(struct_type) => {
                            // Operand must be integer constant
                            if let Operand::Imm(AnyConst::Int(value)) = index {
                                // Convert the value to usize
                                let (sign, bigint_list) = value.value.to_u32_digits();
                                if sign != num_bigint::Sign::Plus || bigint_list.len() != 1 {
                                    return Err(crate::utils::Error::TypeMismatch {
                                        instr: instruction.fmt(type_registry, None).to_string(),
                                        expected: "constant integer".to_string(),
                                        found: format!("{}", index.fmt(None)),
                                    });
                                }
                                let index_value = bigint_list[0] as usize;

                                // Finally, check index bounds
                                if index_value >= struct_type.element_types.len() {
                                    return Err(crate::utils::Error::ElementIndexOutOfBounds {
                                        instr: instruction.fmt(type_registry, None).to_string(),
                                        ty: type_registry.fmt(elem).to_string(),
                                        index: index_value,
                                        max: struct_type.element_types.len(),
                                    });
                                }
                                elem = struct_type.element_types[index_value];
                            } else {
                                return Err(crate::utils::Error::TypeMismatch {
                                    instr: instruction.fmt(type_registry, None).to_string(),
                                    expected: "constant integer".to_string(),
                                    found: format!("{}", index.fmt(None)),
                                });
                            }
                        }
                        _ => {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: "aggregate type".to_string(),
                                found: type_registry.fmt(elem).to_string(),
                            });
                        }
                    }
                }
            }
            Invoke => {
                let invoke = instruction.try_as_invoke_ref().unwrap();

                // Check that the fn type is a ptr and that's it, no type checking of arguments here
                let fn_type = get_operand_type(&invoke.function)?;
                let ptr_type = type_registry.search_or_insert(PtrType.into());
                if fn_type != ptr_type {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: type_registry.fmt(ptr_type).to_string(),
                        found: type_registry.fmt(fn_type).to_string(),
                    });
                }
            }
            Phi => {
                let dest_type = instruction.destination_type().unwrap();

                // All incoming values must match the destination type
                for operand in instruction.operands() {
                    let operand_type = get_operand_type(operand)?;
                    if operand_type != dest_type {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: type_registry.fmt(dest_type).to_string(),
                            found: type_registry.fmt(operand_type).to_string(),
                        });
                    }
                }
            }
            Select => {
                let dest_type = instruction.destination_type().unwrap();

                let mut operands_iterator = instruction.operands();
                let condition_operand = operands_iterator.next().unwrap();
                let condition_type = get_operand_type(condition_operand)?;
                let ty = type_registry.get(condition_type).unwrap();
                if !matches!(
                    *ty,
                    AnyType::Primary(PrimaryType::Int(i_type)) if i_type.num_bits() == 1
                ) {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "i1".to_string(),
                        found: type_registry.fmt(condition_type).to_string(),
                    });
                }

                let true_operand = operands_iterator.next().unwrap();
                let false_operand = operands_iterator.next().unwrap();
                let true_type = get_operand_type(true_operand)?;
                let false_type = get_operand_type(false_operand)?;

                if true_type != dest_type {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: type_registry.fmt(dest_type).to_string(),
                        found: type_registry.fmt(true_type).to_string(),
                    });
                }

                if false_type != dest_type {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: type_registry.fmt(dest_type).to_string(),
                        found: type_registry.fmt(false_type).to_string(),
                    });
                }
            }
            Cast => {
                let cast = instruction.try_as_cast_ref().unwrap();

                // Check the output type and input type
                let input_typeref = get_operand_type(&cast.value)?;
                let output_type = type_registry.get(cast.ty).unwrap();
                let input_type = type_registry.get(get_operand_type(&cast.value)?).unwrap();

                // Check vectorization consistency
                let input_vc_size = input_type
                    .try_as_primary_ref()
                    .and_then(|pt| pt.try_as_vc_ref())
                    .map(|vc| vc.size);
                let output_vc_size = output_type
                    .try_as_primary_ref()
                    .and_then(|pt| pt.try_as_vc_ref())
                    .map(|vc| vc.size);

                if input_vc_size != output_vc_size {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: if let Some(size) = output_vc_size {
                            format!("<{} x T>", size)
                        } else {
                            "T".to_string()
                        },
                        found: type_registry.fmt(input_typeref).to_string(),
                    });
                }

                // Utility functions to assume integer/float types
                let assume_integer = |ty: &AnyType| -> Result<IType, crate::utils::Error> {
                    if let Some(primary_type) = ty.try_as_primary_ref() {
                        if let Some(i_type) = primary_type.try_as_int_ref() {
                            return Ok(*i_type);
                        }
                        if let Some(vc_type) = primary_type.try_as_vc_ref()
                            && let Some(i_type) = vc_type.ty.try_as_int_ref()
                        {
                            return Ok(*i_type);
                        }
                    }
                    Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "iN".to_string(),
                        found: type_registry.fmt(input_typeref).to_string(),
                    })
                };

                let assume_float = |ty: &AnyType| -> Result<FType, crate::utils::Error> {
                    if let Some(primary_type) = ty.try_as_primary_ref() {
                        if let Some(f_type) = primary_type.try_as_float_ref() {
                            return Ok(*f_type);
                        }
                        if let Some(vc_type) = primary_type.try_as_vc_ref()
                            && let Some(f_type) = vc_type.ty.try_as_float_ref()
                        {
                            return Ok(*f_type);
                        }
                    }
                    Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "fp".to_string(),
                        found: type_registry.fmt(input_typeref).to_string(),
                    })
                };

                let assume_ptr = |ty: &AnyType| -> Result<PtrType, crate::utils::Error> {
                    if let Some(primary_type) = ty.try_as_primary_ref()
                        && let Some(ptr_type) = primary_type.try_as_ptr_ref()
                    {
                        return Ok(*ptr_type);
                    }

                    Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "ptr".to_string(),
                        found: type_registry.fmt(input_typeref).to_string(),
                    })
                };

                use crate::modules::instructions::misc::CastVariant::*;
                match cast.variant {
                    Trunc => {
                        let input_i_type = assume_integer(&input_type)?;
                        let output_i_type = assume_integer(&output_type)?;

                        if input_i_type.num_bits() <= output_i_type.num_bits() {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: format!("iN where N < {}", input_i_type.num_bits()),
                                found: type_registry.fmt(input_typeref).to_string(),
                            });
                        }
                    }
                    ZExt | SExt => {
                        let input_i_type = assume_integer(&input_type)?;
                        let output_i_type = assume_integer(&output_type)?;

                        if input_i_type.num_bits() >= output_i_type.num_bits() {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: format!("iN where N > {}", input_i_type.num_bits()),
                                found: type_registry.fmt(input_typeref).to_string(),
                            });
                        }
                    }
                    FpTrunc => {
                        let input_f_type = assume_float(&input_type)?;
                        let output_f_type = assume_float(&output_type)?;

                        if input_f_type.byte_size() <= output_f_type.byte_size() {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: format!(
                                    "one of {}",
                                    FType::iter()
                                        .filter(|t| t.byte_size() < input_f_type.byte_size())
                                        .map(|t| t.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                                found: type_registry.fmt(input_typeref).to_string(),
                            });
                        }
                    }
                    FpExt => {
                        let input_f_type = assume_float(&input_type)?;
                        let output_f_type = assume_float(&output_type)?;

                        if input_f_type.byte_size() >= output_f_type.byte_size() {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: format!(
                                    "one of {}",
                                    FType::iter()
                                        .filter(|t| t.byte_size() > input_f_type.byte_size())
                                        .map(|t| t.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                                found: type_registry.fmt(input_typeref).to_string(),
                            });
                        }
                    }
                    FpToUI => {
                        assume_float(&input_type)?;
                        assume_integer(&output_type)?;
                    }
                    FpToSI => {
                        assume_float(&input_type)?;
                        assume_integer(&output_type)?;
                    }
                    UIToFp => {
                        assume_integer(&input_type)?;
                        assume_float(&output_type)?;
                    }
                    SIToFp => {
                        assume_integer(&input_type)?;
                        assume_float(&output_type)?;
                    }
                    PtrToInt => {
                        assume_ptr(&input_type)?;
                        assume_integer(&output_type)?;
                    }
                    IntToPtr => {
                        assume_integer(&input_type)?;
                        assume_ptr(&output_type)?;
                    }
                    Bitcast => {
                        // Input and output types must be of the same size
                        // Only work between primary types
                        let input_size = assume_float(&input_type)
                            .map(|x| x.byte_size() * 8)
                            .or_else(|_| assume_integer(&input_type).map(|x| x.num_bits()))
                            .map_err(|_| crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: "iN or fp or <K x iN> or <K x fp>".to_string(),
                                found: type_registry.fmt(input_typeref).to_string(),
                            })?;

                        let output_size = assume_float(&output_type)
                            .map(|x| x.byte_size() * 8)
                            .or_else(|_| assume_integer(&output_type).map(|x| x.num_bits()))
                            .map_err(|_| crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: "iN or fp or <K x iN> or <K x fp>".to_string(),
                                found: type_registry.fmt(input_typeref).to_string(),
                            })?;

                        if input_size != output_size {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: format!("type of size {}", input_size),
                                found: type_registry.fmt(input_typeref).to_string(),
                            });
                        }
                    }
                };
            }
            InsertValue | ExtractValue => {
                // Constraints is that a_typeref[indices...] == b_typeref
                let (a_typeref, b_typeref, indices) = if instruction.op() == InsertValue {
                    let insert_value = instruction.try_as_insert_value_ref().unwrap();
                    let aggregate_typeref = get_operand_type(&insert_value.aggregate)?;
                    let value_typeref = get_operand_type(&insert_value.value)?;

                    if aggregate_typeref != insert_value.ty {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: type_registry.fmt(insert_value.ty).to_string(),
                            found: type_registry.fmt(aggregate_typeref).to_string(),
                        });
                    }

                    (aggregate_typeref, value_typeref, &insert_value.indices)
                } else {
                    let extract_value = instruction.try_as_extract_value_ref().unwrap();
                    (
                        get_operand_type(&extract_value.aggregate)?,
                        extract_value.ty,
                        &extract_value.indices,
                    )
                };

                // Now, verify that the value matches the expected type
                let mut expected_typeref = a_typeref;
                for &index in indices.iter() {
                    let ty = type_registry.get(expected_typeref).unwrap();
                    match &*ty {
                        AnyType::Primary(PrimaryType::Vc(VcType {
                            ty,
                            size: VcSize::Fixed(fixed_vc_size),
                        })) => {
                            if index >= *fixed_vc_size as u32 {
                                return Err(crate::utils::Error::ElementIndexOutOfBounds {
                                    instr: instruction.fmt(type_registry, None).to_string(),
                                    ty: type_registry.fmt(expected_typeref).to_string(),
                                    index: index as usize,
                                    max: *fixed_vc_size as usize,
                                });
                            } else {
                                let entry_typeref = type_registry
                                    .search_or_insert(AnyType::Primary(ty.clone().into()));
                                expected_typeref = entry_typeref;
                            }
                        }
                        AnyType::Array(array_type) => {
                            if index >= array_type.num_elements as u32 {
                                return Err(crate::utils::Error::ElementIndexOutOfBounds {
                                    instr: instruction.fmt(type_registry, None).to_string(),
                                    ty: type_registry.fmt(expected_typeref).to_string(),
                                    index: index as usize,
                                    max: array_type.num_elements as usize,
                                });
                            } else {
                                expected_typeref = array_type.ty;
                            }
                        }
                        AnyType::Struct(struct_type) => {
                            if index as usize >= struct_type.element_types.len() {
                                return Err(crate::utils::Error::ElementIndexOutOfBounds {
                                    instr: instruction.fmt(type_registry, None).to_string(),
                                    ty: type_registry.fmt(expected_typeref).to_string(),
                                    index: index as usize,
                                    max: struct_type.element_types.len(),
                                });
                            } else {
                                expected_typeref = struct_type.element_types[index as usize];
                            }
                        }
                        _ => {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: instruction.fmt(type_registry, None).to_string(),
                                expected: "aggregate type".to_string(),
                                found: type_registry.fmt(expected_typeref).to_string(),
                            });
                        }
                    }
                }

                // Finally, compare expected type with value type
                if expected_typeref != b_typeref {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: type_registry.fmt(expected_typeref).to_string(),
                        found: type_registry.fmt(b_typeref).to_string(),
                    });
                }
            }
            MetaAssert | MetaAssume => {
                let operand = instruction.operands().next().unwrap();
                let operand_type = get_operand_type(operand)?;
                let ty = type_registry.get(operand_type).unwrap();
                if !matches!(
                    *ty,
                    AnyType::Primary(PrimaryType::Int(i_type)) if i_type.num_bits() == 1
                ) {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "i1".to_string(),
                        found: type_registry.fmt(operand_type).to_string(),
                    });
                }
            }
            MetaIsDef => {
                let dest_type = instruction.destination_type().unwrap();
                let ty = type_registry.get(dest_type).unwrap();
                if !matches!(
                    *ty,
                    AnyType::Primary(PrimaryType::Int(i_type)) if i_type.num_bits() == 1
                ) {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: instruction.fmt(type_registry, None).to_string(),
                        expected: "i1".to_string(),
                        found: type_registry.fmt(dest_type).to_string(),
                    });
                }

                // No constraint on the operand type
            }
            MetaProb => {
                // Destination type must be floating-point, operand type must be scalar fp/int
                let dest_type = instruction.destination_type().unwrap();
                let ty = type_registry.get(dest_type).unwrap();
                match *ty {
                    AnyType::Primary(PrimaryType::Float(_)) => {}
                    _ => {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: "fp".to_string(),
                            found: type_registry.fmt(dest_type).to_string(),
                        });
                    }
                }

                let operand = instruction.operands().next().unwrap();
                let operand_type = get_operand_type(operand)?;
                let ty = type_registry.get(operand_type).unwrap();
                match *ty {
                    AnyType::Primary(PrimaryType::Float(_))
                    | AnyType::Primary(PrimaryType::Int(_)) => {}
                    _ => {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: "fp or iN".to_string(),
                            found: type_registry.fmt(operand_type).to_string(),
                        });
                    }
                }
            }
            MetaAnalysisStat => {
                // Destination type must be integer of >= 2 bits
                let dest_type = instruction.destination_type().unwrap();
                let ty = type_registry.get(dest_type).unwrap();
                match *ty {
                    AnyType::Primary(PrimaryType::Int(i_type)) if i_type.num_bits() >= 2 => {}
                    _ => {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: instruction.fmt(type_registry, None).to_string(),
                            expected: "iN (N >= 2)".to_string(),
                            found: type_registry.fmt(dest_type).to_string(),
                        });
                    }
                }
            }
            MetaForall => {
                // No operand, destination type not checked
            }
        }
    }

    // Finally, check terminators
    for terminator in terminator_iterator {
        use crate::modules::terminator::HyTerminator::*;

        match terminator {
            Branch(branch) => {
                // cond must be i1
                let typeref = get_operand_type(&branch.cond)?;
                let ty = type_registry.get(typeref).unwrap();
                if !matches!(
                    *ty,
                    AnyType::Primary(PrimaryType::Int(i_type)) if i_type.num_bits() == 1
                ) {
                    return Err(crate::utils::Error::TypeMismatch {
                        instr: terminator.fmt(Some(type_registry), None).to_string(),
                        expected: "i1".to_string(),
                        found: type_registry.fmt(typeref).to_string(),
                    });
                }
            }
            Ret(ret) => {
                if let Some(ret_type) = return_type {
                    if let Some(actual_ret_type) = &ret.value {
                        let typeref = get_operand_type(actual_ret_type)?;
                        if typeref != ret_type {
                            return Err(crate::utils::Error::TypeMismatch {
                                instr: terminator.fmt(Some(type_registry), None).to_string(),
                                expected: type_registry.fmt(ret_type).to_string(),
                                found: type_registry.fmt(typeref).to_string(),
                            });
                        }
                    } else {
                        return Err(crate::utils::Error::TypeMismatch {
                            instr: terminator.fmt(Some(type_registry), None).to_string(),
                            expected: type_registry.fmt(ret_type).to_string(),
                            found: "void".to_string(),
                        });
                    }
                } else if let Some(actual_ret_type) = &ret.value {
                    let typeref = get_operand_type(actual_ret_type)?;

                    return Err(crate::utils::Error::TypeMismatch {
                        instr: terminator.fmt(Some(type_registry), None).to_string(),
                        expected: "void".to_string(),
                        found: type_registry.fmt(typeref).to_string(),
                    });
                }
            }
            Jump(_) | Trap(_) => {}
        }
    }

    Ok(())
}
