use wasmi::{Engine, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc};

// Re-export core types and serde_json so the macro has reliable references
pub use printproof3d_core::{IssueSeverity, ValidationIssue, ValidationReport, ValidationStatus};
pub use serde_json;

pub struct MyState {
    limits: StoreLimits,
}

/// Runtime loader and executor for WASM validation plugins.
pub struct PluginEngine {
    engine: Engine,
}

impl Default for PluginEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginEngine {
    /// Creates a new PluginEngine instance.
    pub fn new() -> Self {
        let mut config = wasmi::Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        Self { engine }
    }

    /// Compiles and instantiates the WASM byte slice, returning a LoadedPlugin.
    pub fn load_plugin(&self, wasm_bytes: &[u8]) -> Result<LoadedPlugin, String> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(16 * 1024 * 1024) // 16 MB limit
            .build();

        let mut store = Store::new(&self.engine, MyState { limits });
        store.limiter(|state| &mut state.limits);

        // Fuel limit of 50M instructions
        store
            .add_fuel(50_000_000)
            .map_err(|e| format!("Failed to set fuel: {:?}", e))?;

        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| format!("Failed to compile WASM module: {:?}", e))?;

        let mut linker = <Linker<MyState>>::new(&self.engine);
        let stub_describe = wasmi::Func::wrap(&mut store, |_: i32| {});
        let stub_throw = wasmi::Func::wrap(&mut store, |_: i32, _: i32| {});
        let _ = linker.define(
            "__wbindgen_placeholder__",
            "__wbindgen_describe",
            stub_describe,
        );
        let _ = linker.define("__wbindgen_placeholder__", "__wbindgen_throw", stub_throw);

        let stub_grow = wasmi::Func::wrap(&mut store, |x: i32| x);
        let stub_set_null = wasmi::Func::wrap(&mut store, |_: i32| {});
        let _ = linker.define(
            "__wbindgen_externref_xform__",
            "__wbindgen_externref_table_grow",
            stub_grow,
        );
        let _ = linker.define(
            "__wbindgen_externref_xform__",
            "__wbindgen_externref_table_set_null",
            stub_set_null,
        );

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("Failed to instantiate module: {:?}", e))?
            .start(&mut store)
            .map_err(|e| format!("Failed to start module: {:?}", e))?;

        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| "Module does not export 'memory'".to_string())?;

        let alloc_fn = instance
            .get_typed_func::<u32, u32>(&store, "alloc")
            .map_err(|e| format!("Module does not export 'alloc' function: {:?}", e))?;

        let dealloc_fn = instance
            .get_typed_func::<(u32, u32), ()>(&store, "dealloc")
            .map_err(|e| format!("Module does not export 'dealloc' function: {:?}", e))?;

        let validate_fn = instance
            .get_typed_func::<(u32, u32), u64>(&store, "validate")
            .map_err(|e| format!("Module does not export 'validate' function: {:?}", e))?;

        Ok(LoadedPlugin {
            store,
            memory,
            alloc_fn,
            dealloc_fn,
            validate_fn,
        })
    }
}

/// An instantiated WASM plugin ready for execution.
pub struct LoadedPlugin {
    store: Store<MyState>,
    memory: Memory,
    alloc_fn: TypedFunc<u32, u32>,
    dealloc_fn: TypedFunc<(u32, u32), ()>,
    validate_fn: TypedFunc<(u32, u32), u64>,
}

impl LoadedPlugin {
    /// Runs the plugin validation logic by passing a JSON string representation
    /// of the ValidationReport, and returns the modified JSON report.
    pub fn execute_validation(&mut self, report_json: &str) -> Result<String, String> {
        let input_bytes = report_json.as_bytes();
        let input_len = input_bytes.len() as u32;

        // 1. Allocate input buffer in WASM memory
        let input_ptr = self
            .alloc_fn
            .call(&mut self.store, input_len)
            .map_err(|e| format!("WASM alloc failed: {:?}", e))?;

        // 2. Write input JSON to WASM memory
        self.memory
            .write(&mut self.store, input_ptr as usize, input_bytes)
            .map_err(|e| format!("Failed to write to WASM memory: {:?}", e))?;

        // 3. Execute validation in the sandbox
        let result_u64 = self
            .validate_fn
            .call(&mut self.store, (input_ptr, input_len))
            .map_err(|e| format!("WASM execution trapped: {}", e))?;

        // 4. Clean up input buffer in WASM memory
        let _ = self
            .dealloc_fn
            .call(&mut self.store, (input_ptr, input_len));

        // 5. Decode output pointer and length
        let output_ptr = (result_u64 >> 32) as u32;
        let output_len = (result_u64 & 0xFFFF_FFFF) as u32;

        if output_len == 0 {
            return Err("WASM validation returned null pointer or zero length".to_string());
        }

        // The guest fully controls `output_len`. Its linear memory is capped at 16 MB, so any
        // larger length is necessarily invalid — reject it before allocating host memory rather
        // than letting a malicious/buggy plugin trigger a multi-gigabyte host allocation that only
        // fails afterwards at the bounds-checked read below.
        const MAX_PLUGIN_OUTPUT_LEN: u32 = 16 * 1024 * 1024;
        if output_len > MAX_PLUGIN_OUTPUT_LEN {
            return Err(format!(
                "WASM validation returned implausible output length {} (exceeds {} byte ceiling)",
                output_len, MAX_PLUGIN_OUTPUT_LEN
            ));
        }

        // 6. Read output JSON bytes from WASM memory
        let mut output_bytes = vec![0u8; output_len as usize];
        self.memory
            .read(&self.store, output_ptr as usize, &mut output_bytes)
            .map_err(|e| format!("Failed to read output from WASM memory: {:?}", e))?;

        // 7. Clean up output buffer in WASM memory
        let _ = self
            .dealloc_fn
            .call(&mut self.store, (output_ptr, output_len));

        // 8. Return deserialized JSON
        String::from_utf8(output_bytes)
            .map_err(|e| format!("Invalid UTF-8 returned from WASM: {:?}", e))
    }
}

/// Macro to easily export WASM plugin symbols from standard Rust validation functions.
#[macro_export]
macro_rules! export_validation_plugin {
    ($validate_fn:expr) => {
        #[no_mangle]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub extern "C" fn alloc(size: u32) -> *mut u8 {
            let layout = std::alloc::Layout::from_size_align(size as usize, 8).unwrap();
            unsafe { std::alloc::alloc(layout) }
        }

        #[no_mangle]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub extern "C" fn dealloc(ptr: *mut u8, size: u32) {
            let layout = std::alloc::Layout::from_size_align(size as usize, 8).unwrap();
            unsafe { std::alloc::dealloc(ptr, layout) }
        }

        #[no_mangle]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        pub extern "C" fn validate(ptr: *mut u8, len: u32) -> u64 {
            let input_bytes = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
            let input_str = match std::str::from_utf8(input_bytes) {
                Ok(s) => s,
                Err(_) => return 0,
            };

            let mut report: $crate::ValidationReport = match $crate::serde_json::from_str(input_str)
            {
                Ok(r) => r,
                Err(_) => return 0,
            };

            let validate_impl: fn(&mut $crate::ValidationReport) = $validate_fn;
            validate_impl(&mut report);

            let output_str = match $crate::serde_json::to_string(&report) {
                Ok(s) => s,
                Err(_) => return 0,
            };

            let output_bytes = output_str.into_bytes();
            let output_len = output_bytes.len() as u32;
            let output_ptr = alloc(output_len);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    output_bytes.as_ptr(),
                    output_ptr,
                    output_len as usize,
                );
            }

            ((output_ptr as u64) << 32) | (output_len as u64)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_plugin_memory_exchange() {
        let wat = r#"
            (module
              (memory (export "memory") 1)
              (func (export "alloc") (param $size i32) (result i32)
                i32.const 1024
              )
              (func (export "dealloc") (param $ptr i32) (param $size i32)
              )
              (func (export "validate") (param $ptr i32) (param $len i32) (result i64)
                (local $i i32)
                (local.set $i (i32.const 0))
                (block
                  (loop
                    (br_if 1 (i32.eq (local.get $i) (local.get $len)))
                    (i32.add (i32.const 2048) (local.get $i))
                    (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
                    i32.store8
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))
                    br 0
                  )
                )
                (i64.or
                  (i64.shl (i64.extend_i32_u (i32.const 2048)) (i64.const 32))
                  (i64.extend_i32_u (local.get $len))
                )
              )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let engine = PluginEngine::new();
        let mut plugin = engine.load_plugin(&wasm_bytes).unwrap();

        let test_json = r#"{"test":"hello"}"#;
        let result = plugin.execute_validation(test_json).unwrap();
        assert_eq!(result, test_json);
    }

    #[test]
    fn test_wasm_plugin_infinite_loop_trap() {
        let wat = r#"
            (module
              (memory (export "memory") 1)
              (func (export "alloc") (param $size i32) (result i32)
                i32.const 1024
              )
              (func (export "dealloc") (param $ptr i32) (param $size i32)
              )
              (func (export "validate") (param $ptr i32) (param $len i32) (result i64)
                (loop
                  br 0
                )
                i64.const 0
              )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let engine = PluginEngine::new();
        let mut plugin = engine.load_plugin(&wasm_bytes).unwrap();
        let res = plugin.execute_validation("{}");
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(
            err_msg.contains("trapped") || err_msg.contains("fuel"),
            "Expected fuel trap error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_wasm_plugin_oom_trap() {
        let wat = r#"
            (module
              (memory (export "memory") 1)
              (func (export "alloc") (param $size i32) (result i32)
                i32.const 1024
              )
              (func (export "dealloc") (param $ptr i32) (param $size i32)
              )
              (func (export "validate") (param $ptr i32) (param $len i32) (result i64)
                ;; Grow memory by 300 pages (approx 19.2MB, which exceeds the 16MB ceiling)
                (memory.grow (i32.const 300))
                i32.const -1
                i32.eq
                (if
                  (then
                    unreachable
                  )
                )
                i64.const 0
              )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let engine = PluginEngine::new();
        let mut plugin = engine.load_plugin(&wasm_bytes).unwrap();
        let res = plugin.execute_validation("{}");
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(
            err_msg.contains("trapped") || err_msg.contains("limit") || err_msg.contains("memory"),
            "Expected memory growth trap error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_export_validation_macro() {
        mod mock_plugin_module {
            use super::super::*;

            fn my_mock_validate(report: &mut ValidationReport) {
                report.status = ValidationStatus::Fail;
            }

            export_validation_plugin!(my_mock_validate);
        }

        let report = ValidationReport {
            status: ValidationStatus::Pass,
            target_printer_profile: "test".to_string(),
            target_material_profile: "test".to_string(),
            model: printproof3d_core::ModelMetadata {
                file_name: "test.stl".to_string(),
                units: "mm".to_string(),
                bounding_box: printproof3d_core::BoundingBox {
                    min_x: 0.0,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 1.0,
                    max_y: 1.0,
                    max_z: 1.0,
                },
            },
            issues: vec![],
            confidence_level: "high".to_string(),
            sliced_settings_assumed: None,
        };

        let report_str = serde_json::to_string(&report).unwrap();
        let len = report_str.len() as u32;

        let ptr = mock_plugin_module::alloc(len);
        assert!(!ptr.is_null());

        unsafe {
            std::ptr::copy_nonoverlapping(report_str.as_ptr(), ptr, len as usize);
        }

        let res = mock_plugin_module::validate(ptr, len);
        assert_ne!(res, 0);

        if cfg!(target_pointer_width = "32") {
            let out_ptr = (res >> 32) as *mut u8;
            let out_len = (res & 0xFFFFFFFF) as u32;
            assert!(!out_ptr.is_null());
            assert!(out_len > 0);

            let out_bytes = unsafe { std::slice::from_raw_parts(out_ptr, out_len as usize) };
            let out_str = std::str::from_utf8(out_bytes).unwrap();
            let parsed_report: ValidationReport = serde_json::from_str(out_str).unwrap();
            assert_eq!(parsed_report.status, ValidationStatus::Fail);

            mock_plugin_module::dealloc(out_ptr, out_len);
        } else {
            let out_len = (res & 0xFFFFFFFF) as u32;
            assert!(out_len > 0);
        }

        mock_plugin_module::dealloc(ptr, len);
    }

    #[test]
    fn test_wasm_macro_compile_and_run() {
        use std::path::Path;
        use std::process::Command;

        // This test compiles a real plugin to `wasm32-unknown-unknown`. That target is optional and
        // is NOT present on a stock toolchain, so skip (rather than fail) when it is absent —
        // `cargo test --workspace` must stay green out of the box. CI installs the target explicitly
        // (see .github/workflows/ci.yml), so coverage is preserved there.
        let wasm_target_installed = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32-unknown-unknown"))
            .unwrap_or(false);
        if !wasm_target_installed {
            eprintln!(
                "skipping test_wasm_macro_compile_and_run: wasm32-unknown-unknown not installed \
                 (run `rustup target add wasm32-unknown-unknown` to enable this test)"
            );
            return;
        }

        // Run cargo build for example-plugin targeting wasm32
        let status = Command::new("cargo")
            .args([
                "build",
                "--package",
                "example-plugin",
                "--target",
                "wasm32-unknown-unknown",
            ])
            .status()
            .expect("Failed to execute cargo build command for example-plugin");

        assert!(
            status.success(),
            "Failed to compile example-plugin to wasm32!"
        );

        // Find the compiled wasm file
        // Workspace root is 2 directories up from crates/plugins/
        let wasm_path = Path::new("../../target/wasm32-unknown-unknown/debug/example_plugin.wasm");

        // Let's verify that the wasm file exists
        assert!(
            wasm_path.exists(),
            "Compiled WASM plugin file not found at {:?}",
            wasm_path
        );

        // Load the plugin engine
        let engine = PluginEngine::new();

        let report_warning = ValidationReport {
            status: ValidationStatus::Pass,
            target_printer_profile: "test".to_string(),
            target_material_profile: "test".to_string(),
            model: printproof3d_core::ModelMetadata {
                file_name: "test.stl".to_string(),
                units: "mm".to_string(),
                bounding_box: printproof3d_core::BoundingBox {
                    min_x: 0.0,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 2.0, // volume = 8.0 (> 5.0, < 1000.0) -> Warning
                    max_y: 2.0,
                    max_z: 2.0,
                },
            },
            issues: vec![],
            confidence_level: "high".to_string(),
            sliced_settings_assumed: None,
        };

        let report_critical = ValidationReport {
            status: ValidationStatus::Pass,
            target_printer_profile: "test".to_string(),
            target_material_profile: "test".to_string(),
            model: printproof3d_core::ModelMetadata {
                file_name: "test.stl".to_string(),
                units: "mm".to_string(),
                bounding_box: printproof3d_core::BoundingBox {
                    min_x: 0.0,
                    min_y: 0.0,
                    min_z: 0.0,
                    max_x: 1.0, // volume = 1.0 (< 5.0) -> Critical
                    max_y: 1.0,
                    max_z: 1.0,
                },
            },
            issues: vec![],
            confidence_level: "high".to_string(),
            sliced_settings_assumed: None,
        };

        let wasm_bytes = std::fs::read(wasm_path).expect("Failed to read compiled wasm file");
        let mut loaded = engine
            .load_plugin(&wasm_bytes)
            .expect("Failed to load plugin");

        // 1. Check warning case
        let input_warning_json = serde_json::to_string(&report_warning).unwrap();
        let output_warning_json = loaded
            .execute_validation(&input_warning_json)
            .expect("Failed to execute validation");
        let mut final_warning: ValidationReport =
            serde_json::from_str(&output_warning_json).unwrap();
        final_warning.enforce_invariants();
        assert_eq!(final_warning.status, ValidationStatus::Warning);
        assert!(!final_warning.issues.is_empty());
        assert_eq!(final_warning.issues[0].id, "VOLUME_TOO_SMALL");

        // 2. Check critical case (invariant revalidation regression test)
        let input_critical_json = serde_json::to_string(&report_critical).unwrap();
        let output_critical_json = loaded
            .execute_validation(&input_critical_json)
            .expect("Failed to execute validation");
        let mut final_critical: ValidationReport =
            serde_json::from_str(&output_critical_json).unwrap();

        // Before enforcing, status is still Pass
        assert_eq!(final_critical.status, ValidationStatus::Pass);
        assert!(!final_critical.issues.is_empty());
        assert_eq!(final_critical.issues[0].id, "VOLUME_CRITICAL");

        // After enforcing, status must be Fail
        final_critical.enforce_invariants();
        assert_eq!(final_critical.status, ValidationStatus::Fail);
    }
}
