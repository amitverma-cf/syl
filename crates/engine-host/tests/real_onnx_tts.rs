use std::path::Path;

use engine_host::onnx_tts::OnnxTtsEngine;

#[test]
#[ignore]
fn synthesizes_real_non_silent_audio() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime_library_path = repo_root.join(".syl/engines/onnxruntime/onnxruntime.dll");
    let model_dir = repo_root.join(".syl/models/mms-tts-eng");
    let model_path = model_dir.join("model.onnx");
    let vocab_path = model_dir.join("vocab.json");

    for p in [&runtime_library_path, &model_path, &vocab_path] {
        assert!(p.exists(), "expected {} to exist", p.display());
    }

    let mut engine = OnnxTtsEngine::load(&runtime_library_path, &model_path, &vocab_path).unwrap();

    let pcm = engine
        .synthesize("hello world, this is a real test")
        .unwrap();

    let sample_rate = 16_000.0;
    let duration_secs = pcm.len() as f32 / sample_rate;
    println!("generated {} samples ({:.2}s)", pcm.len(), duration_secs);

    assert!(
        duration_secs > 0.5,
        "expected at least half a second of audio, got {duration_secs}s"
    );

    let rms = (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len() as f32).sqrt();
    println!("rms energy: {rms}");
    assert!(rms > 0.001, "expected non-silent audio, got rms={rms}");

    let wav_path = std::env::temp_dir().join("syl_tts_smoke.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
    for sample in &pcm {
        let clamped = (*sample).clamp(-1.0, 1.0);
        writer
            .write_sample((clamped * i16::MAX as f32) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
    println!("wrote {}", wav_path.display());
}
