pub const CACHE_PATH: &str = "selenium-cache";
pub const SELENIUM_MANAGER_PATH: &str = "selenium-manager";
pub const THIRTY_DAYS: u64 = 3600 * 24 * 30;
pub static SELENIUM_DATA: &'static [u8] = include_bytes!("../selenium-manager");