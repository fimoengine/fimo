//! Definition of marker types.
use crate::fimo_marker;
use fimo_object::vtable::DefaultMarker;

fimo_marker! {
    /// A marker which implements neither `Send` nor `Sync`.
    pub marker NoneMarker {
        _inner: DefaultMarker,
    }
}

fimo_marker! {
    /// A marker which implements `Send`.
    #![requires(Send)]
    pub marker SendMarker {
        _inner: DefaultMarker,
    }
}

unsafe impl Send for SendMarker {}

fimo_marker! {
    /// A marker which implements `Sync`.
    #![requires(Sync)]
    pub marker SyncMarker {
        _inner: DefaultMarker,
    }
}

unsafe impl Sync for SyncMarker {}

fimo_marker! {
    /// A marker which implements both `Send` and `Sync`.
    #![requires(Send, Sync)]
    pub marker SendSyncMarker;
}
