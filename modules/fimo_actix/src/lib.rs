//! Implementation of the `fimo-actix` interface.
#![feature(unboxed_closures)]
#![feature(async_closure)]
#![feature(fn_traits)]
#![feature(c_unwind)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    rustdoc::broken_intra_doc_links
)]
use fimo_actix_interface::actix::dev::Server;
use fimo_actix_interface::actix::rt::{Arbiter, System};
use fimo_actix_interface::actix::{App, HttpServer, Scope};
use fimo_actix_interface::{
    Callback, CallbackId, ScopeBuilder, ScopeBuilderId, ServerEvent, ServerStatus,
};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::net::ToSocketAddrs;
use std::ops::RangeFrom;
use std::sync::mpsc::Sender;

#[cfg(feature = "module")]
pub mod module;

/// Server manager
pub struct FimoActixServer<A: 'static + ToSocketAddrs + Sync> {
    inner: Mutex<ActixServerInner<A>>,
}

struct ActixServerInner<A: 'static + ToSocketAddrs + Sync> {
    address: A,
    arbiter: Arbiter,
    status: ServerStatus,
    server: Option<Server>,
    scope_ids: RangeFrom<usize>,
    callback_ids: RangeFrom<usize>,
    scopes: BTreeMap<String, ScopeBuilder>,
    registered_scopes: BTreeMap<usize, String>,
    callbacks: BTreeMap<usize, Callback>,
}

impl<A: 'static + ToSocketAddrs + Sync> FimoActixServer<A> {
    /// Constructs a new `FimoActixServer`.
    ///
    /// The server starts after [`FimoActixServer::start`] is called.
    pub fn new(address: A) -> Self {
        Self {
            inner: Mutex::new(ActixServerInner::new(address)),
        }
    }

    /// Starts the server.
    ///
    /// Returns the status of the server after the operation has been completed.
    pub fn start(&self) -> ServerStatus {
        self.inner.lock().start()
    }

    /// Stops the server.
    ///
    /// Returns the status of the server after the operation has been completed.
    pub fn stop(&self) -> ServerStatus {
        self.inner.lock().stop()
    }

    /// Pauses the server.
    ///
    /// Returns the status of the server after the operation has been completed.
    pub fn pause(&self) -> ServerStatus {
        self.inner.lock().pause()
    }

    /// Resumes the server.
    ///
    /// Returns the status of the server after the operation has been completed.
    pub fn resume(&self) -> ServerStatus {
        self.inner.lock().resume()
    }

    /// Restarts the server.
    ///
    /// Returns the status of the server after the operation has been completed.
    pub fn restart(&self) -> ServerStatus {
        self.inner.lock().restart()
    }

    /// Fetches the execution status of the server,
    pub fn get_server_status(&self) -> ServerStatus {
        self.inner.lock().get_server_status()
    }

    /// Registers a new scope to the server.
    ///
    /// The scopes will be added to the server on startup or restart.
    pub fn register_scope(&self, path: &str, builder: ScopeBuilder) -> Option<ScopeBuilderId> {
        self.inner.lock().register_scope(path, builder)
    }

    /// Unregisters a scope from the server.
    ///
    /// Has no effect on the currently running server.
    ///
    /// # Panic
    ///
    /// The `id` must stem from a call to [`FimoActixServer::register_scope`].
    pub fn unregister_scope(&self, id: ScopeBuilderId) {
        self.inner.lock().unregister_scope(id)
    }

    /// Registers a callback to the server.
    pub fn register_callback(&self, callback: Callback) -> CallbackId {
        self.inner.lock().register_callback(callback)
    }

    /// Unregisters a callback from the server.
    ///
    /// # Panic
    ///
    /// The `id` must stem from a call to [`FimoActixServer::register_callback`].
    pub fn unregister_callback(&self, id: CallbackId) {
        self.inner.lock().unregister_callback(id)
    }
}

impl<A: 'static + ToSocketAddrs + Sync> std::fmt::Debug for FimoActixServer<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FimoActixServer)")
    }
}

impl<A: 'static + ToSocketAddrs + Sync> ActixServerInner<A> {
    fn new(address: A) -> Self {
        let arbiter = std::thread::spawn(|| {
            let _ = System::new();
            Arbiter::new()
        })
        .join()
        .unwrap();

        Self {
            address,
            arbiter,
            status: ServerStatus::Stopped,
            server: None,
            scope_ids: (0usize..),
            callback_ids: (0usize..),
            scopes: Default::default(),
            registered_scopes: Default::default(),
            callbacks: Default::default(),
        }
    }

    fn start(&mut self) -> ServerStatus {
        if matches!(self.status, ServerStatus::Stopped) {
            self.dispatch_event(ServerEvent::Starting);

            if self.start_server() {
                self.status = ServerStatus::Running;
                self.dispatch_event(ServerEvent::Started);
            } else {
                self.dispatch_event(ServerEvent::Aborted)
            }
        }

        self.status
    }

    fn stop(&mut self) -> ServerStatus {
        if matches!(self.status, ServerStatus::Running | ServerStatus::Paused) {
            self.dispatch_event(ServerEvent::Stopping);

            if self.stop_server() {
                self.status = ServerStatus::Stopped;
                self.dispatch_event(ServerEvent::Stopped);
            } else {
                self.dispatch_event(ServerEvent::Aborted)
            }
        }

        self.status
    }

    fn pause(&mut self) -> ServerStatus {
        if matches!(self.status, ServerStatus::Running) {
            self.dispatch_event(ServerEvent::Pausing);

            if self.pause_server() {
                self.status = ServerStatus::Paused;
                self.dispatch_event(ServerEvent::Paused);
            } else {
                self.dispatch_event(ServerEvent::Aborted)
            }
        }

        self.status
    }

    fn resume(&mut self) -> ServerStatus {
        if matches!(self.status, ServerStatus::Paused) {
            self.dispatch_event(ServerEvent::Resuming);

            if self.resume_server() {
                self.status = ServerStatus::Running;
                self.dispatch_event(ServerEvent::Resumed);
            } else {
                self.dispatch_event(ServerEvent::Aborted)
            }
        }

        self.status
    }

    fn restart(&mut self) -> ServerStatus {
        if matches!(self.status, ServerStatus::Running | ServerStatus::Paused) {
            self.dispatch_event(ServerEvent::Restarting);

            if self.stop_server() {
                if self.start_server() {
                    self.status = ServerStatus::Running;
                    self.dispatch_event(ServerEvent::Resumed);
                } else {
                    self.status = ServerStatus::Stopped;
                    self.dispatch_event(ServerEvent::Aborted)
                }
            } else {
                self.dispatch_event(ServerEvent::Aborted)
            }
        }

        self.status
    }

    fn dispatch_event(&mut self, event: ServerEvent) {
        for callback in self.callbacks.values_mut() {
            callback(event)
        }
    }

    fn start_server(&mut self) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();

        async fn builder<A: 'static + ToSocketAddrs + Sync>(
            address: &'static A,
            scopes: &'static BTreeMap<String, ScopeBuilder>,
            tx: Sender<Option<Server>>,
        ) {
            let server = {
                let server_builder = HttpServer::new(move || {
                    let mut app = App::new();
                    for (path, builder) in scopes {
                        let scope = Scope::new(path);
                        app = app.service(builder(scope));
                    }
                    app
                })
                .bind(address);
                if server_builder.is_err() {
                    return;
                }
                let server_builder = server_builder.unwrap();
                server_builder.run()
            };

            tx.send(Some(server.clone())).unwrap();
            server.await.unwrap();
        }

        // safety: extending the lifetimes is sound, because we are going to wait
        // until we receive a result. The references will be dropped before returning
        // to the caller.
        let address = unsafe { &*(&self.address as *const _) };
        let scopes = unsafe { &*(&self.scopes as *const _) };
        let future = builder::<A>(address, scopes, tx);
        self.arbiter.spawn(future);

        self.server = rx.recv().unwrap();
        self.server.is_some()
    }

    fn stop_server(&mut self) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();

        async fn stop_server(server: Server, tx: Sender<()>) {
            server.stop(true).await;
            tx.send(()).unwrap();
        }
        let future = stop_server(self.server.clone().unwrap(), tx);
        self.arbiter.spawn(future);
        let _ = rx.recv().unwrap();

        true
    }

    fn pause_server(&mut self) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();

        async fn pause_server(server: Server, tx: Sender<()>) {
            server.pause().await;
            tx.send(()).unwrap();
        }
        let future = pause_server(self.server.clone().unwrap(), tx);
        self.arbiter.spawn(future);
        let _ = rx.recv().unwrap();

        true
    }

    fn resume_server(&mut self) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();

        async fn resume_server(server: Server, tx: Sender<()>) {
            server.resume().await;
            tx.send(()).unwrap();
        }
        let future = resume_server(self.server.clone().unwrap(), tx);
        self.arbiter.spawn(future);
        let _ = rx.recv().unwrap();

        true
    }

    fn get_server_status(&self) -> ServerStatus {
        self.status
    }

    fn register_scope(&mut self, path: &str, builder: ScopeBuilder) -> Option<ScopeBuilderId> {
        if self.scopes.contains_key(path) {
            return None;
        }

        let id = self.scope_ids.next().unwrap();
        let scope_name = String::from(path);

        self.scopes.insert(scope_name.clone(), builder);
        self.registered_scopes.insert(id, scope_name);

        Some(unsafe { ScopeBuilderId::from_usize(id) })
    }

    fn unregister_scope(&mut self, id: ScopeBuilderId) {
        let id = usize::from(id);
        let scope = self.registered_scopes.remove(&id).unwrap();
        self.scopes.remove(&scope);
    }

    fn register_callback(&mut self, callback: Callback) -> CallbackId {
        let id = self.callback_ids.next().unwrap();
        self.callbacks.insert(id, callback);

        unsafe { CallbackId::from_usize(id) }
    }

    fn unregister_callback(&mut self, id: CallbackId) {
        let id = usize::from(id);
        self.callbacks.remove(&id).unwrap();
    }
}

impl<A: 'static + ToSocketAddrs + Sync> std::fmt::Debug for ActixServerInner<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(ActixServerInner)")
    }
}