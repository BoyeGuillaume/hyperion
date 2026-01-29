use hyinstr::types::{
    TypeRegistry, Typeref,
    aggregate::{ArrayType, StructType},
    primary::IType,
};

#[test]
fn merge_basic_dedup() {
    let reg_a = TypeRegistry::new([1, 2, 3, 4, 5, 6]);
    let reg_b = TypeRegistry::new([0u8; 6]);

    // A already has i32
    let a_i32 = reg_a.search_or_insert(IType::I32.into());

    // B has i32 (duplicate) and i64 (new for A)
    let b_i32 = reg_b.search_or_insert(IType::I32.into());
    let b_i64 = reg_b.search_or_insert(IType::I64.into());

    let mapping = reg_a.merge_with(&reg_b);

    // i32 should map to the existing A typeref
    assert_eq!(mapping.get(&b_i32), Some(&a_i32));

    // i64 should be inserted into A and map accordingly
    let mapped_i64 = mapping.get(&b_i64).copied().expect("missing i64 mapping");
    assert_eq!(reg_a.get(mapped_i64).as_deref(), Some(&IType::I64.into()));

    // Ensure dedup: inserting i64 again yields same typeref
    let a_i64_again = reg_a.search_or_insert(IType::I64.into());
    assert_eq!(a_i64_again, mapped_i64);
}

#[test]
fn merge_aggregates_dependency_order() {
    let reg_a = TypeRegistry::new([0u8; 6]);
    let reg_b = TypeRegistry::new([0u8; 6]);

    // Base types in B
    let b_i8 = reg_b.search_or_insert(IType::I8.into());
    let b_i32 = reg_b.search_or_insert(IType::I32.into());

    // Dependent types in B
    let b_vec = reg_b.search_or_insert(
        hyinstr::types::primary::VcType {
            ty: IType::I8.into(),
            size: 4.into(),
        }
        .into(),
    );
    let b_struct = reg_b.search_or_insert(
        StructType {
            element_types: vec![b_i32, b_vec],
            packed: false,
        }
        .into(),
    );
    let b_array = reg_b.search_or_insert(
        ArrayType {
            ty: b_i32,
            num_elements: 3,
        }
        .into(),
    );

    let mapping = reg_a.merge_with(&reg_b);

    // All B types should have a mapping
    for b in [b_i8, b_i32, b_vec, b_struct, b_array] {
        assert!(mapping.contains_key(&b));
    }

    // Check that dependent types were remapped to A's corresponding typerefs
    let a_i32 = mapping[&b_i32];
    let a_vec = mapping[&b_vec];
    let a_struct = mapping[&b_struct];
    let a_array = mapping[&b_array];

    assert_eq!(reg_a.get(a_i32).as_deref(), Some(&IType::I32.into()));
    assert_eq!(
        reg_a.get(a_vec).as_deref(),
        Some(
            &hyinstr::types::primary::VcType {
                ty: IType::I8.into(),
                size: 4.into(),
            }
            .into()
        )
    );
    assert_eq!(
        reg_a.get(a_struct).as_deref(),
        Some(
            &StructType {
                element_types: vec![a_i32, a_vec],
                packed: false,
            }
            .into()
        )
    );
    assert_eq!(
        reg_a.get(a_array).as_deref(),
        Some(
            &ArrayType {
                ty: a_i32,
                num_elements: 3
            }
            .into()
        )
    );

    // Re-merge should be idempotent (map to the same A typerefs)
    let mapping2 = reg_a.merge_with(&reg_b);
    for b in [b_i8, b_i32, b_vec, b_struct, b_array] {
        assert_eq!(mapping2[&b], mapping[&b]);
    }
}

#[test]
fn merge_with_wildcard_refs() {
    let reg_a = TypeRegistry::new([0u8; 6]);
    let reg_b = TypeRegistry::new([0u8; 6]);

    // Array with wildcard element typeref in B
    let wc = Typeref::new_wildcard(7);
    let b_array_wc = reg_b.search_or_insert(
        ArrayType {
            ty: wc,
            num_elements: 2,
        }
        .into(),
    );

    let mapping = reg_a.merge_with(&reg_b);
    let a_array_wc = mapping[&b_array_wc];

    // Wildcard should be preserved inside the aggregate
    assert_eq!(
        reg_a.get(a_array_wc).as_deref(),
        Some(
            &ArrayType {
                ty: wc,
                num_elements: 2
            }
            .into()
        )
    );
}
