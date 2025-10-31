use serde::Serialize;
use serde_json::{Map, Value, to_string_pretty};
use wavers::Wav;

use crate::{analysers::Analyser, cli::Cli, output};

#[derive(Serialize)]
struct JsonOutput {
    analysis: Map<String, Value>,
    duration: f32,
    num_channels: u16,
    num_samples: usize,
    sample_rate: i32,
}

pub fn write_json(args: &Cli, wav: &Wav<i32>, analysers: &Vec<Box<dyn Analyser>>) {
    let Some(path) = args.json.as_ref() else {
        return;
    };

    let mut analysis = Map::new();

    for analyser in analysers.iter() {
        if let Some((key, value)) = analyser.json() {
            analysis.insert(key, value);
        }
    }

    if analysis.is_empty() {
        // Shouldn't happen
        return;
    }

    let (_, spec) = wav.wav_spec();
    let sample_rate = spec.fmt_chunk.sample_rate;
    let num_samples = wav.n_samples();

    std::fs::write(
        path,
        to_string_pretty(&JsonOutput {
            analysis,
            duration: num_samples as f32 / sample_rate as f32,
            num_channels: wav.n_channels(),
            num_samples,
            sample_rate,
        })
        .unwrap(),
    )
    .expect("Could not write JSON output to file");

    output!("Wrote JSON output to {}", path);
}
