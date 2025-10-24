mod analysers;
mod cli;
mod output;

use clap::Parser;
use std::process::ExitCode;
use wavers::{Wav, WaversResult};

use analysers::{Analyser, silence::SilenceAnalyser};
use cli::Cli;
use output::{fmt_frame, init_output};

use crate::analysers::underruns::UnderrunAnalyser;

const ERR_CONTAINS_UNDERRUN: u8 = 0b0001;
const ERR_CONTAINS_SILENCE: u8 = 0b0010;

fn analyse(args: &Cli, wav: &mut Wav<i32>) -> u8 {
    let mut return_code = 0;

    let mut analysers: Vec<Box<dyn Analyser>> = vec![
        Box::new(SilenceAnalyser::new(args, wav).expect("Could not initialize EbuR128")),
        Box::new(UnderrunAnalyser::new(args, wav)),
    ];

    let digits = wav.n_samples().to_string().len();
    let num_frames = wav.n_samples();
    let frames = wav.frames();

    for (frame_counter, frame) in frames.enumerate() {
        let frame_label = fmt_frame(frame_counter, digits);
        output::inc();

        for analyser in analysers.iter_mut() {
            analyser.analyse(&frame_label, frame_counter, &frame);
        }
    }

    let frame_label = fmt_frame(num_frames, digits);

    for analyser in analysers.iter() {
        return_code |= analyser.finish(&frame_label);
    }

    return_code
}

fn main() -> ExitCode {
    let args = Cli::parse();
    let Ok(mut wav): WaversResult<Wav<i32>> = Wav::from_path(&args.input) else {
        println!("Could not open file: {}", args.input);
        return ExitCode::from(1);
    };

    if !args.underrun && !args.silence {
        println!("Neither underrun nor silence detection is active, exiting.");
        return ExitCode::from(1);
    }

    let (_, spec) = wav.wav_spec();
    init_output(&args, wav.n_samples() as u64);

    output!("[+] sample rate:        {}", &spec.fmt_chunk.sample_rate);
    output!("[+] channels:           {}", wav.n_channels());
    output!("[+] total samples:      {}", wav.n_samples());

    if args.silence {
        output!("[+] silence threshold:  {} LUFS-S", &args.lufs);
    }
    if args.underrun {
        output!("[+] underrun threshold: {} samples", &args.samples);
    }

    let code = analyse(&args, &mut wav);

    output::finish();

    ExitCode::from(code)
}
