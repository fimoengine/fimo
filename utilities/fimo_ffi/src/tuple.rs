//! C compatible tuple definitions.
use std::fmt::{Debug, Formatter};

/// Casts a C type to and from an equivalent Rust type.
pub trait ReprC {
    /// Rust type.
    type T: ReprRust;

    /// Casts the tuple to an equivalent Rust type.
    fn into_rust(self) -> Self::T;

    /// Casts the Rust tuple to the C type.
    fn from_rust(t: Self::T) -> Self;
}

/// Casts a Rust type to and from an equivalent C type.
pub trait ReprRust {
    /// Rust type.
    type T: ReprC;

    /// Casts the tuple to an equivalent C type.
    fn into_c(self) -> Self::T;

    /// Casts the C tuple to the Rust type.
    fn from_c(t: Self::T) -> Self;
}

macro_rules! tuple_impls {
    ($(
        $(#[$attr:meta])*
        $Tuple:ident {
            $(($idx:tt) -> $T:ident)+
        }
    )+) => {
        $(
            #[repr(C)]
            #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
            $(#[$attr])*
            pub struct $Tuple<$($T),+>($(pub $T),+);

            impl<$($T),+> ReprC for $Tuple<$($T),+> {
                type T = ($($T),+,);

                fn into_rust(self) -> Self::T {
                    ($(self.$idx),+,)
                }

                fn from_rust(t: Self::T) -> Self {
                    Self($(t.$idx),+)
                }
            }

            impl<$($T),+> ReprRust for ($($T),+,) {
                type T = $Tuple<$($T),+>;

                fn into_c(self) -> Self::T {
                    $Tuple::<$($T),+>($(self.$idx),+)
                }

                fn from_c(t: Self::T) -> Self {
                    ($(t.$idx),+,)
                }
            }

            impl<$($T: Debug),+> Debug for $Tuple<$($T),+> {
                #[allow(non_snake_case)]
                fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    let mut builder = f.debug_tuple("");
                    let $Tuple($(ref $T,)+) = *self;
                    $(
                        builder.field(&$T);
                    )+

                    builder.finish()
                }
            }
        )+
    }
}

tuple_impls! {
    /// Tuple with one generic type.
    Tuple1 {
        (0) -> A
    }
    /// Tuple with two generic types.
    Tuple2 {
        (0) -> A
        (1) -> B
    }
    /// Tuple with three generic types.
    Tuple3 {
        (0) -> A
        (1) -> B
        (2) -> C
    }
    /// Tuple with four generic types.
    Tuple4 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
    }
    /// Tuple with five generic types.
    Tuple5 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
    }
    /// Tuple with six generic types.
    Tuple6 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
    }
    /// Tuple with seven generic types.
    Tuple7 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
    }
    /// Tuple with eight generic types.
    Tuple8 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
    }
    /// Tuple with nine generic types.
    Tuple9 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
    }
    /// Tuple with ten generic types.
    Tuple10 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
    }
    /// Tuple with eleven generic types.
    Tuple11 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
    }
    /// Tuple with twelve generic types.
    Tuple12 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
    }
    /// Tuple with thirteen generic types.
    Tuple13 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
    }
    /// Tuple with fourteen generic types.
    Tuple14 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
    }
    /// Tuple with fifteen generic types.
    Tuple15 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
    }
    /// Tuple with sixteen generic types.
    Tuple16 {
        (0) -> A
        (1) -> B
        (2) -> C
        (3) -> D
        (4) -> E
        (5) -> F
        (6) -> G
        (7) -> H
        (8) -> I
        (9) -> J
        (10) -> K
        (11) -> L
        (12) -> M
        (13) -> N
        (14) -> O
        (15) -> P
    }
}