use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;

const TARGET_SAMPLE_RATE: u32 = 16_000;

fn resample_to_16khz(samples: &[f32], input_rate: u32, channels: u16) -> Vec<f32> {
    let mono: Vec<f32> = samples
        .chunks_exact(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();

    if input_rate == TARGET_SAMPLE_RATE {
        return mono;
    }

    let ratio = TARGET_SAMPLE_RATE as f64 / input_rate as f64;
    let out_len = (mono.len() as f64 * ratio) as usize;
    (0..out_len)
        .map(|i| {
            let src_pos = i as f64 / ratio;
            let idx = src_pos as usize;
            let frac = (src_pos - idx as f64) as f32;
            let a = mono.get(idx).copied().unwrap_or(0.0);
            let b = mono.get(idx + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

pub fn record_16khz_mono(seconds: f32) -> Result<Vec<f32>, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no default input (microphone) device available".to_string())?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("failed to get default input config: {e}"))?;

    let input_rate = config.sample_rate().0;
    let channels = config.channels();
    let sample_format = config.sample_format();

    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));
    let captured_for_stream = captured.clone();

    let err_fn = |err| tracing::error!(?err, "audio input stream error");

    let stream = match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    captured_for_stream
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .extend_from_slice(data);
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        SampleFormat::I16 => device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    let mut buf = captured_for_stream
                        .lock()
                        .unwrap_or_else(|p| p.into_inner());
                    buf.extend(data.iter().map(|&s| s as f32 / i16::MAX as f32));
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        other => return Err(format!("unsupported input sample format: {other:?}")),
    };

    stream.play().map_err(|e| e.to_string())?;
    std::thread::sleep(std::time::Duration::from_secs_f32(seconds.max(0.1)));
    drop(stream);

    let raw = captured.lock().unwrap_or_else(|p| p.into_inner()).clone();
    Ok(resample_to_16khz(&raw, input_rate, channels))
}

pub fn play_16khz_mono(pcm: &[f32]) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output (speaker) device available".to_string())?;
    let config = device
        .default_output_config()
        .map_err(|e| format!("failed to get default output config: {e}"))?;

    let output_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let upsampled = if output_rate == TARGET_SAMPLE_RATE {
        pcm.to_vec()
    } else {
        let ratio = output_rate as f64 / TARGET_SAMPLE_RATE as f64;
        let out_len = (pcm.len() as f64 * ratio) as usize;
        (0..out_len)
            .map(|i| {
                let src_pos = i as f64 / ratio;
                let idx = src_pos as usize;
                let frac = (src_pos - idx as f64) as f32;
                let a = pcm.get(idx).copied().unwrap_or(0.0);
                let b = pcm.get(idx + 1).copied().unwrap_or(a);
                a + (b - a) * frac
            })
            .collect()
    };

    let mut cursor = 0usize;
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let done_for_stream = done.clone();
    let total_frames = upsampled.len();
    let err_fn = |err| tracing::error!(?err, "audio output stream error");

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                for frame in data.chunks_mut(channels) {
                    let sample = upsampled.get(cursor).copied().unwrap_or(0.0);
                    for out in frame.iter_mut() {
                        *out = sample;
                    }
                    cursor += 1;
                    if cursor >= total_frames {
                        done_for_stream.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| e.to_string())?;

    stream.play().map_err(|e| e.to_string())?;
    let playback_secs = total_frames as f32 / TARGET_SAMPLE_RATE as f32;
    std::thread::sleep(std::time::Duration::from_secs_f32(playback_secs + 0.2));
    Ok(())
}
