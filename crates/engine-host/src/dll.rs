use std::ffi::CString;
use std::path::Path;

#[cfg(target_os = "windows")]
extern "system" {
    fn SetDllDirectoryW(lp_path_name: *const u16) -> i32;
}

#[cfg(target_os = "windows")]
pub fn prioritize_dll_directory(dir: &Path) {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe { SetDllDirectoryW(wide.as_ptr()) };
}

#[cfg(not(target_os = "windows"))]
pub fn prioritize_dll_directory(_dir: &Path) {}

pub fn path_to_cstring(path: &Path) -> CString {
    CString::new(path.to_string_lossy().as_bytes()).unwrap_or_default()
}

fn ggml_base_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ggml.dll"
    } else if cfg!(target_os = "macos") {
        "libggml.dylib"
    } else {
        "libggml.so"
    }
}

pub fn load_ggml_backends(library_dir: &Path) -> Result<(), libloading::Error> {
    let dir_c = path_to_cstring(library_dir);
    let ggml_base_path = library_dir.join(ggml_base_library_name());
    let ggml = unsafe { crate::ggml_bindings::GgmlBackend::new(&ggml_base_path) }?;
    unsafe { ggml.ggml_backend_load_all_from_path(dir_c.as_ptr()) };
    Ok(())
}
