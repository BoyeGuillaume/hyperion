# HyFormal

HyFormal is a Rust crate for representing and manipulating formal systems using a unified expression language.
It supports types, terms, and logical formulas in a single framework, along with efficient encoding and decoding mechanisms.

## Features

- A single expression language (in `expr::*`) covering type constructors (Bool, Omega, Powerset, Tuple),
  term constructors (Variable, Lambda, Call, If, Tuple), and logic (True, False, Not, And, Or, Implies, Iff, Equal, Forall, Exists).
- A compact append-only encoding (`encoding::tree::TreeBuf`) with borrowed decoding (`AnyExprRef`) for allocation-free traversal.
- A pretty-printer (`expr::pretty`) and a parser (`parser`) for a readable concrete syntax.

### Quickstart

```rust
use hyformal::prelude::*;

let x = InlineVariable::new_from_raw(0);
let prop = forall(x, Bool, implies(x, equals(x, x)));
let e = prop.encode();
assert!(matches!(e.as_ref().view().type_(), expr::variant::ExprType::Forall));

## Arena-backed building and transformation

For short-lived construction and rewriting of expressions without allocation churn, use the arena API in `arena`.
You can mix structural arena nodes with borrowed pre-encoded subtrees and deep-copy results when needed.

```rust
use hyformal::arena::{ExprArenaCtx, ArenaAnyExpr};
use hyformal::expr::{Expr, view::ExprView, variant::ExprType};
use hyformal::expr::defs::{True, False, And};

let ctx = ExprArenaCtx::new();

// Mix arena views with borrowed encoded subtrees
let pre = And { lhs: True, rhs: False }.encode();
let leaf = ctx.reference_external(pre.as_ref());
let wrapped = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(leaf)));

// Deep copy an arena expression inside the same context
let copy = ctx.deep_copy(wrapped);

let e1 = wrapped.encode();
let e2 = copy.encode();
assert_eq!(e1.as_ref().type_(), ExprType::Not);
assert!(e1 == e2);
```
```
