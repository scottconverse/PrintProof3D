// Printer Adapter Factory
use crate::{
    bambu::BambuAdapter, moonraker::MoonrakerAdapter, octoprint::OctoPrintAdapter,
    prusalink::PrusaLinkAdapter, rrf::RrfAdapter, serial::MarlinSerialAdapter, AdapterError,
    PrinterAdapter,
};
use printproof3d_core::{connection::PrinterConnectionConfig, PrinterProfile, ProtocolFamily};

pub struct PrinterAdapterFactory;

impl PrinterAdapterFactory {
    /// Builds a concrete implementation of PrinterAdapter from profile and connection config.
    pub fn build(
        profile: &PrinterProfile,
        config: &PrinterConnectionConfig,
    ) -> Result<Box<dyn PrinterAdapter>, AdapterError> {
        // Run config validations first
        config.validate().map_err(AdapterError::ConnectionFailed)?;

        match config.protocol_family {
            ProtocolFamily::BambuMqtt => {
                Ok(Box::new(BambuAdapter::new(profile.clone(), config.clone())))
            }
            ProtocolFamily::Klipper => Ok(Box::new(MoonrakerAdapter::new(
                profile.clone(),
                config.clone(),
            ))),
            ProtocolFamily::OctoPrint => Ok(Box::new(OctoPrintAdapter::new(
                profile.clone(),
                config.clone(),
            ))),
            ProtocolFamily::PrusaLink => Ok(Box::new(PrusaLinkAdapter::new(
                profile.clone(),
                config.clone(),
            ))),
            ProtocolFamily::RepRapFirmware => {
                Ok(Box::new(RrfAdapter::new(profile.clone(), config.clone())))
            }
            ProtocolFamily::MarlinSerial => Ok(Box::new(MarlinSerialAdapter::new(
                profile.clone(),
                config.clone(),
            ))),
            _ => Err(AdapterError::ConnectionFailed(format!(
                "Unsupported protocol family: {:?}",
                config.protocol_family
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printproof3d_core::{
        connection::{AuthType, ConnectionMode, DispatchPolicy},
        BedShape, BuildVolume, FirmwareFlavor,
    };

    fn dummy_profile() -> PrinterProfile {
        PrinterProfile {
            manufacturer: "Prusa".to_string(),
            model: "MK4".to_string(),
            protocol_family: ProtocolFamily::PrusaLink,
            build_volume: BuildVolume::Rectangular {
                x: 250.0,
                y: 210.0,
                z: 220.0,
            },
            bed_shape: BedShape::Rectangular,
            nozzle_diameters: vec![0.4],
            default_nozzle_diameter: 0.4,
            min_layer_height: 0.05,
            max_layer_height: 0.30,
            max_hotend_temp: 300.0,
            max_bed_temp: 120.0,
            has_enclosure: false,
            supports_mmu: false,
            firmware_flavor: FirmwareFlavor::Prusa,
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

    #[test]
    fn test_factory_builds_bambu() {
        let profile = dummy_profile();
        let config = PrinterConnectionConfig {
            name: "Bambu Simulator".to_string(),
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::BambuMqtt,
            base_url: Some("127.0.0.1".to_string()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
        };

        let adapter = PrinterAdapterFactory::build(&profile, &config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_factory_invalid_config_fails() {
        let profile = dummy_profile();
        let config = PrinterConnectionConfig {
            name: "".to_string(), // Invalid
            mode: ConnectionMode::Simulator,
            protocol_family: ProtocolFamily::BambuMqtt,
            base_url: Some("127.0.0.1".to_string()),
            serial_path: None,
            serial_baud_rate: None,
            auth_type: AuthType::None,
            api_key_env_var: None,
            username: None,
            password_env_var: None,
            tls_enabled: false,
            dispatch_policy: DispatchPolicy::AllowStart,
        };

        let adapter = PrinterAdapterFactory::build(&profile, &config);
        assert!(adapter.is_err());
    }
}
