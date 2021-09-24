use fimo_actix_interface::{FimoActix, FimoActixCaster, ServerStatus};
use fimo_module_core::DynArc;
use std::error::Error;
use reqwest::Url;

fn initialize() -> Result<DynArc<FimoActix, FimoActixCaster>, Box<dyn Error>> {
    let (core_instance, core_interface) = module_loading::get_core_interface()?;
    module_loading::get_actix_interface(&core_instance, &core_interface)
}

#[test]
fn startup_server() -> Result<(), Box<dyn Error>> {
    let actix = initialize()?;
    assert_eq!(actix.get_server_status(), ServerStatus::Stopped);
    assert_eq!(actix.start(), ServerStatus::Running);
    assert_eq!(actix.get_server_status(), ServerStatus::Running);

    let url = Url::parse("http://127.0.0.1:8080/core/settings")?;
    let body = reqwest::blocking::get(url)?.text()?;

    println!("{}", body);

    assert_eq!(actix.stop(), ServerStatus::Stopped);
    assert_eq!(actix.get_server_status(), ServerStatus::Stopped);

    Ok(())
}
