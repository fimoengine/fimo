use fimo_actix_interface::actix::{web, HttpResponse, Responder, Scope};
use fimo_core_interface::rust::{
    settings_registry::{
        SettingsEvent, SettingsEventCallbackHandle, SettingsItem, SettingsRegistryPath, ROOT_PATH,
    },
    FimoCore,
};
use parking_lot::Mutex;
use std::collections::BTreeMap;

struct CoreSettings {
    root: Mutex<BTreeMap<String, SettingsItem>>,
}

async fn index() -> impl Responder {
    "Hello world!"
}

async fn settings(data: web::Data<CoreSettings>) -> impl Responder {
    let root = data.root.lock();
    HttpResponse::Ok().json(&*root)
}

pub(crate) fn scope_builder(
    core: &FimoCore,
) -> (
    impl Fn(Scope) -> Scope + Send + Sync,
    SettingsEventCallbackHandle<'_>,
) {
    let registry = core.get_settings_registry();
    let handle = registry
        .register_callback(
            ROOT_PATH,
            Box::new(|_p: &SettingsRegistryPath, _e: SettingsEvent<'_>| {}),
        )
        .unwrap();

    let root = registry.read_all();
    let data = web::Data::new(CoreSettings {
        root: Mutex::new(root),
    });

    let func = move |scope: Scope| {
        scope
            .app_data(data.clone())
            .route("/index", web::get().to(index))
            .route("/settings", web::get().to(settings))
    };

    (func, handle)
}
