//! WASM plugin runtime for processing frame buffers.
//!
//! Plugins implement a simple interface:
//! - `process(input_ptr: i32, output_ptr: i32, width: i32, height: i32, params_ptr: i32, params_len: i32)`

#[cfg(feature = "plugins")]
use std::collections::HashMap;
#[cfg(feature = "plugins")]
use std::path::Path;
#[cfg(feature = "plugins")]
use std::sync::Mutex;

#[cfg(feature = "plugins")]
use tracing::{debug, info, warn};
#[cfg(feature = "plugins")]
use wasmtime::*;

#[cfg(feature = "plugins")]
use crate::error::MediaPipelineError;

#[cfg(feature = "plugins")]
static PLUGIN_CACHE: std::sync::LazyLock<Mutex<HashMap<String, Vec<u8>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(feature = "plugins")]
pub struct PluginInstance {
    engine: Engine,
    module: Module,
}

#[cfg(feature = "plugins")]
impl PluginInstance {
    /// Load a WASM plugin from a file.
    pub fn load(path: &Path) -> Result<Self, MediaPipelineError> {
        let engine = Engine::default();
        let wasm_bytes = std::fs::read(path)
            .map_err(|e| MediaPipelineError::Export(format!("failed to read plugin: {e}")))?;
        let module = Module::new(&engine, &wasm_bytes)
            .map_err(|e| MediaPipelineError::Export(format!("failed to compile plugin: {e}")))?;
        Ok(Self { engine, module })
    }

    /// Load a WASM plugin from bytes.
    pub fn from_bytes(wasm: &[u8]) -> Result<Self, MediaPipelineError> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm)
            .map_err(|e| MediaPipelineError::Export(format!("failed to compile plugin: {e}")))?;
        Ok(Self { engine, module })
    }

    /// Process a frame buffer through the plugin.
    ///
    /// `input` and `output` are RGBA pixel buffers (4 bytes per pixel).
    /// `params` is a map of parameter names to values.
    pub fn process(
        &self,
        input: &[u8],
        output: &mut [u8],
        width: u32,
        height: u32,
        params: &HashMap<String, f32>,
    ) -> Result<(), MediaPipelineError> {
        let mut store = Store::new(&self.engine, ());
        let mut linker = Linker::new(&self.engine);

        // Create memory
        let memory_type = MemoryType::new(256, None); // 256 pages = 16MB
        let memory = Memory::new(&mut store, memory_type)
            .map_err(|e| MediaPipelineError::Export(format!("wasm memory: {e}")))?;
        linker
            .define(&mut store, "env", "memory", memory)
            .map_err(|e| MediaPipelineError::Export(format!("wasm linker: {e}")))?;

        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| MediaPipelineError::Export(format!("wasm instantiate: {e}")))?;

        let buf_size = input.len();

        // Write input to WASM memory at offset 0
        memory
            .write(&mut store, 0, input)
            .map_err(|e| MediaPipelineError::Export(format!("wasm write input: {e}")))?;

        // Serialize params as [f32] array
        let param_values: Vec<f32> = params.values().copied().collect();
        let params_bytes: Vec<u8> = param_values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let params_offset = buf_size * 2; // after input and output regions
        if !params_bytes.is_empty() {
            memory
                .write(&mut store, params_offset, &params_bytes)
                .map_err(|e| MediaPipelineError::Export(format!("wasm write params: {e}")))?;
        }

        // Call process function
        let process_fn = instance
            .get_typed_func::<(i32, i32, i32, i32, i32, i32), ()>(&mut store, "process")
            .map_err(|e| MediaPipelineError::Export(format!("wasm process fn not found: {e}")))?;

        process_fn
            .call(
                &mut store,
                (
                    0,               // input_ptr
                    buf_size as i32, // output_ptr
                    width as i32,
                    height as i32,
                    params_offset as i32,
                    param_values.len() as i32,
                ),
            )
            .map_err(|e| MediaPipelineError::Export(format!("wasm process error: {e}")))?;

        // Read output from WASM memory
        memory
            .read(&store, buf_size, output)
            .map_err(|e| MediaPipelineError::Export(format!("wasm read output: {e}")))?;

        Ok(())
    }
}
