use std::cell::RefCell;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use hyformal::{arena::ArenaAllocableExpr, prelude::*};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

fn build_simple_expr() -> impl Expr + for<'a> ArenaAllocableExpr<'a> {
    // Simple nested expression: forall x: Powerset(Omega). forall y: Omega. exists z: x. z = y
    let x = InlineVariable::new(Variable::Internal(0));
    let y = InlineVariable::new(Variable::Internal(1));
    let z = InlineVariable::new(Variable::Internal(2));

    let omega = forall(
        x,
        powerset(Omega),
        forall(y, Omega, exists(z, x, z.equals(y))),
    );
    omega
}

fn build_complex_expr() -> hyformal::expr::AnyExpr {
    // Build a medium-sized expression by nesting simple expressions. Use randomness seeded for
    // determinism.
    let rng = ChaCha20Rng::seed_from_u64(0x42);
    let arena = ExprArenaCtx::new();

    fn next_create<'a>(
        budget: usize,
        rng: &mut impl Rng,
        arena: &'a ExprArenaCtx<'a>,
    ) -> &'a RefCell<ArenaAnyExpr<'a>> {
        if budget == 0 || rng.random_bool(0.2) {
            // Select a leaf node randomly sampled uniformly
            return match rng.random_range(0..=5) {
                0 => True.alloc_in(arena),
                1 => False.alloc_in(arena),
                2 => Bool.alloc_in(arena),
                3 => Omega.alloc_in(arena),
                4 => Never.alloc_in(arena),
                5 => InlineVariable::new_from_raw(rng.next_u32()).alloc_in(arena),
                _ => unreachable!(),
            };
        }

        // Otherwise build a nested expression
        return match rng.random_range(0..=7) {
            0 => {
                let left = next_create(budget - 1, rng, arena);
                let right = next_create(budget - 1, rng, arena);
                and(left, right).alloc_in(arena)
            }
            1 => {
                let left = next_create(budget - 1, rng, arena);
                let right = next_create(budget - 1, rng, arena);
                or(left, right).alloc_in(arena)
            }
            2 => {
                let var = InlineVariable::new(Variable::Internal(rng.next_u32() & 0x7fffffff));
                let dtype = next_create(budget - 1, rng, arena);
                let inner = next_create(budget - 1, rng, arena);
                forall(var, dtype, inner).alloc_in(arena)
            }
            3 => {
                let var = InlineVariable::new(Variable::Internal(rng.next_u32() & 0x7fffffff));
                let dtype = next_create(budget - 1, rng, arena);
                let inner = next_create(budget - 1, rng, arena);
                exists(var, dtype, inner).alloc_in(arena)
            }
            4 => {
                let base = next_create(budget - 1, rng, arena);
                powerset(base).alloc_in(arena)
            }
            5 => {
                let func = next_create(budget - 1, rng, arena);
                let arg = next_create(budget - 1, rng, arena);
                func.apply(arg).alloc_in(arena)
            }
            6 => {
                let left = next_create(budget - 1, rng, arena);
                let right = next_create(budget - 1, rng, arena);
                left.equals(right).alloc_in(arena)
            }
            7 => {
                let inner = next_create(budget - 1, rng, arena);
                not(inner).alloc_in(arena)
            }
            _ => unreachable!(),
        };
    }

    let expr = next_create(8, &mut rng.clone(), &arena);
    expr.encode()
}

fn bench_arena_encode(c: &mut Criterion) {
    let expr = build_simple_expr();

    // Prepare the arena with the provided expression
    let arena = ExprArenaCtx::new();
    let simple_expr = expr.alloc_in(&arena);

    // Benchmark encoding
    c.bench_function("arena_encode_simple", |b| {
        b.iter(|| {
            black_box(simple_expr.encode());
        })
    });

    // Prepare a large complex expression
    let complex_expr = build_complex_expr();
    let large_expr = arena.deep_copy_ref(complex_expr.as_ref());

    // Benchmark encoding of large expression
    c.bench_function("arena_encode_complex", |b| {
        b.iter(|| {
            black_box(large_expr.encode());
        })
    });
}

fn bench_deep_copy(c: &mut Criterion) {
    // Prepare a borrowed encoded expr and then deep-copy it into an arena
    let complex_expr = build_complex_expr();
    let simple_expr = build_simple_expr().encode();
    let arena = ExprArenaCtx::new();

    // Benchmark deep-copying
    c.bench_function("deep_copy_ref_simple", |b| {
        b.iter(|| {
            black_box(arena.deep_copy_ref(simple_expr.as_ref()));
        })
    });

    c.bench_function("deep_copy_ref_complex", |b| {
        b.iter(|| {
            black_box(arena.deep_copy_ref(complex_expr.as_ref()));
        })
    });
}

fn bench_walk_arena(c: &mut Criterion) {
    // Prepare a large complex expression in an arena
    let arena = ExprArenaCtx::new();
    let complex_expr = build_complex_expr().deep_copy_in(&arena);
    let simple_expr = build_simple_expr().alloc_in(&arena);

    // Benchmark walking
    c.bench_function("walk_arena_count_simple", |b| {
        b.iter(|| {
            let mut count = 0usize;

            walk(simple_expr, (), |_, node| {
                node.for_each_unary(|x, _| x.schedule_immediate(()));
                count += 1;
            });

            black_box(count);
        });
    });

    // Benchmark walking
    c.bench_function("walk_arena_count_complex", |b| {
        b.iter(|| {
            let mut count = 0usize;

            walk(complex_expr, (), |_, node| {
                node.for_each_unary(|x, _| x.schedule_immediate(()));
                count += 1;
            });

            black_box(count);
        });
    });
}

criterion_group!(
    benches,
    bench_arena_encode,
    bench_deep_copy,
    bench_walk_arena,
);
criterion_main!(benches);
