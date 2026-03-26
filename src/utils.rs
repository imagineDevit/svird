use crate::constants::{CACHE_PATH, args::*, THIRTY_DAYS};
use crate::types::{Browsers, LaunchResult};
use serde_json::to_value;
use std::path::{Path, PathBuf};
use thirtyfour::BrowserCapabilitiesHelper;
use anyhow::Result;


/// Create the driver executable file using the selenium-manager executable
///
/// @browser: the browser to create the driver for
///
/// @returns: the path of the created driver
pub(crate) fn create_driver<P: AsRef<Path>>(browser: Browsers) -> Result<Option<String>> {

    let mut manager = selenium_manager::get_manager_by_browser(String::from(browser.as_str()))?;

    manager.set_cache_path(CACHE_PATH.to_string());
    manager.set_ttl(THIRTY_DAYS);

    manager.download_driver()?;

     let driver_path = manager.get_driver_path_in_cache()?;

     Ok(Some(driver_path.to_string_lossy().to_string()))

}

pub(crate) async fn check_port_available(port: u16) -> LaunchResult<bool> {
    match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Add browser options to browser capabilities
pub(crate) fn add_args<T>(capabilities: Vec<(&str, Option<&str>)>, caps: &mut T) -> LaunchResult<()>
where
    T: BrowserCapabilitiesHelper,
{
    if capabilities.len() > 0 {
        let args = capabilities
            .into_iter()
            .map(|(key, value)| match value {
                None => key.to_string(),
                Some(value) => format!("{key}={value}"),
            })
            .collect::<Vec<String>>();

        caps.insert_browser_option(ARGS, to_value(args)?)?;
    }

    Ok(())
}
