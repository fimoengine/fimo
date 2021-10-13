use fimo_actix_interface::actix::{web, HttpResponse, Responder, Scope};
use fimo_core_interface::rust::settings_registry::SettingsRegistryPathBuf;
use fimo_core_interface::rust::{
    settings_registry::{
        SettingsEvent, SettingsEventCallbackHandle, SettingsItem, SettingsRegistryPath,
    },
    FimoCore,
};
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
        path: SettingsRegistryPathBuf,
    },
    Write {
        time: SystemTime,
        item: SettingsItem,
        path: SettingsRegistryPathBuf,
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

pub(crate) fn scope_builder(
    core: &FimoCore,
) -> (
    impl Fn(Scope) -> Scope + Send + Sync,
    SettingsEventCallbackHandle<'_>,
) {
    let mut tmp_item = None;
    let (tx, rx) = std::sync::mpsc::channel();
    let callback = move |path: &SettingsRegistryPath, event: &SettingsEvent| match event {
        SettingsEvent::Remove { .. } => {
            let _ = tx.send(Event::Remove {
                time: SystemTime::now(),
                path: path.to_path_buf(),
            });
        }
        SettingsEvent::StartWrite { new } => tmp_item = Some(new.clone()),
        SettingsEvent::EndWrite { .. } => {
            let item = tmp_item.take().unwrap();

            let _ = tx.send(Event::Write {
                time: SystemTime::now(),
                item,
                path: path.to_path_buf(),
            });
        }
        SettingsEvent::AbortWrite => {
            tmp_item = None;
        }
    };

    let registry = core.get_settings_registry();
    let handle = registry
        .register_callback(SettingsRegistryPath::root(), Box::new(callback))
        .unwrap();

    let root = registry
        .read(SettingsRegistryPath::root())
        .unwrap()
        .unwrap();
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
