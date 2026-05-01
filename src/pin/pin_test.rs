#![cfg(test)]

use super::*;
use crate::config::Config;
use tempfile::tempdir;

#[test]
fn test_store_and_verify_pin() -> Result<(), AppError> {
    let dir = tempdir().map_err(AppError::Io)?;
    let config_path = dir.path().join("test_config.toml");
    let mut config = Config::new(&Some(config_path.clone()))?;

    let test_pin = "123456";
    store_pin(&mut config, test_pin)?;

    assert!(verify_pin(&config, test_pin)?);
    assert!(!verify_pin(&config, "654321")?);

    Ok(())
}
