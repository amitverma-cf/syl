use libloading::Library;
use syl_core::EngineId;

pub struct EnginePlugin {
    pub id: EngineId,
    _library: Library,
}

impl EnginePlugin {
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
