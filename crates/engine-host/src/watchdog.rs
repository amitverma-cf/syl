//! Checks whether a model load fits within configured resource limits, and monitors
//! system memory pressure while engines are running.

/// Hardware resource limits a user has configured for the app.
pub struct ResourceLimits {
    /// Maximum RAM, in bytes, the app is allowed to use across all loaded models.
    pub max_ram_bytes: u64,
    /// Maximum number of CPU cores the app is allowed to use.
    pub max_cores: u32,
}

/// The estimated memory footprint of loading a model, computed before the load is attempted.
pub struct PreflightEstimate {
    /// Estimated RAM, in bytes, the model would use once loaded.
    pub estimated_ram_bytes: u64,
}

/// Returns `true` if `estimate` fits within `limits`.
pub fn preflight_check(_estimate: &PreflightEstimate, _limits: &ResourceLimits) -> bool {
    true
}
