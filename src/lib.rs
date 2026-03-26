

mod types;
mod errors;
mod constants;

use crate::errors::DriverLaunchError;
use crate::errors::DriverLaunchError::CreateDriverError;
use crate::types::{Browsers, LaunchResult};
use crate::constants::*;

use serde_json::to_value;

use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::{env, fs};

use thirtyfour::prelude::*;

/// Create the selenium-manager executable file
fn create_manager_file() -> LaunchResult<()> {
    let mut manager_file = File::create(SELENIUM_MANAGER_PATH)?;

    manager_file.write_all(SELENIUM_DATA)?;

    let dir = env::current_dir()?;

    let manager_path = dir.join(SELENIUM_MANAGER_PATH);

    Command::new("chmod")
        .arg("+x")
        .arg(manager_path.clone())
        .output()?;

    Ok(())
}

/// Create the driver executable file using the selenium-manager executable
///
/// And returns the path of the created driver
fn create_driver(browser: Browsers) -> LaunchResult<Option<String>> {
    create_manager_file()?;

    if !fs::exists(CACHE_PATH)? {
        fs::create_dir(CACHE_PATH)?;
    }

    let result = Command::new(format!("./{}", SELENIUM_MANAGER_PATH))
        .args(&[
            "--browser",
            browser.as_str(),
            "--cache-path",
            CACHE_PATH,
            "--ttl",
            THIRTY_DAYS.to_string().as_str(),
        ])
        .output()?;

    let regex = regex::Regex::new(r"Driver path: (?<path>[^\r\n]+)")?;

    regex
        .captures(String::from_utf8_lossy(&result.stdout).as_ref())
        .map(|caps| caps.name("path"))
        .map(|om| om.map(|m| m.as_str().to_string()))
        .ok_or(CreateDriverError("Driver path not found".to_string()))
}

async fn check_port_available(port: u16) -> LaunchResult<bool> {
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

        caps.insert_browser_option("args", to_value(args)?)?;
    }

    Ok(())
}

/// Run a driver on the given port and with the given capabilities
///
/// If the driver does not run within the timeout, an Error is returned
async fn run_driver_and_execute<F, Fut, R, E>(
    browsers: Browsers,
    port: u16,
    capabilities: impl Into<Capabilities>,
    timeout_in_seconds: u16,
    handler: F,
) -> LaunchResult<R>
where
    F: Fn(WebDriver) -> Fut,
    Fut: Future<Output = Result<R, E>> + Send + 'static,
    E: Error + Debug + Send + 'static,
{
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port_available = check_port_available(port).await?;

    if !port_available {
        return Err(CreateDriverError(format!(
            "Port {} is already in use",
            port
        )));
    }

    if let Some(driver_path) = create_driver(browsers)? {

        let command = tokio::spawn(async move {
            Command::new(driver_path)
                .arg(format!("--port={port}"))
                .spawn()
        });

        let caps = capabilities.into();

        let mut running = false;

        let mut driver: Option<WebDriver> = None;

        let mut retries = timeout_in_seconds * 2;

        while !running {
            match WebDriver::new(format!("http://localhost:{port}"), caps.clone()).await {
                Ok(d) => {
                    log::info!("🚀 Successfully connected to driver");
                    running = true;
                    driver = Some(d);
                }
                Err(_) => {
                    retries -= 1;

                    if retries == 0 {
                        break;
                    }
                    log::info!("⏳ Waiting for driver to start ...");
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }

        match driver {
            None => Err(CreateDriverError(format!(
                "Failed to run the driver after {} secondes",
                timeout_in_seconds
            ))),
            Some(driver) => match handler(driver.clone()).await {
                Ok(r) => {
                    command.await.unwrap()?.kill()?;
                    Ok(r)
                }
                Err(e) => {
                    log::error!("Error while executing the driver => {}", e);
                    command.await.unwrap()?.kill()?;
                    driver.quit().await?;
                    Err(DriverLaunchError::DriverExecError(e.to_string()))
                }
            },
        }
    } else {
        Err(DriverLaunchError::RunDriverError(
            "Driver path not found".to_string(),
        ))
    }
}

/// Run a chrome driver
///
/// ```no_run
/// use driver_launcher::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     run_chrome_driver(53442, vec![("--headless", None)], 5, |driver| async move {
///         driver.goto("https://www.google.com").await?;
///         driver.quit().await?;
///         Ok::<(), DriverLaunchError>(())
///     })
///     .await
///     .unwrap();
/// }
/// ```
pub async fn run_chrome_driver<F, Fut, R, E>(
    port: u16,
    capabilities: Vec<(&str, Option<&str>)>,
    timeout_in_seconds: u16,
    handler: F,
) -> LaunchResult<R>
where
    F: Fn(WebDriver) -> Fut,
    Fut: Future<Output = Result<R, E>> + Send + 'static,
    E: Error + Debug + Send + 'static,
{
    let mut caps = DesiredCapabilities::chrome();

    add_args(capabilities, &mut caps)?;

    run_driver_and_execute(Browsers::CHROME, port, caps, timeout_in_seconds, handler).await
}

/// Run a firefox driver
///
/// ```
/// use driver_launcher::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     run_firefox_driver(53442, vec![("--headless", None)], 5, |driver| async move {
///         driver.goto("https://www.google.com").await?;
///         driver.quit().await?;
///         Ok::<(), DriverLaunchError>(())
///     })
///     .await
///     .unwrap();
/// }
/// ```
pub async fn run_firefox_driver<F, Fut, R, E>(
    port: u16,
    capabilities: Vec<(&str, Option<&str>)>,
    timeout_in_seconds: u16,
    handler: F,
) -> LaunchResult<R>
where
    F: Fn(WebDriver) -> Fut,
    Fut: Future<Output = Result<R, E>> + Send + 'static,
    E: Error + Debug + Send + 'static,
{
    let mut caps = DesiredCapabilities::firefox();

    add_args(capabilities, &mut caps)?;

    run_driver_and_execute(Browsers::FIREFOX, port, caps, timeout_in_seconds, handler).await
}

/// Run an edge driver
///
/// ```
/// use driver_launcher::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     run_edge_driver(53443, vec![("--headless", None)], 5, |driver| async move {
///         driver.goto("https://www.google.com").await?;
///         driver.quit().await?;
///         Ok::<(), DriverLaunchError>(())
///     })
///     .await
///     .unwrap();
/// }
/// ```
pub async fn run_edge_driver<F, Fut, R, E>(
    port: u16,
    capabilities: Vec<(&str, Option<&str>)>,
    timeout_in_seconds: u16,
    handler: F,
) -> LaunchResult<R>
where
    F: Fn(WebDriver) -> Fut,
    Fut: Future<Output = Result<R, E>> + Send + 'static,
    E: Error + Debug + Send + 'static,
{
    let mut caps = DesiredCapabilities::edge();

    add_args(capabilities, &mut caps)?;

    run_driver_and_execute(Browsers::EDGE, port, caps, timeout_in_seconds, handler).await
}

/// Run a safari driver
///
/// ```
/// use driver_launcher::prelude::*;
///
/// #[tokio::main]
/// async fn main() {
///     run_safari_driver(53444, vec![], 5, |driver| async move {
///         driver.goto("https://www.apple.com").await?;
///         driver.quit().await?;
///         Ok::<(), DriverLaunchError>(())
///     })
///     .await
///     .unwrap();
/// }
/// ```
pub async fn run_safari_driver<F, Fut, R, E>(
    port: u16,
    capabilities: Vec<(&str, Option<&str>)>,
    timeout_in_seconds: u16,
    handler: F,
) -> LaunchResult<R>
where
    F: Fn(WebDriver) -> Fut,
    Fut: Future<Output = Result<R, E>> + Send + 'static,
    E: Error + Debug + Send + 'static,
{
    let caps = DesiredCapabilities::safari();

    // SafariCapabilities does not implement BrowserCapabilitiesHelper; custom
    // browser args are not supported for Safari via the add_args mechanism.
    let _ = capabilities;

    run_driver_and_execute(Browsers::SAFARI, port, caps, timeout_in_seconds, handler).await
}



pub mod prelude {
    pub use crate::errors::DriverLaunchError;
    pub use crate::run_chrome_driver;
    pub use crate::run_edge_driver;
    pub use crate::run_firefox_driver;
    pub use crate::run_safari_driver;
}