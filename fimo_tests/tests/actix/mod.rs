use fimo_actix_int::{IFimoActix, ServerStatus};
use fimo_core_int::settings::ISettingsRegistryExt;
use fimo_core_int::IFimoCore;
use fimo_ffi::{error::wrap_error, provider::request_obj};
use fimo_module::{context::IContext, Error, ErrorKind, QueryBuilder};
use fimo_tests::ContextBuilder;
use reqwest::Url;

#[test]
fn startup_server() -> Result<(), Error> {
    ContextBuilder::new()
        .with_actix()
        .with_core()
        .build(|context| {
            let core = context
                .get_interface(QueryBuilder.query_version::<dyn IFimoCore>(super::CORE_VERSION))?;
            let core = request_obj::<dyn IFimoCore + '_>(core)
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Core interface not found"))?;

            let actix = context.get_interface(
                QueryBuilder.query_version::<dyn IFimoActix>(super::ACTIX_VERSION),
            )?;
            let actix = request_obj::<dyn IFimoActix + '_>(actix)
                .ok_or_else(|| Error::new(ErrorKind::NotFound, "Actix interface not found"))?;

            assert_eq!(actix.get_server_status(), ServerStatus::Stopped);
            assert_eq!(actix.start(), ServerStatus::Running);
            assert_eq!(actix.get_server_status(), ServerStatus::Running);

            let url = Url::parse("http://127.0.0.1:8080/core/settings")
                .map_err(|e| Error::new(ErrorKind::Internal, wrap_error(e)))?;
            let body = reqwest::blocking::get(url)
                .map_err(|e| Error::new(ErrorKind::Internal, wrap_error(e)))?
                .text()
                .map_err(|e| Error::new(ErrorKind::Internal, wrap_error(e)))?;

            println!("{}", body);

            let root_item = core.settings().read_all();
            let body_root = serde_json::from_str(body.as_str()).unwrap();

            assert_eq!(root_item, body_root);

            assert_eq!(actix.stop(), ServerStatus::Stopped);
            assert_eq!(actix.get_server_status(), ServerStatus::Stopped);

            Ok(())
        })
}
