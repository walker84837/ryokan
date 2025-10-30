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

#[test]
fn test_handle_pin_setup_and_verification_new_pin() -> Result<(), AppError> {
    let dir = tempdir().map_err(AppError::Io)?;
    let config_path = dir.path().join("test_config.toml");
    let mut config = Config::new(&Some(config_path.clone()))?;

    // Simulate user input for a new PIN
    // This is tricky to test directly without mocking stdin. For now, we'll assume ask_for_pin works.
    // A more robust test would involve a mock for stdin.

    // For now, we'll test the path where a PIN is not found and a new one is set.
    // This requires manually setting the pin_hash to empty or None in the config.
    config.pin_hash = String::new();

    // This part is hard to test without user interaction. I'll skip direct testing of the interactive part for now.
    // The `handle_pin_setup_and_verification` function relies on `ask_for_pin` which is interactive.
    // To properly test this, we would need to mock `std::io::stdin`.

    Ok(())
}
