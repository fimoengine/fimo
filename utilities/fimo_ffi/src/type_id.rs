//! Implementation of a stable [`std::any::TypeId`] alternative.

use fimo_ffi_codegen::CTypeBridge;
pub use fimo_ffi_codegen::StableTypeId;

/// Alternative for a [`std::any::TypeId`] for identifying types at
/// runtime. The id of a type can be configured by implementing the
/// [`TypeInfo`] trait. The default implementation does not guarantee
/// any stability across multiple compiler versions and environments.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, StableTypeId, CTypeBridge)]
#[uuid("f439d7b5-1a5a-481c-ad07-2c499c05c913")]
pub struct StableTypeId {
    #[ignored]
    id: [u8; 16],
}

impl StableTypeId {
    /// Returns the [`struct@StableTypeId`] of the type this generic function
    /// has been instantiated with.
    pub const fn of<T: 'static + ?Sized>() -> StableTypeId {
        Self {
            id: <T as StableTypeIdProvider>::ID.to_ne_bytes(),
        }
    }
}

trait StableTypeIdProvider: 'static {
    const ID: u128;
}

impl<T: TypeInfo + ?Sized> StableTypeIdProvider for T {
    const ID: u128 = hash_type_info::<T>();
}

/// Trait collecting the type information used for computing
/// a stable type id.
pub trait TypeInfo: 'static {
    /// Seed id of the type.
    const ID: uuid::Uuid;

    /// Variant of the type.
    const VARIANT: usize = 0;

    /// Type name.
    const NAME: &'static str = std::any::type_name::<Self>();

    /// Ids of the members.
    const MEMBER_IDS: &'static [StableTypeId] = &[];
}

macro_rules! primitive_impl {
    ($($T:ty, $UUID:literal);*) => {
        $(
            impl TypeInfo for $T {
                const ID: uuid::Uuid = uuid::uuid!($UUID);
            }
        )*
    };
}

macro_rules! generic_impl {
    ($($T:ty, $UUID:literal, $NAME:literal);*) => {
        $(
            impl<T: 'static + ?Sized> TypeInfo for $T {
                const ID: uuid::Uuid = uuid::uuid!($UUID);

                const NAME: &'static str = $NAME;

                const MEMBER_IDS: &'static [StableTypeId] = &[
                    StableTypeId::of::<T>()
                ];
            }
        )*
    };
}

macro_rules! tuple_impl {
    ($UUID:literal; $(($($T:ident),+));+) => {
        $(
            impl<$($T: 'static),+> TypeInfo for ($($T),+,) {
                const ID: uuid::Uuid = uuid::uuid!($UUID);

                const MEMBER_IDS: &'static [StableTypeId] = &[
                    $(StableTypeId::of::<$T>()),*
                ];
            }
        )+
    };
}

primitive_impl! {
    bool, "d5581ffb-3a00-496e-96b0-a10e5eac256b";
    char, "513b876e-3d7f-431d-a4a3-4269a40e43be";
    f32, "d4e0675d-c7b7-4f47-9412-f6981252fc74";
    f64, "cbbd8323-eab0-45ba-84fc-67ed714d385b";
    i8, "7c79baf6-a432-4104-8f1e-b903da78e36c";
    i16, "d3d401a6-a60f-459a-82ea-7929479e69c6";
    i32, "e6f8fada-3949-4304-9331-515c723fa728";
    i64, "6db55ded-eefc-4d42-bc5f-0f54560759ed";
    i128, "bc946179-04ed-4c82-a501-be09a74b9b53";
    isize, "262a3d3e-374a-47c7-95e6-39e61069f49b";
    u8, "9c155aa4-7448-4344-a06d-44c15137c87a";
    u16, "717cbde3-87fb-473b-8d78-3664f8e405ad";
    u32, "0d66549d-8fba-4a96-a080-01355de829d1";
    u64, "68fe2e45-d600-4b75-9b8e-8e3ee8c874bb";
    u128, "69a56341-3083-464c-abdd-633ee9798691";
    (), "0a7c6633-f58d-47e5-b961-0dd2da53b972";
    usize, "49c62fca-2434-4d94-ba07-b0974abff898"
}

generic_impl! {
    *const T, "e38fd003-2714-405f-9089-38143395e923", "const ptr";
    *mut T, "5304488c-6826-49f6-a160-e5d28f0bc499", "mut ptr";
    &'static T, "ccca75f4-0e48-4315-a181-70a78591022d", "const ref";
    &'static mut T, "bc870783-87e6-42e1-aa6f-4299916b7a82", "mut ref";
    std::ptr::NonNull<T>, "a7e279ee-c2ef-44b7-97a3-2473764fda76", "NonNull";
    std::marker::PhantomData<T>, "13cd6b13-64e2-4caf-ac11-10b0c97062aa", "PhantomData"
}

tuple_impl! {
    "6a577874-f552-473b-a73f-64bb6db65a65";
    (A);
    (A, B);
    (A, B, C);
    (A, B, C, D);
    (A, B, C, D, E);
    (A, B, C, D, E, F);
    (A, B, C, D, E, F, G);
    (A, B, C, D, E, F, G, H);
    (A, B, C, D, E, F, G, H, I);
    (A, B, C, D, E, F, G, H, I, J);
    (A, B, C, D, E, F, G, H, I, J, K);
    (A, B, C, D, E, F, G, H, I, J, K, L);
    (A, B, C, D, E, F, G, H, I, J, K, L, M);
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N);
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P)
}

// General implementation uses generates an unstable id by hashing a [`std::any::TypeId`].
impl<T: ?Sized + 'static> TypeInfo for T {
    default const ID: uuid::Uuid = uuid::uuid!("eec3abd9-46af-4bde-a6f0-27ff0652d3ea");
    default const VARIANT: usize = unsafe { std::mem::transmute(std::any::TypeId::of::<T>()) };
    default const NAME: &'static str = std::any::type_name::<Self>();
    default const MEMBER_IDS: &'static [StableTypeId] = &[];
}

impl TypeInfo for str {
    const ID: uuid::Uuid = uuid::uuid!("db9345e0-bae6-43e0-80f2-c420571fd333");
}

impl<T: 'static> TypeInfo for [T] {
    const ID: uuid::Uuid = uuid::uuid!("a020d7de-fe7d-49f6-9ef8-21a635673cdc");
    const NAME: &'static str = "slice";
    const MEMBER_IDS: &'static [StableTypeId] = &[StableTypeId::of::<T>()];
}

impl<T: 'static, const N: usize> TypeInfo for [T; N] {
    const ID: uuid::Uuid = uuid::uuid!("15a66b91-7727-40bc-9faf-346701cba1d2");
    const VARIANT: usize = N;
    const NAME: &'static str = "array";
    const MEMBER_IDS: &'static [StableTypeId] = &[StableTypeId::of::<T>()];
}

const fn hash_type_info<T: TypeInfo + ?Sized>() -> u128 {
    use std::intrinsics::{const_allocate, const_deallocate};
    use std::ptr::copy_nonoverlapping;

    let size = 16 + 8 + T::NAME.len() + (T::MEMBER_IDS.len() * 16);
    let buf = unsafe { const_allocate(size, 1) };

    let mut ptr = buf;
    unsafe {
        // Copy type uuid.
        let id = T::ID.as_bytes();
        copy_nonoverlapping(id.as_ptr(), ptr, 16);
        ptr = ptr.add(16);

        // Copy type variant.
        let variant: [u8; 8] = T::VARIANT.to_ne_bytes();
        copy_nonoverlapping(variant.as_ptr(), ptr, 8);
        ptr = ptr.add(8);

        // Copy type name.
        let name = T::NAME;
        copy_nonoverlapping(name.as_ptr(), ptr, name.len());
        ptr = ptr.add(name.len());

        // Copy member ids.
        let mut i = 0;
        loop {
            if i == T::MEMBER_IDS.len() {
                break;
            }

            let id = T::MEMBER_IDS[i].id;
            copy_nonoverlapping(id.as_ptr(), ptr, 16);
            ptr = ptr.add(16);
            i += 1;
        }
    }

    // Generated randomly.
    const SEED: u64 = 10362567237498601785;
    let slice = unsafe { std::slice::from_raw_parts(buf, size) };
    let res = xxhash_rust::const_xxh3::xxh3_128_with_seed(slice, SEED);

    unsafe { const_deallocate(buf, size, 1) };

    res
}
