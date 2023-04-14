//! Implementation of the module.
use crate::FimoActixServer;
use fimo_actix_int::{
    Callback, CallbackId, IFimoActix, IFimoActixExt, ScopeBuilder, ScopeBuilderId, ServerStatus,
};
use fimo_core_int::settings::{
    ISettingsRegistryExt, SettingsEventCallbackHandle, SettingsEventCallbackId, SettingsItem,
    SettingsPath,
};
use fimo_core_int::IFimoCore;
use fimo_ffi::error::{Error, ErrorKind};
use fimo_ffi::provider::{request_obj, IProvider};
use fimo_ffi::type_id::StableTypeId;
use fimo_ffi::{DynObj, ObjBox, Object, Version};
use fimo_module::context::{IInterface, IInterfaceContext};
use fimo_module::module::{Interface, ModuleBuilderBuilder};
use fimo_module::{QueryBuilder, VersionQuery};
use std::fmt::{Debug, Formatter};
use std::path::Path;

#[cfg(feature = "module")]
mod core_bindings;

/// Struct implementing the `fimo-actix` interface.
#[derive(Object, StableTypeId)]
#[name("FimoActixInterface")]
#[uuid("d7eeb555-6cdc-412e-9d2b-b10f3069c298")]
#[interfaces(IInterface, IFimoActix)]
pub struct ActixInterface<'a> {
    server: FimoActixServer<String>,
    context: &'a DynObj<dyn IInterfaceContext + 'a>,
    core: &'a DynObj<dyn IFimoCore + 'a>,
    #[allow(clippy::type_complexity)]
    scope_builder: Option<(SettingsEventCallbackId, ScopeBuilderId)>,
}

impl Debug for ActixInterface<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FimoActixInterface")
            .field("server", &self.server)
            .field("context", &(self.context as *const _))
            .finish_non_exhaustive()
    }
}

impl Drop for ActixInterface<'_> {
    fn drop(&mut self) {
        if let Some((id, scope_id)) = self.scope_builder.take() {
            let registry = self.core.settings();
            let handle = unsafe { SettingsEventCallbackHandle::from_raw_parts(id, registry) };
            registry
                .unregister_callback(handle)
                .expect("could not unregister the core binding");

            self.unregister_scope_raw(scope_id)
                .expect("could not unregister the core scope");
        }
    }
}

impl IProvider for ActixInterface<'_> {
    fn provide<'a>(&'a self, demand: &mut fimo_ffi::provider::Demand<'a>) {
        demand.provide_obj::<dyn IFimoActix + 'a>(fimo_ffi::ptr::coerce_obj(self));
    }
}

impl IInterface for ActixInterface<'_> {
    fn name(&self) -> &str {
        ActixInterface::NAME
    }

    fn version(&self) -> Version {
        ActixInterface::VERSION
    }

    fn extensions(&self) -> &[fimo_ffi::String] {
        &[]
    }
}

impl IFimoActix for ActixInterface<'_> {
    #[inline]
    fn start(&self) -> ServerStatus {
        self.server.start()
    }

    #[inline]
    fn stop(&self) -> ServerStatus {
        self.server.stop()
    }

    #[inline]
    fn pause(&self) -> ServerStatus {
        self.server.pause()
    }

    #[inline]
    fn resume(&self) -> ServerStatus {
        self.server.resume()
    }

    #[inline]
    fn restart(&self) -> ServerStatus {
        self.server.restart()
    }

    #[inline]
    fn get_server_status(&self) -> ServerStatus {
        self.server.get_server_status()
    }

    #[inline]
    fn register_scope_raw(
        &self,
        path: &str,
        builder: ScopeBuilder,
    ) -> fimo_module::Result<ScopeBuilderId> {
        self.server.register_scope(path, builder)
    }

    #[inline]
    fn unregister_scope_raw(&self, id: ScopeBuilderId) -> fimo_module::Result<()> {
        self.server.unregister_scope(id)
    }

    #[inline]
    fn register_callback_raw(&self, f: Callback) -> fimo_module::Result<CallbackId> {
        self.server.register_callback(f)
    }

    #[inline]
    fn unregister_callback_raw(&self, id: CallbackId) -> fimo_module::Result<()> {
        self.server.unregister_callback(id)
    }
}

const REQUIRED_CORE_VERSION: VersionQuery = VersionQuery::Minimum(Version::new_short(0, 1, 0));

impl Interface for ActixInterface<'_> {
    type Result<'a> = ActixInterface<'a>;
    const NAME: &'static str = QueryBuilder.name::<dyn IFimoActix>();
    const VERSION: Version = Version::new_short(0, 1, 0);

    fn extensions(_feature: Option<&str>) -> Vec<String> {
        vec![]
    }

    fn dependencies(feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        if feature.is_none() {
            vec![QueryBuilder.query_version::<dyn IFimoCore>(REQUIRED_CORE_VERSION)]
        } else {
            vec![]
        }
    }

    fn optional_dependencies(_feature: Option<&str>) -> Vec<fimo_module::InterfaceQuery> {
        vec![]
    }

    fn construct<'a>(
        _module_root: &Path,
        context: &'a DynObj<dyn IInterfaceContext + 'a>,
    ) -> fimo_module::Result<ObjBox<Self::Result<'a>>> {
        let core = context
            .get_interface(QueryBuilder.query_version::<dyn IFimoCore>(REQUIRED_CORE_VERSION))?;
        let core = request_obj::<dyn IFimoCore + 'a>(core)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "The core interface was not found"))?;

        let settings_path = SettingsPath::new("fimo-actix").unwrap();
        let settings = core
            .settings()
            .read_or(settings_path, ModuleSettings::default())
            .map_err(|_| Error::from(ErrorKind::FailedPrecondition))?;

        let address = format!("127.0.0.1:{}", settings.port);
        let mut server = ActixInterface {
            server: FimoActixServer::new(address),
            context,
            core,
            scope_builder: None,
        };

        if settings.bind_core {
            bind_core(&mut server, core);
        }

        Ok(ObjBox::new(server))
    }
}

fimo_module::module!(|path, features| {
    Ok(
        ModuleBuilderBuilder::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .with_interface::<ActixInterface<'_>>()
            .build(path, features),
    )
});

fn bind_core<'a>(server: &mut ActixInterface<'a>, core: &DynObj<dyn IFimoCore + 'a>) {
    let (builder, callback) = core_bindings::scope_builder(core);
    let scope = server
        .register_scope("/core", builder)
        .expect("could not register `core` scope");

    let (scope_id, _) = scope.into_raw_parts();
    let (id, _) = callback.into_raw_parts();
    server.scope_builder = Some((id, scope_id));
}

#[derive(Copy, Clone)]
struct ModuleSettings {
    port: u16,
    bind_core: bool,
}

impl Default for ModuleSettings {
    fn default() -> Self {
        Self {
            port: 8080,
            bind_core: true,
        }
    }
}

impl From<ModuleSettings> for SettingsItem {
    fn from(s: ModuleSettings) -> Self {
        let mut item = SettingsItem::new_object();
        let map = item.as_map_mut().unwrap();
        map.insert("port".into(), s.port.into());
        map.insert("bind_core".into(), s.bind_core.into());
        item
    }
}

impl TryFrom<SettingsItem> for ModuleSettings {
    type Error = Error;

    fn try_from(value: SettingsItem) -> Result<Self, Self::Error> {
        let mut map = value
            .into_map()
            .ok_or_else(|| Error::new(ErrorKind::InvalidArgument, "Expected map"))?;

        let path_err = || Error::from(ErrorKind::NotFound);
        let err_f = |_e| Error::from(ErrorKind::InvalidArgument);

        let port: u16 = map
            .remove("port")
            .ok_or_else(path_err)?
            .try_into()
            .map_err(err_f)?;
        let bind_core: bool = map
            .remove("bind_core")
            .ok_or_else(path_err)?
            .try_into()
            .map_err(err_f)?;

        Ok(Self { port, bind_core })
    }
}
