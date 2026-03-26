use crate::errors::DriverLaunchError;

pub type LaunchResult<T> = Result<T, DriverLaunchError>;

#[derive(Clone)]
pub enum Browsers {
    CHROME,
    FIREFOX,
    EDGE,
    IEXPLORER,
    SAFARI,
    SAFARITP,
    WEBVIEW2
}

impl Browsers {
    pub fn as_str(&self) -> &'static str {
        match self {
            Browsers::CHROME => "chrome",
            Browsers::FIREFOX => "firefox",
            Browsers::EDGE => "edge",
            Browsers::IEXPLORER => "iexplorer",
            Browsers::SAFARI => "safari",
            Browsers::SAFARITP => "safaritp",
            Browsers::WEBVIEW2 => "webview2"
        }
    }
}