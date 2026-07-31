use std::path::Path;

use crate::bindings::LlamaCpp;
use crate::dll::path_to_cstring;

#[derive(Debug, thiserror::Error)]
pub enum GgufMetaError {
    #[error("failed to load llama.cpp library: {0}")]
    LibraryLoad(#[source] libloading::Error),
    #[error("failed to parse gguf metadata from {}", .0.display())]
    Parse(std::path::PathBuf),
}

const FTYPE_NAMES: &[(u32, &str)] = &[
    (0, "F32"),
    (1, "F16"),
    (2, "Q4_0"),
    (3, "Q4_1"),
    (7, "Q8_0"),
    (8, "Q5_0"),
    (9, "Q5_1"),
    (10, "Q2_K"),
    (11, "Q3_K_S"),
    (12, "Q3_K_M"),
    (13, "Q3_K_L"),
    (14, "Q4_K_S"),
    (15, "Q4_K_M"),
    (16, "Q5_K_S"),
    (17, "Q5_K_M"),
    (18, "Q6_K"),
    (19, "IQ2_XXS"),
    (20, "IQ2_XS"),
    (21, "Q2_K_S"),
    (22, "IQ3_XS"),
    (23, "IQ3_XXS"),
    (24, "IQ1_S"),
    (25, "IQ4_NL"),
    (26, "IQ3_S"),
    (27, "IQ3_M"),
    (28, "IQ2_S"),
    (29, "IQ2_M"),
    (30, "IQ4_XS"),
    (31, "IQ1_M"),
    (32, "BF16"),
    (36, "TQ1_0"),
    (37, "TQ2_0"),
    (38, "MXFP4_MOE"),
];

fn ftype_name(ftype: u32) -> String {
    FTYPE_NAMES
        .iter()
        .find(|(id, _)| *id == ftype)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| format!("unknown ({ftype})"))
}

fn ggml_base_library_path(llama_library_path: &Path) -> std::path::PathBuf {
    let dir = llama_library_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let file_name = if cfg!(target_os = "windows") {
        "ggml-base.dll"
    } else if cfg!(target_os = "macos") {
        "libggml-base.dylib"
    } else {
        "libggml-base.so"
    };
    dir.join(file_name)
}

pub fn read_quantization(library_path: &Path, gguf_path: &Path) -> Result<String, GgufMetaError> {
    let ggml_base_path = ggml_base_library_path(library_path);
    let lib = unsafe { LlamaCpp::new(&ggml_base_path) }.map_err(GgufMetaError::LibraryLoad)?;

    let path_c = path_to_cstring(gguf_path);
    let params = crate::bindings::gguf_init_params {
        no_alloc: true,
        ctx: std::ptr::null_mut(),
    };

    let ctx = unsafe { lib.gguf_init_from_file(path_c.as_ptr(), params) };
    if ctx.is_null() {
        return Err(GgufMetaError::Parse(gguf_path.to_path_buf()));
    }

    let key = std::ffi::CString::new("general.file_type").unwrap_or_default();
    let key_id = unsafe { lib.gguf_find_key(ctx, key.as_ptr()) };
    let result = if key_id >= 0 {
        let ftype = unsafe { lib.gguf_get_val_u32(ctx, key_id) };
        ftype_name(ftype)
    } else {
        "unknown".to_string()
    };

    unsafe { lib.gguf_free(ctx) };
    Ok(result)
}
