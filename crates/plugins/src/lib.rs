use wasmi::{Engine, Linker, Memory, Module, Store, TypedFunc};

// Re-export core types and serde_json so the macro has reliable references
pub use printproof3d_core::{IssueSeverity, ValidationIssue, ValidationReport, ValidationStatus};
pub use serde_json;

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
        let engine = Engine::default();
        Self { engine }
    }

    /// Compiles and instantiates the WASM byte slice, returning a LoadedPlugin.
    pub fn load_plugin(&self, wasm_bytes: &[u8]) -> Result<LoadedPlugin, String> {
        let mut store = Store::new(&self.engine, ());
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| format!("Failed to compile WASM module: {:?}", e))?;

        let mut linker = <Linker<()>>::new(&self.engine);
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
    store: Store<()>,
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
            .map_err(|e| format!("WASM validation failed: {:?}", e))?;

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
        pub extern "C" fn alloc(size: u32) -> *mut u8 {
            let layout = std::alloc::Layout::from_size_align(size as usize, 8).unwrap();
            unsafe { std::alloc::alloc(layout) }
        }

        #[no_mangle]
        pub extern "C" fn dealloc(ptr: *mut u8, size: u32) {
            let layout = std::alloc::Layout::from_size_align(size as usize, 8).unwrap();
            unsafe { std::alloc::dealloc(ptr, layout) }
        }

        #[no_mangle]
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
}
