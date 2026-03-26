
mod types;

mod constants;
mod utils;
mod utils_tests;

use crate::types::{Browsers, LaunchResult};

use std::error::Error;
use std::fmt::Debug;
use std::process::Command;
use anyhow::anyhow;
use thirtyfour::{Capabilities, DesiredCapabilities, WebDriver};
use crate::utils::{add_args, check_port_available, create_driver};

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
        return Err(anyhow!(format!(
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
            None => Err(anyhow!(format!(
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
                    Err(anyhow!(e.to_string()))
                }
            },
        }
    } else {
        Err(anyhow!("Driver path not found".to_string()))
    }
}

/// Run a chrome driver
///
/// ```no_run
/// use svird::prelude::*;
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
/// ```no_run
/// use svird::prelude::*;
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
/// ```no_run
/// use svird::prelude::*;
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
/// ```no_run
/// use svird::prelude::*;
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
    pub use crate::run_chrome_driver;
    pub use crate::run_edge_driver;
    pub use crate::run_firefox_driver;
    pub use crate::run_safari_driver;
}