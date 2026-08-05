pub struct ResourceLimits {
    pub max_ram_bytes: u64,
    pub max_cores: u32,
}

pub struct PreflightEstimate {
    pub estimated_ram_bytes: u64,
}

pub fn preflight_check(_estimate: &PreflightEstimate, _limits: &ResourceLimits) -> bool {
    true
}
