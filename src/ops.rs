//! Operator sugar for propositions.
//!
//! The `define_ops_prop!` macro implements `BitAnd`, `BitOr`, and `Not` on the given
//! proposition-like type, so you can write `p & q`, `p | q`, and `!p`.
macro_rules! define_ops_prop {
    (
        $name:ident
        $( <
            $( $($lft:lifetime),+ $(,)? )?
            $( $($gen_name:ident: $gen:tt ),+ $(,)? )?
        > )?
    ) => {
        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Prop
        > std::ops::BitAnd<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = And<Self, _O1>;

            fn bitand(self, rhs: _O1) -> Self::Output {
                And {
                    left: self,
                    right: rhs,
                }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Prop
        > std::ops::BitOr<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Or<Self, _O1>;

            fn bitor(self, rhs: _O1) -> Self::Output {
                Or {
                    left: self,
                    right: rhs,
                }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
        > std::ops::Not for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Not<Self>;

            fn not(self) -> Self::Output {
                Not { inner: self }
            }
        }
    };
}
