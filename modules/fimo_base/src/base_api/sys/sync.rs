use emf_core_base_rs::ffi::collections::NonNullConst;
use emf_core_base_rs::ffi::sys::sync_handler::{
    SyncHandler as SyncHandlerFFI, SyncHandlerInterface, SyncHandlerVTable,
};
use emf_core_base_rs::ffi::{Bool, TypeWrapper};
use emf_core_base_rs::sys::sync_handler::{SyncHandler, SyncHandlerAPI};
use parking_lot::lock_api::RawMutex;
use parking_lot::Mutex;
use std::ptr::NonNull;

#[derive(Debug)]
struct DefaultSyncInternal {
    pub mutex: Mutex<()>,
}

#[derive(Debug)]
pub struct DefaultSync {
    internal: Box<DefaultSyncInternal>,
    interface: Box<SyncHandlerInterface>,
}

impl DefaultSync {
    const VTABLE: SyncHandlerVTable = SyncHandlerVTable {
        lock_fn: TypeWrapper(DefaultSync::lock_internal),
        try_lock_fn: TypeWrapper(DefaultSync::try_lock_internal),
        unlock_fn: TypeWrapper(DefaultSync::unlock_internal),
    };

    /// Create a new instance.
    #[inline]
    pub fn new() -> Self {
        let mut internal = Box::new(DefaultSyncInternal {
            mutex: Mutex::new(()),
        });

        let interface = SyncHandlerInterface {
            handler: Some(NonNull::from(&mut *internal).cast()),
            vtable: NonNullConst::from(&Self::VTABLE),
        };

        Self {
            internal,
            interface: Box::new(interface),
        }
    }

    #[inline]
    pub fn as_interface(&self) -> SyncHandler<'static> {
        // Safety: The handler is valid.
        unsafe { SyncHandler::from_raw(*self.interface) }
    }

    extern "C-unwind" fn lock_internal(handler: Option<NonNull<SyncHandlerFFI>>) {
        // Safety: Correct type is guaranteed.
        unsafe {
            handler
                .unwrap()
                .cast::<DefaultSyncInternal>()
                .as_mut()
                .mutex
                .raw()
                .lock()
        }
    }

    extern "C-unwind" fn try_lock_internal(handler: Option<NonNull<SyncHandlerFFI>>) -> Bool {
        // Safety: Correct type is guaranteed.
        if unsafe {
            handler
                .unwrap()
                .cast::<DefaultSyncInternal>()
                .as_mut()
                .mutex
                .raw()
                .try_lock()
        } {
            Bool::True
        } else {
            Bool::False
        }
    }

    extern "C-unwind" fn unlock_internal(handler: Option<NonNull<SyncHandlerFFI>>) {
        // Safety: Correct type is guaranteed.
        unsafe {
            handler
                .unwrap()
                .cast::<DefaultSyncInternal>()
                .as_mut()
                .mutex
                .raw()
                .unlock()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::base_api::sys::sync::DefaultSync;
    use emf_core_base_rs::sys::sync_handler::SyncHandlerAPI;
    use std::cell::Cell;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};

    #[test]
    fn lock() {
        struct CellSend<T>(Cell<T>);

        unsafe impl<T> Send for CellSend<T> {}
        unsafe impl<T> Sync for CellSend<T> {}

        let sync = Arc::new(DefaultSync::new());
        let data = Arc::new(CellSend(Cell::new(0usize)));

        let mut threads = Vec::new();

        const ITERATIONS: usize = 10000;
        for _ in 0..ITERATIONS {
            let sync_thr = Arc::clone(&sync);
            let data_thr = Arc::clone(&data);

            threads.push(std::thread::spawn(move || {
                let sync_i = sync_thr.as_interface();
                unsafe { sync_i.lock() };
                data_thr.0.set(data_thr.0.get() + 1);
                unsafe { sync_i.unlock() };
            }));
        }

        // Await for all threads to finish.
        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(data.0.get(), ITERATIONS);
    }

    #[test]
    fn try_lock() {
        let sync = Arc::new(DefaultSync::new());
        let data = Arc::new(AtomicBool::new(false));
        let barrier = Arc::new(Barrier::new(2));

        let sync_int = sync.as_interface();
        assert_eq!(unsafe { sync_int.try_lock() }, true);

        let t = {
            let sync_t = Arc::clone(&sync);
            let data_t = Arc::clone(&data);
            let barrier_t = Arc::clone(&barrier);

            std::thread::spawn(move || {
                let sync_int = sync_t.as_interface();
                assert_eq!(unsafe { sync_int.try_lock() }, false);
                barrier_t.wait();

                unsafe { sync_int.lock() };
                data_t.store(true, Ordering::Release);
                unsafe { sync_int.unlock() };
            })
        };

        barrier.wait();
        assert_eq!(data.load(Ordering::Acquire), false);
        unsafe { sync_int.unlock() };

        t.join().unwrap();
        assert_eq!(data.load(Ordering::Acquire), true);
    }
}
