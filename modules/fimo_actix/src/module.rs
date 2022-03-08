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
use fimo_ffi::ptr::{IBase, IBaseExt};
use fimo_ffi::{DynObj, ObjArc, ObjectId, Version};
use fimo_module::{
    FimoInterface, IModule, IModuleInstance, IModuleInterface, IModuleLoader, ModuleInfo,
};
use std::fmt::{Debug, Formatter};
use std::path::Path;

#[cfg(feature = "module")]
mod core_bindings;

/// Name of the module.
pub const MODULE_NAME: &str = "fimo_actix";

/// Struct implementing the `fimo-actix` interface.
#[derive(ObjectId)]
#[fetch_vtable(
    uuid = "d7eeb555-6cdc-412e-9d2b-b10f3069c298",
    interfaces(IModuleInterface, IFimoActix)
)]
pub struct FimoActixInterface {
    server: FimoActixServer<String>,
    parent: ObjArc<DynObj<dyn IModuleInstance>>,
    #[allow(clippy::type_complexity)]
    core: Option<(
        ObjArc<DynObj<dyn IFimoCore>>,
        SettingsEventCallbackId,
        ScopeBuilderId,
    )>,
}

impl Debug for FimoActixInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(FimoActixInterface)")
    }
}

impl IModuleInterface for FimoActixInterface {
    #[inline]
    fn as_inner(&self) -> &DynObj<dyn IBase + Send + Sync> {
        let inner = fimo_ffi::ptr::coerce_obj::<_, dyn IFimoActix + Send + Sync>(self);
        inner.cast_super()
    }

    #[inline]
    fn name(&self) -> &str {
        <dyn IFimoActix>::NAME
    }

    #[inline]
    fn version(&self) -> Version {
        <dyn IFimoActix>::VERSION
    }

    #[inline]
    fn extensions(&self) -> &[&str] {
        <dyn IFimoActix>::EXTENSIONS
    }

    #[inline]
    fn extension(&self, _name: &str) -> Option<&DynObj<dyn IBase + Send + Sync>> {
        None
    }

    #[inline]
    fn instance(&self) -> ObjArc<DynObj<dyn IModuleInstance>> {
        self.parent.clone()
    }
}

impl IFimoActix for FimoActixInterface {
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

impl Drop for FimoActixInterface {
    fn drop(&mut self) {
        if let Some((core, id, scope_id)) = self.core.take() {
            let registry = core.settings();
            let handle = unsafe { SettingsEventCallbackHandle::from_raw_parts(id, registry) };
            registry
                .unregister_callback(handle)
                .expect("could not unregister the core binding");

            self.unregister_scope_raw(scope_id)
                .expect("could not unregister the core scope");
        }
    }
}

fn module_info() -> ModuleInfo {
    ModuleInfo {
        name: MODULE_NAME.into(),
        version: <dyn IFimoActix>::VERSION.into(),
    }
}

fimo_module::rust_module!(load_module);

fn load_module(
    loader: &'static DynObj<dyn IModuleLoader>,
    path: &Path,
) -> fimo_module::Result<ObjArc<DynObj<dyn IModule>>> {
    let module = fimo_module::module::Module::new(module_info(), path, loader, |module| {
        let builder = fimo_module::module::InstanceBuilder::new(module);

        let desc = <dyn IFimoActix>::new_descriptor();
        let deps = &[<dyn IFimoCore>::new_descriptor()];
        let f = |instance, mut deps: Vec<_>| {
            // we only have one dependency so it must reside as the first element of the vec.
            let core = fimo_module::try_downcast_arc::<dyn IFimoCore, _>(deps.remove(0))?;

            let settings_path = SettingsPath::new("fimo-actix").unwrap();
            let settings = core
                .settings()
                .read_or(settings_path, ModuleSettings::default())
                .map_err(|_| Error::from(ErrorKind::FailedPrecondition))?;

            let address = format!("127.0.0.1:{}", settings.port);
            let mut server = ObjArc::new(FimoActixInterface {
                server: FimoActixServer::new(address),
                parent: ObjArc::coerce_obj(instance),
                core: None,
            });

            if settings.bind_core {
                server = bind_core(server, core)
            }

            Ok(ObjArc::coerce_obj(server))
        };

        let instance = builder.interface(desc, deps, f).build();
        Ok(instance)
    });
    Ok(ObjArc::coerce_obj(module))
}

fn bind_core(
    mut server: ObjArc<FimoActixInterface>,
    core: ObjArc<DynObj<dyn IFimoCore>>,
) -> ObjArc<FimoActixInterface> {
    let (builder, callback) = core_bindings::scope_builder(&*core);
    let scope = server
        .register_scope("/core", builder)
        .expect("could not register `core` scope");

    let (scope_id, _) = scope.into_raw_parts();
    let inner = ObjArc::get_mut(&mut server).unwrap();
    let (id, _) = callback.into_raw_parts();
    inner.core = Some((core, id, scope_id));

    server
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
