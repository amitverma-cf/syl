const MEL_FILTERS_NPY: &[u8] = include_bytes!("../vendor/whisper-assets/mel_80.npy");

pub const N_MELS: usize = 80;
pub const N_FFT: usize = 400;
pub const HOP_LENGTH: usize = 160;
pub const N_FRAMES: usize = 3000;
pub const N_SAMPLES: usize = 480_000;
const N_FREQ_BINS: usize = N_FFT / 2 + 1;

fn load_mel_filters() -> Vec<f32> {
    let magic = &MEL_FILTERS_NPY[0..6];
    assert_eq!(magic, b"\x93NUMPY", "malformed mel filter asset");
    let major = MEL_FILTERS_NPY[6];
    let header_len_bytes = if major == 1 { 2 } else { 4 };
    let header_start = 8 + header_len_bytes;
    let header_len = if major == 1 {
        u16::from_le_bytes([MEL_FILTERS_NPY[8], MEL_FILTERS_NPY[9]]) as usize
    } else {
        u32::from_le_bytes([
            MEL_FILTERS_NPY[8],
            MEL_FILTERS_NPY[9],
            MEL_FILTERS_NPY[10],
            MEL_FILTERS_NPY[11],
        ]) as usize
    };
    let data_start = header_start + header_len;
    let data = &MEL_FILTERS_NPY[data_start..];
    data.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

fn hann_window(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos())
        .collect()
}

fn reflect_pad(samples: &[f32], pad: usize) -> Vec<f32> {
    let mut padded = Vec::with_capacity(samples.len() + 2 * pad);
    for i in (1..=pad).rev() {
        padded.push(samples[i.min(samples.len() - 1)]);
    }
    padded.extend_from_slice(samples);
    for i in 1..=pad {
        let idx = samples.len().saturating_sub(1).saturating_sub(i);
        padded.push(samples[idx]);
    }
    padded
}

pub fn log_mel_spectrogram(pcm: &[f32]) -> Vec<f32> {
    let mut samples = pcm.to_vec();
    samples.resize(N_SAMPLES, 0.0);

    let window = hann_window(N_FFT);
    let padded = reflect_pad(&samples, N_FFT / 2);

    let mut planner = rustfft::FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N_FFT);

    let mut power_spectrum = vec![0f32; N_FREQ_BINS * N_FRAMES];
    let mut buffer = vec![rustfft::num_complex::Complex::new(0f32, 0f32); N_FFT];

    for frame in 0..N_FRAMES {
        let start = frame * HOP_LENGTH;
        for i in 0..N_FFT {
            buffer[i] = rustfft::num_complex::Complex::new(padded[start + i] * window[i], 0.0);
        }
        fft.process(&mut buffer);
        for bin in 0..N_FREQ_BINS {
            power_spectrum[bin * N_FRAMES + frame] = buffer[bin].norm_sqr();
        }
    }

    let mel_filters = load_mel_filters();
    let mut mel_spec = vec![0f32; N_MELS * N_FRAMES];
    for mel in 0..N_MELS {
        for bin in 0..N_FREQ_BINS {
            let weight = mel_filters[mel * N_FREQ_BINS + bin];
            if weight == 0.0 {
                continue;
            }
            let row = &power_spectrum[bin * N_FRAMES..(bin + 1) * N_FRAMES];
            let out_row = &mut mel_spec[mel * N_FRAMES..(mel + 1) * N_FRAMES];
            for frame in 0..N_FRAMES {
                out_row[frame] += weight * row[frame];
            }
        }
    }

    for v in mel_spec.iter_mut() {
        *v = v.max(1e-10).log10();
    }
    let max_log = mel_spec.iter().cloned().fold(f32::MIN, f32::max);
    for v in mel_spec.iter_mut() {
        *v = v.max(max_log - 8.0);
        *v = (*v + 4.0) / 4.0;
    }

    mel_spec
}
