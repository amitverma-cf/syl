//! Dynamic loading of engine plugin shared libraries.

use core_types::EngineId;
use libloading::Library;

/// A native engine plugin (llama.cpp, ONNX Runtime, Stable Diffusion, ...) loaded from a
/// shared library at runtime.
pub struct EnginePlugin {
    /// The engine this plugin implements.
    pub id: EngineId,
    _library: Library,
}

impl EnginePlugin {
    /// Loads the shared library at `path` and returns a handle to it, keeping the library
    /// resident for as long as the returned `EnginePlugin` is alive.
    ///
    /// # Errors
    /// Returns an error if the library at `path` cannot be opened.
    ///
    /// # Safety
    /// The caller must ensure the library at `path` implements the expected plugin ABI.
    /// Loading and calling into a native shared library is inherently unsafe.
    pub unsafe fn load(id: EngineId, path: &std::path::Path) -> Result<Self, libloading::Error> {
        let library = Library::new(path)?;
        Ok(Self {
            id,
            _library: library,
        })
    }
}
