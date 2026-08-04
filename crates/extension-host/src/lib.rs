mod capability;
mod manager;
mod manifest;
mod process;
mod protocol;

pub use capability::{check_requirements, UnsupportedRequirement, HOST_CAPABILITIES};
pub use manager::{discover_installed_extensions, find_extension, with_backend_args};
pub use manifest::{
    load_manifest, Contributions, ExtensionBackend, ExtensionManifest, ManifestError,
    UiContribution,
};
pub use process::{ExtensionProcess, ExtensionProcessError};
pub use protocol::{RpcErrorPayload, RpcMessage};
