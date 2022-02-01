//! Definition of marker types.
use crate::fimo_marker;
use fimo_object::vtable::DefaultMarker;

pub use fimo_object::vtable::{SendMarker, SendSyncMarker, SyncMarker};

fimo_marker! {
    /// A marker which implements neither `Send` nor `Sync`.
    pub marker NoneMarker {
        _inner: DefaultMarker,
    }
}
