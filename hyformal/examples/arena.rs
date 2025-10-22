use hyformal::arena::{ArenaAllocableExpr, ArenaAnyExpr, ExprArenaCtx};
use hyformal::expr::variant::ExprType;
use hyformal::expr::view::ExprView;
use hyformal::prelude::*;

fn main() {
    // Create a temporary arena context
    let ctx = ExprArenaCtx::new();

    // Simple arena-local nodes
    let x = InlineVariable::new_from_raw(0);
    let var_x = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Variable(x)));
    let t = True.alloc_in(&ctx);

    // Build an And(Variable(x), True) node inside the arena
    let and_node = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::And(var_x, t)));

    // Encode the arena-backed node and inspect the resulting type
    let encoded_and = and_node.encode();
    println!("Encoded root type: {:?}", encoded_and.as_ref().type_());
    assert_eq!(encoded_and.as_ref().type_(), ExprType::And);

    // Create a pre-encoded subtree and reference it from the arena as a leaf
    let pre = Or {
        lhs: True,
        rhs: False,
    }
    .encode();
    let pre_ref = pre.as_ref();
    let leaf_ref = ctx.reference_external(pre_ref);

    // Wrap the referenced subtree inside a Not node in the arena
    let wrapped = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(leaf_ref)));
    let wrapped_encoded = wrapped.encode();
    println!("Wrapped root type: {:?}", wrapped_encoded.as_ref().type_());

    // Demonstrate deep_copy: copy the arena-built `and_node` into new nodes in the same arena
    let and_copy = ctx.deep_copy(and_node);
    let c_enc = and_copy.encode();
    println!("and_copy type: {:?}", c_enc.as_ref().type_());
    assert!(encoded_and == c_enc);

    // Demonstrate deep_copy_ref: import an owned/buffered expression into the arena
    let owned = Not {
        inner: And {
            lhs: True,
            rhs: False,
        },
    }
    .encode();
    let imported = ctx.deep_copy_ref(owned.as_ref());
    println!("imported type: {:?}", imported.encode().as_ref().type_());
    assert_eq!(imported.encode().as_ref().type_(), ExprType::Not);

    // Mix everything: build If(condition = wrapped, then = and_node, else = imported)
    let if_node = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::If {
        condition: wrapped,
        then_branch: and_node,
        else_branch: imported,
    }));

    let if_encoded = if_node.encode();
    println!(
        "Final composed root type: {:?}",
        if_encoded.as_ref().type_()
    );
    // Should be an If node
    assert_eq!(if_encoded.as_ref().type_(), ExprType::If);

    println!("Arena example finished successfully.");
}
