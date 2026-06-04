#[cfg(test)]
mod tests {
    use crate::factory::PrinterAdapterFactory;
    use crate::AdapterError;
    use printproof3d_core::{
        connection::{AuthType, ConnectionMode, DispatchPolicy, PrinterConnectionConfig},
        BedShape, BuildVolume, FirmwareFlavor, PrinterProfile, ProtocolFamily,
    };
    use std::path::Path;

    fn dummy_profile(protocol: ProtocolFamily) -> PrinterProfile {
        PrinterProfile {
            manufacturer: "Test".to_string(),
            model: "Generic".to_string(),
            protocol_family: protocol,
            build_volume: BuildVolume::Rectangular {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
            bed_shape: BedShape::Rectangular,
            nozzle_diameters: vec![0.4],
            default_nozzle_diameter: 0.4,
            min_layer_height: 0.05,
            max_layer_height: 0.3,
            max_hotend_temp: 300.0,
            max_bed_temp: 120.0,
            has_enclosure: false,
            supports_mmu: false,
            firmware_flavor: FirmwareFlavor::Marlin,
            supported_file_types: vec!["gcode".to_string()],
            supports_direct_upload: true,
            supports_pause_resume: true,
            supports_cancel: true,
            supports_job_progress: true,
            supports_webcam: false,
            supports_chamber_temp: false,
            known_quirks: vec![],
            unsafe_commands: vec![],
            filename_restrictions: None,
        }
    }

    fn dummy_config(protocol: ProtocolFamily, policy: DispatchPolicy) -> PrinterConnectionConfig {
        let is_serial = protocol == ProtocolFamily::MarlinSerial;
        PrinterConnectionConfig {
            name: "Test Connection".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: protocol,
            base_url: Some("http://127.0.0.1".to_string()),
            serial_path: if is_serial {
                Some("COM3".to_string())
            } else {
                None
            },
            serial_baud_rate: if is_serial { Some(115200) } else { None },
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: policy,
            simulator_scenario: None,
        }
    }

    async fn test_policy_limits(protocol: ProtocolFamily) {
        // 1. Test DryRunOnly policy
        let profile = dummy_profile(protocol.clone());
        let config = dummy_config(protocol.clone(), DispatchPolicy::DryRunOnly);
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();

        let dummy_path = Path::new("dummy.gcode");
        let upload_res = adapter.upload_file(dummy_path, "dummy.gcode").await;
        assert!(
            matches!(upload_res, Err(AdapterError::UploadFailed(_))),
            "Expected upload failure under DryRunOnly for {:?}",
            protocol
        );

        let start_res = adapter.start_job("dummy_id").await;
        assert!(
            matches!(start_res, Err(AdapterError::CommandFailed(_))),
            "Expected start job failure under DryRunOnly for {:?}",
            protocol
        );

        let pause_res = adapter.pause_job().await;
        assert!(
            matches!(pause_res, Err(AdapterError::CommandFailed(_))),
            "Expected pause failure under DryRunOnly for {:?}",
            protocol
        );

        // 2. Test UploadOnly policy
        let config_upload = dummy_config(protocol.clone(), DispatchPolicy::UploadOnly);
        let adapter_upload = PrinterAdapterFactory::build(&profile, &config_upload).unwrap();

        let start_res = adapter_upload.start_job("dummy_id").await;
        assert!(
            matches!(start_res, Err(AdapterError::CommandFailed(_))),
            "Expected start job failure under UploadOnly for {:?}",
            protocol
        );

        let emergency_res = adapter_upload.emergency_stop().await;
        assert!(
            matches!(emergency_res, Err(AdapterError::CommandFailed(_))),
            "Expected emergency stop failure under UploadOnly for {:?}",
            protocol
        );
    }

    #[tokio::test]
    async fn test_dispatch_policy_limits_all_adapters() {
        test_policy_limits(ProtocolFamily::BambuMqtt).await;
        test_policy_limits(ProtocolFamily::Klipper).await;
        test_policy_limits(ProtocolFamily::OctoPrint).await;
        test_policy_limits(ProtocolFamily::PrusaLink).await;
        test_policy_limits(ProtocolFamily::RepRapFirmware).await;
        test_policy_limits(ProtocolFamily::MarlinSerial).await;
    }

    #[tokio::test]
    async fn test_moonraker_credentials_hardening() {
        // ApiKey auth configured but no environment variable name specified in config
        let profile = dummy_profile(ProtocolFamily::Klipper);
        let mut config = dummy_config(ProtocolFamily::Klipper, DispatchPolicy::AllowStart);
        config.auth_type = AuthType::ApiKey;
        config.api_key_env_var = None;

        let adapter_res = PrinterAdapterFactory::build(&profile, &config);
        assert!(
            matches!(adapter_res, Err(AdapterError::ValidationError(_))),
            "Expected ValidationError when api_key_env_var is None under ApiKey auth"
        );

        // Env var configured but not set in environment
        config.api_key_env_var = Some("TEST_MOONRAKER_API_KEY_UNSET".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when env var is unset"
        );

        // Env var set but empty
        std::env::set_var("TEST_MOONRAKER_API_KEY_EMPTY", "  ");
        config.api_key_env_var = Some("TEST_MOONRAKER_API_KEY_EMPTY".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when env var is empty whitespace"
        );
    }

    #[tokio::test]
    async fn test_octoprint_credentials_hardening() {
        let profile = dummy_profile(ProtocolFamily::OctoPrint);
        let mut config = dummy_config(ProtocolFamily::OctoPrint, DispatchPolicy::AllowStart);
        config.auth_type = AuthType::ApiKey;
        config.api_key_env_var = None;

        let adapter_res = PrinterAdapterFactory::build(&profile, &config);
        assert!(
            matches!(adapter_res, Err(AdapterError::ValidationError(_))),
            "Expected ValidationError when api_key_env_var is None under ApiKey auth"
        );

        config.api_key_env_var = Some("TEST_OCTOPRINT_API_KEY_UNSET".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when env var is unset"
        );

        std::env::set_var("TEST_OCTOPRINT_API_KEY_EMPTY", "  ");
        config.api_key_env_var = Some("TEST_OCTOPRINT_API_KEY_EMPTY".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when env var is empty"
        );
    }

    #[tokio::test]
    async fn test_prusalink_credentials_hardening() {
        let profile = dummy_profile(ProtocolFamily::PrusaLink);
        let mut config = dummy_config(ProtocolFamily::PrusaLink, DispatchPolicy::AllowStart);
        config.auth_type = AuthType::Digest;
        config.username = None;
        config.password_env_var = None;

        let adapter_res = PrinterAdapterFactory::build(&profile, &config);
        assert!(
            matches!(adapter_res, Err(AdapterError::ValidationError(_))),
            "Expected ValidationError when username is None under Digest auth"
        );

        config.username = Some("".to_string());
        let adapter_res = PrinterAdapterFactory::build(&profile, &config);
        assert!(
            matches!(adapter_res, Err(AdapterError::ValidationError(_))),
            "Expected ValidationError when username is empty under Digest auth"
        );

        config.username = Some("maker".to_string());
        config.password_env_var = Some("TEST_PRUSALINK_PASSWORD_UNSET".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when password env var is unset"
        );

        std::env::set_var("TEST_PRUSALINK_PASSWORD_EMPTY", "");
        config.password_env_var = Some("TEST_PRUSALINK_PASSWORD_EMPTY".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when password env var is empty"
        );
    }

    #[tokio::test]
    async fn test_prusalink_credentials_hardening_default_branch() {
        let profile = dummy_profile(ProtocolFamily::PrusaLink);
        let mut config = dummy_config(ProtocolFamily::PrusaLink, DispatchPolicy::AllowStart);
        config.auth_type = AuthType::None;
        config.username = None;
        config.password_env_var = None;

        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when username is None under default auth branch"
        );

        config.username = Some("maker".to_string());
        config.password_env_var = None;
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when password_env_var is None under default auth branch"
        );

        config.password_env_var = Some("TEST_PRUSALINK_DEFAULT_PASSWORD_UNSET".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when password env var is unset under default auth branch"
        );

        std::env::set_var("TEST_PRUSALINK_DEFAULT_PASSWORD_EMPTY", "");
        config.password_env_var = Some("TEST_PRUSALINK_DEFAULT_PASSWORD_EMPTY".to_string());
        let adapter = PrinterAdapterFactory::build(&profile, &config).unwrap();
        let upload_res = adapter
            .upload_file(Path::new("test.gcode"), "test.gcode")
            .await;
        assert!(
            matches!(upload_res, Err(AdapterError::AuthenticationFailed(_))),
            "Expected Auth failure when password env var is empty under default auth branch"
        );
    }
}
