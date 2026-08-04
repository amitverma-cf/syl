//! Host-provided capabilities extensions can declare in their manifest's
//! `requires` list. Empty today — no extension in this pass needs anything
//! from the host beyond being called into — but real and checked so a
//! future extension (a PDF reader needing workspace file access, say) has
//! an actual contract to declare against instead of the host silently
//! accepting a requirement it can't satisfy.
pub const HOST_CAPABILITIES: &[&str] = &[];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("extension requires capability {0:?}, which this host does not provide")]
pub struct UnsupportedRequirement(pub String);

/// Checks every capability an extension's manifest declares in `requires`
/// against what this host actually implements — called before the
/// extension is ever spawned, so an unsatisfiable requirement fails fast
/// with a clear error instead of the extension process starting and then
/// failing confusingly partway through its first real request.
pub fn check_requirements(requires: &[String]) -> Result<(), UnsupportedRequirement> {
    for capability in requires {
        if !HOST_CAPABILITIES.contains(&capability.as_str()) {
            return Err(UnsupportedRequirement(capability.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_requirements_is_always_satisfied() {
        check_requirements(&[]).unwrap();
    }

    #[test]
    fn an_unimplemented_requirement_is_rejected() {
        let err = check_requirements(&["workspace.fs/v1".to_string()]).unwrap_err();
        assert_eq!(err, UnsupportedRequirement("workspace.fs/v1".to_string()));
    }
}
