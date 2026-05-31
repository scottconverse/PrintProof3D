// PrintProof3D Connection Config Model
use crate::ProtocolFamily;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Connection target mode indicating whether to route to a simulation host or physical hardware.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionMode {
    Simulator,
    Physical,
}

/// The set of standard simulation scenarios used to dry-run and QA printer state transitions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorScenario {
    Idle,
    AlreadyPrinting,
    Paused,
    Heating,
    UploadAccepted,
    UploadRejected,
    BadCredentials,
    OfflineOrConnectionRefused,
    TimeoutOrSlowResponse,
    MalformedTelemetry,
    StorageFull,
    UnsupportedFileType,
    EmergencyStopAccepted,
    EmergencyStopRejected,
}

/// Authentication protocol type utilized by the target adapter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    None,
    ApiKey,
    Digest,
    Password,
}

/// Dispatch rules stating whether execution is allowed or restricted to staging/dry-runs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DispatchPolicy {
    DryRunOnly,
    UploadOnly,
    AllowStart,
}

/// Structure detailing connection parameters, keys, endpoints, and policies.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PrinterConnectionConfig {
    /// Human readable target target label.
    pub name: String,
    /// Connection target mode (simulator or physical).
    pub mode: ConnectionMode,
    /// Network protocol family.
    pub protocol_family: ProtocolFamily,
    /// Base URL or network IP address.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Serial port device endpoint path.
    #[serde(default)]
    pub serial_path: Option<String>,
    /// Serial connection baud rate.
    #[serde(default)]
    pub serial_baud_rate: Option<u32>,
    /// Authentication protocol selection.
    #[serde(default = "default_auth_type")]
    pub auth_type: AuthType,
    /// Environment variable storing the API token/secret.
    #[serde(default)]
    pub api_key_env_var: Option<String>,
    /// Username for credential verification.
    #[serde(default)]
    pub username: Option<String>,
    /// Environment variable storing the client password.
    #[serde(default)]
    pub password_env_var: Option<String>,
    /// Secure socket TLS state.
    #[serde(default)]
    pub tls_enabled: bool,
    /// Pre-flight print execution policy.
    #[serde(default = "default_dispatch_policy")]
    pub dispatch_policy: DispatchPolicy,
    /// Optional simulator scenario.
    #[serde(default)]
    pub simulator_scenario: Option<SimulatorScenario>,
}

fn default_auth_type() -> AuthType {
    AuthType::None
}

fn default_dispatch_policy() -> DispatchPolicy {
    DispatchPolicy::DryRunOnly
}

impl PrinterConnectionConfig {
    /// Validates the configuration model invariants, returning clear, actionable instructions.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err(
                "Connection name cannot be empty. Please specify a descriptive target name."
                    .to_string(),
            );
        }

        let is_blank = |opt: &Option<String>| {
            opt.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
        };

        // Physical validation invariants
        if self.mode == ConnectionMode::Physical {
            if self.protocol_family == ProtocolFamily::Unknown {
                return Err("Validation Error: Physical connections cannot use protocol family 'unknown'.".to_string());
            }

            if self.protocol_family == ProtocolFamily::MarlinSerial {
                if is_blank(&self.serial_path) {
                    return Err("Validation Error: For physical 'marlin_serial' connections, 'serial_path' must be specified (e.g., 'COM3' or '/dev/ttyUSB0').".to_string());
                }
                if let Some(baud) = self.serial_baud_rate {
                    if baud == 0 {
                        return Err("Validation Error: Baud rate must be greater than 0.".to_string());
                    }
                }
            } else {
                if is_blank(&self.base_url) {
                    return Err(format!(
                        "Validation Error: For physical network target '{}' (protocol: {:?}), 'base_url' must be specified.",
                        self.name, self.protocol_family
                    ));
                }
            }
        }

        // Authentication validation invariants
        match self.auth_type {
            AuthType::ApiKey => {
                if is_blank(&self.api_key_env_var) {
                    return Err("Validation Error: Auth type 'api_key' requires setting the 'api_key_env_var' field with the environment variable name containing the secret key.".to_string());
                }
            }
            AuthType::Password | AuthType::Digest => {
                if is_blank(&self.username) {
                    return Err(
                        "Validation Error: Auth type requires a non-empty 'username' value."
                            .to_string(),
                    );
                }
                if is_blank(&self.password_env_var) {
                    return Err("Validation Error: Auth type requires setting the 'password_env_var' field with the environment variable name containing the password.".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let config = PrinterConnectionConfig {
            name: "Bambu Sim".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::BambuMqtt,
            base_url: Some("http://localhost".to_string()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_marlin_physical() {
        let config = PrinterConnectionConfig {
            name: "Marlin Physical".to_string(),
            mode: ConnectionMode::Physical,
            protocol_family: ProtocolFamily::MarlinSerial,
            base_url: None,
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let res = config.validate();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("serial_path"));
    }

    #[test]
    fn test_invalid_network_physical() {
        let config = PrinterConnectionConfig {
            name: "Klipper Physical".to_string(),
            mode: ConnectionMode::Physical,
            protocol_family: ProtocolFamily::Klipper,
            base_url: None,
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let res = config.validate();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("base_url"));
    }

    #[test]
    fn test_invalid_api_key_auth() {
        let config = PrinterConnectionConfig {
            name: "OctoPrint".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::OctoPrint,
            base_url: None,
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::ApiKey,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        let res = config.validate();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("api_key_env_var"));
    }

    #[test]
    fn test_digest_auth_requires_credentials() {
        let mut config = PrinterConnectionConfig {
            name: "PrusaLink Digest".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::PrusaLink,
            base_url: Some("http://localhost".to_string()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::Digest,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        assert!(config.validate().is_err()); // empty username/password

        config.username = Some("   ".to_string());
        config.password_env_var = Some("PRUSALINK_PASSWORD".to_string());
        assert!(config.validate().is_err()); // whitespace username

        config.username = Some("admin".to_string());
        config.password_env_var = Some("".to_string());
        assert!(config.validate().is_err()); // empty password env var

        config.password_env_var = Some("PRUSALINK_PASSWORD".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_baud_rate_zero_invalid() {
        let config = PrinterConnectionConfig {
            name: "Marlin Physical Zero Baud".to_string(),
            mode: ConnectionMode::Physical,
            protocol_family: ProtocolFamily::MarlinSerial,
            base_url: None,
            serial_path: Some("COM3".to_string()),
            serial_baud_rate: Some(0),
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
            simulator_scenario: None,
        };
        assert!(config.validate().is_err());
    }
}
