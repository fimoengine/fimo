use fimo_actix_interface::actix::{web, HttpResponse, Responder, Scope};
use fimo_core_interface::rust::{
    CallbackHandle, FimoCore, InterfaceGuard, SettingsItem, SettingsUpdateCallback,
};
use parking_lot::Mutex;

struct CoreSettings {
    root: Mutex<SettingsItem>,
}

async fn index() -> impl Responder {
    "Hello world!"
}

async fn settings(data: web::Data<CoreSettings>) -> impl Responder {
    let root = data.root.lock();
    HttpResponse::Ok().json(&*root)
}

pub(crate) fn scope_builder(
    mut core: InterfaceGuard<'_, dyn FimoCore>,
) -> (
    impl Fn(Scope) -> Scope + Send + Sync,
    CallbackHandle<SettingsUpdateCallback>,
) {
    let registry = core.as_settings_registry_mut();
    let handle = registry
        .register_callback("", Box::new(|_p, _e| {}))
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
