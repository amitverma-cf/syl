use std::ffi::CString;
use std::path::Path;

use crate::dll::{load_ggml_backends, path_to_cstring, prioritize_dll_directory};
use crate::sd_bindings::StableDiffusionCpp;

#[derive(Debug, thiserror::Error)]
pub enum SdError {
    #[error("failed to load stable-diffusion.cpp library: {0}")]
    LibraryLoad(#[source] libloading::Error),
    #[error("failed to create stable-diffusion.cpp context from {}", .0.display())]
    ContextCreate(std::path::PathBuf),
    #[error("image generation failed")]
    Generate,
    #[error("failed to encode generated image as PNG: {0}")]
    PngEncode(#[from] png::EncodingError),
}

pub struct SdEngine {
    lib: StableDiffusionCpp,
    ctx: *mut crate::sd_bindings::sd_ctx_t,
}

unsafe impl Send for SdEngine {}

impl SdEngine {
    pub fn load(library_path: &Path, model_path: &Path, n_threads: i32) -> Result<Self, SdError> {
        if let Some(dir) = library_path.parent() {
            prioritize_dll_directory(dir);
        }

        let lib = unsafe { StableDiffusionCpp::new(library_path) }.map_err(SdError::LibraryLoad)?;

        let backend_dir = library_path.parent().unwrap_or_else(|| Path::new("."));
        load_ggml_backends(backend_dir).map_err(SdError::LibraryLoad)?;

        let mut ctx_params: crate::sd_bindings::sd_ctx_params_t = unsafe { std::mem::zeroed() };
        unsafe { lib.sd_ctx_params_init(&mut ctx_params) };

        let model_path_c = path_to_cstring(model_path);
        ctx_params.model_path = model_path_c.as_ptr();
        ctx_params.n_threads = n_threads;

        let ctx = unsafe { lib.new_sd_ctx(&ctx_params) };
        if ctx.is_null() {
            return Err(SdError::ContextCreate(model_path.to_path_buf()));
        }

        Ok(Self { lib, ctx })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn txt2img(
        &mut self,
        prompt: &str,
        negative_prompt: &str,
        width: i32,
        height: i32,
        steps: i32,
        seed: i64,
    ) -> Result<Vec<u8>, SdError> {
        let mut img_params: crate::sd_bindings::sd_img_gen_params_t = unsafe { std::mem::zeroed() };
        unsafe { self.lib.sd_img_gen_params_init(&mut img_params) };

        let prompt_c = CString::new(prompt).unwrap_or_default();
        let negative_prompt_c = CString::new(negative_prompt).unwrap_or_default();
        img_params.prompt = prompt_c.as_ptr();
        img_params.negative_prompt = negative_prompt_c.as_ptr();
        img_params.width = width;
        img_params.height = height;
        img_params.sample_params.sample_steps = steps;
        img_params.seed = seed;
        img_params.batch_count = 1;

        let mut images_out: *mut crate::sd_bindings::sd_image_t = std::ptr::null_mut();
        let mut num_images_out: i32 = 0;
        let ok = unsafe {
            self.lib
                .generate_image(self.ctx, &img_params, &mut images_out, &mut num_images_out)
        };
        if !ok || images_out.is_null() || num_images_out == 0 {
            return Err(SdError::Generate);
        }

        let image = unsafe { &*images_out };
        let png_bytes = encode_png(image);
        unsafe { self.lib.free_sd_images(images_out, num_images_out) };
        png_bytes
    }
}

fn encode_png(image: &crate::sd_bindings::sd_image_t) -> Result<Vec<u8>, SdError> {
    let pixel_count = (image.width as usize) * (image.height as usize) * (image.channel as usize);
    let pixels = unsafe { std::slice::from_raw_parts(image.data, pixel_count) };

    let mut png_bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut png_bytes, image.width, image.height);
    let color_type = match image.channel {
        1 => png::ColorType::Grayscale,
        3 => png::ColorType::Rgb,
        4 => png::ColorType::Rgba,
        _ => png::ColorType::Rgb,
    };
    encoder.set_color(color_type);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)?;
    drop(writer);
    Ok(png_bytes)
}

impl Drop for SdEngine {
    fn drop(&mut self) {
        unsafe { self.lib.free_sd_ctx(self.ctx) };
    }
}
