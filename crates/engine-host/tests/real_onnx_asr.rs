use std::path::Path;

use engine_host::onnx_asr::OnnxAsrEngine;

fn read_wav_16khz_mono_f32(path: &Path) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 16000, "fixture must be 16kHz");
    assert_eq!(spec.channels, 1, "fixture must be mono");
    match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / i16::MAX as f32)
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    }
}

#[test]
#[ignore]
fn transcribes_real_speech_audio() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runtime_library_path = repo_root.join(".syl/engines/onnxruntime/onnxruntime.dll");
    let model_dir = repo_root.join(".syl/models/whisper-tiny");
    let encoder_path = model_dir.join("encoder_model.onnx");
    let decoder_path = model_dir.join("decoder_model.onnx");
    let tokenizer_path = model_dir.join("tokenizer.json");
    let wav_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/jfk.wav");

    for p in [
        &runtime_library_path,
        &encoder_path,
        &decoder_path,
        &tokenizer_path,
        &wav_path,
    ] {
        assert!(p.exists(), "expected {} to exist", p.display());
    }

    let pcm = read_wav_16khz_mono_f32(&wav_path);

    let mut engine = OnnxAsrEngine::load(
        &runtime_library_path,
        &encoder_path,
        &decoder_path,
        &tokenizer_path,
    )
    .unwrap();

    let text = engine.transcribe(&pcm).unwrap();
    println!("transcribed: {text}");

    let lower = text.to_lowercase();
    assert!(
        lower.contains("country") || lower.contains("ask"),
        "expected transcription to mention the well-known JFK quote, got: {text}"
    );
}
