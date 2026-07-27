//! Dynamic loading of engine plugin shared libraries via `libloading`.
//! Decision #2: dynamic loading over static linking, so engines update without app rebuilds.

use core_types::EngineId;
use libloading::Library;

pub struct EnginePlugin {
    pub id: EngineId,
    #[allow(dead_code)]
    library: Library,
}

impl EnginePlugin {
    /// # Safety
    /// Loading and calling into a native shared library is inherently unsafe:
    /// the caller must ensure the library at `path` implements the expected plugin ABI.
    pub unsafe fn load(id: EngineId, path: &std::path::Path) -> Result<Self, libloading::Error> {
        let library = Library::new(path)?;
        Ok(Self { id, library })
    }
}
