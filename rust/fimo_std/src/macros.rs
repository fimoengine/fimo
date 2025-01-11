macro_rules! cfg_internal {
    ($($item:item)*) => {
        $(
            #[cfg(any(fimo_internals, doc))]
            #[doc(cfg(fimo_internals))]
            $item
        )*
    };
}
