use fimo_actix_int::actix::{web, HttpResponse, Responder, Scope};
use fimo_core_int::settings::{
    ISettingsRegistry, ISettingsRegistryExt, ISettingsRegistryInner, SettingsEvent,
    SettingsEventCallbackHandle, SettingsItem, SettingsPath, SettingsPathBuf,
};
use fimo_core_int::IFimoCore;
use fimo_ffi::DynObj;
use futures::lock::Mutex;
use serde::Serialize;
use std::sync::mpsc::Receiver;
use std::time::SystemTime;

struct CoreSettings {
    inner: Mutex<CoreSettingsInner>,
}

struct CoreSettingsInner {
    root: SettingsItem,
    events: Vec<Event>,
    rx: Receiver<Event>,
}

impl CoreSettingsInner {
    fn process_events(&mut self) {
        for event in self.rx.try_iter() {
            match &event {
                Event::Remove { path, .. } => {
                    let _ = self.root.remove(path);
                }
                Event::Write { item, path, .. } => {
                    let _ = self.root.write(path, item.clone());
                }
            }

            self.events.push(event);
        }
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
enum Event {
    Remove {
        time: SystemTime,
        path: SettingsPathBuf,
    },
    Write {
        time: SystemTime,
        item: SettingsItem,
        path: SettingsPathBuf,
    },
}

async fn index() -> impl Responder {
    "Hello world!"
}

async fn settings(data: web::Data<CoreSettings>) -> impl Responder {
    let mut inner = data.inner.lock().await;
    inner.process_events();
    HttpResponse::Ok().json(&inner.root)
}

async fn settings_events(data: web::Data<CoreSettings>) -> impl Responder {
    let mut inner = data.inner.lock().await;
    inner.process_events();
    HttpResponse::Ok().json(&inner.events)
}

pub(crate) fn scope_builder<'a>(
    core: &'a DynObj<dyn IFimoCore + '_>,
) -> (
    impl Fn(Scope) -> Scope + Send + Sync,
    SettingsEventCallbackHandle<'a, DynObj<dyn ISettingsRegistry + 'a>>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let callback = move |inner: &'_ DynObj<dyn ISettingsRegistryInner + '_>,
                         path: &SettingsPath,
                         event: SettingsEvent| match event {
        SettingsEvent::Removed => {
            let _ = tx.send(Event::Remove {
                time: SystemTime::now(),
                path: path.to_path_buf(),
            });
        }
        SettingsEvent::Updated => {
            let item = inner.read(path).unwrap().unwrap();

            let _ = tx.send(Event::Write {
                time: SystemTime::now(),
                item,
                path: path.to_path_buf(),
            });
        }
    };

    let registry = core.settings();
    let handle = registry
        .register_callback(SettingsPath::root(), callback)
        .unwrap();

    let root = registry.read(SettingsPath::root()).unwrap().unwrap();
    let data = web::Data::new(CoreSettings {
        inner: Mutex::new(CoreSettingsInner {
            root,
            events: vec![],
            rx,
        }),
    });

    let func = move |scope: Scope| {
        scope
            .app_data(data.clone())
            .route("/index", web::get().to(index))
            .route("/settings", web::get().to(settings))
            .route("/settings_events", web::get().to(settings_events))
    };

    (func, handle)
}
