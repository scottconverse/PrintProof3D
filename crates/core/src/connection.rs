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
    pub base_url: Option<String>,
    /// Serial port device endpoint path.
    pub serial_path: Option<String>,
    /// Serial connection baud rate.
    pub serial_baud_rate: Option<u32>,
    /// Authentication protocol selection.
    pub auth_type: AuthType,
    /// Environment variable storing the API token/secret.
    pub api_key_env_var: Option<String>,
    /// Username for credential verification.
    pub username: Option<String>,
    /// Environment variable storing the client password.
    pub password_env_var: Option<String>,
    /// Secure socket TLS state.
    pub tls_enabled: bool,
    /// Pre-flight print execution policy.
    pub dispatch_policy: DispatchPolicy,
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

        // Physical validation invariants
        if self.mode == ConnectionMode::Physical {
            if self.protocol_family == ProtocolFamily::MarlinSerial {
                if self.serial_path.is_none() {
                    return Err("Validation Error: For physical 'marlin_serial' connections, 'serial_path' must be specified (e.g., 'COM3' or '/dev/ttyUSB0').".to_string());
                }
            } else {
                let need_base_url = !matches!(self.protocol_family, ProtocolFamily::Unknown);
                if need_base_url && self.base_url.is_none() {
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
                if self.api_key_env_var.is_none() {
                    return Err("Validation Error: Auth type 'api_key' requires setting the 'api_key_env_var' field with the environment variable name containing the secret key.".to_string());
                }
            }
            AuthType::Password => {
                if self.username.is_none() {
                    return Err(
                        "Validation Error: Auth type 'password' requires a 'username' value."
                            .to_string(),
                    );
                }
                if self.password_env_var.is_none() {
                    return Err("Validation Error: Auth type 'password' requires setting the 'password_env_var' field with the environment variable name containing the password.".to_string());
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
        };
        let res = config.validate();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("api_key_env_var"));
    }
}
