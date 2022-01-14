use fimo_actix_interface::ServerStatus;
use fimo_module_core::{Error, ErrorKind};
use reqwest::Url;

#[test]
fn startup_server() -> Result<(), Error> {
    let (core_instance, core) = module_loading::get_core_interface()?;
    let actix = module_loading::get_actix_interface(&core_instance, &core)?;

    assert_eq!(actix.get_server_status(), ServerStatus::Stopped);
    assert_eq!(actix.start(), ServerStatus::Running);
    assert_eq!(actix.get_server_status(), ServerStatus::Running);

    let url = Url::parse("http://127.0.0.1:8080/core/settings")
        .map_err(|e| Error::new(ErrorKind::Internal, e))?;
    let body = reqwest::blocking::get(url)
        .map_err(|e| Error::new(ErrorKind::Internal, e))?
        .text()
        .map_err(|e| Error::new(ErrorKind::Internal, e))?;

    println!("{}", body);

    let root_item = core.get_settings_registry().read_all();
    let body_root = serde_json::from_str(body.as_str()).unwrap();

    assert_eq!(root_item, body_root);

    assert_eq!(actix.stop(), ServerStatus::Stopped);
    assert_eq!(actix.get_server_status(), ServerStatus::Stopped);

    Ok(())
}
