mod cli;
mod output;
mod states;

use clap::Parser;
use ebur128::{EbuR128, Mode};
use std::process::ExitCode;
use wavers::{Wav, WaversResult};

use cli::Cli;
use output::{Output, fmt_frame, frame_to_time};
use states::{DetectorState, SilenceState};

const ERR_CONTAINS_UNDERRUN: u8 = 0b0001;
const ERR_CONTAINS_SILENCE: u8 = 0b0010;

fn analyse(args: &Cli, wav: &mut Wav<i32>) -> u8 {
    let mut return_code = 0;
    let (_, spec) = wav.wav_spec();
    let sample_rate = spec.fmt_chunk.sample_rate;
    let Ok(mut loudness) = EbuR128::new(
        wav.n_channels().into(),
        sample_rate as u32,
        Mode::S | Mode::I,
    ) else {
        panic!("Could not initialize EbuR128");
    };

    let mut states = vec![
        DetectorState {
            underrun_count: 0,
            underrun_prev_index: 0,
        };
        wav.n_channels().into()
    ];

    let mut silence_state = SilenceState::new();

    let silence_window_size = sample_rate as usize * wav.n_channels() as usize;
    let mut silence_frame_buf = vec![0; silence_window_size];
    let mut silence_frame_buf_iter = 0;
    let mut silence_count = 0;

    let digits = wav.n_samples().to_string().len();
    let num_frames = wav.n_samples();
    let frames = wav.frames();

    let output = Output::new(&args, num_frames as u64);

    for (frame_counter, frame) in frames.enumerate() {
        let frame_label = fmt_frame(frame_counter, digits);
        output.inc();

        // Detect silence
        if args.silence {
            for sample in frame.iter() {
                silence_frame_buf[silence_frame_buf_iter] = *sample;
                silence_frame_buf_iter += 1;
            }

            if silence_frame_buf_iter >= silence_window_size {
                silence_frame_buf_iter = 0;
                loudness.reset();

                if let Err(err) = loudness.add_frames_i32(&silence_frame_buf) {
                    println!(
                        "Warning: error adding frame to loudness measurement: {:?}",
                        &err
                    );
                }

                let lufs = loudness.loudness_shortterm().unwrap_or(f64::NEG_INFINITY);
                if lufs < args.lufs && silence_state.previous_lufs >= args.lufs {
                    silence_state.silence_start_frame = frame_counter;
                    println!(
                        "[{}] SILENCE START: LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {}",
                        frame_label,
                        lufs,
                        loudness.loudness_global().unwrap_or(-f64::INFINITY),
                        frame_to_time(frame_counter, sample_rate)
                    );
                }

                if lufs >= args.lufs && silence_state.previous_lufs < args.lufs {
                    silence_state.silence_end_frame = frame_counter;
                    silence_count +=
                        silence_state.silence_end_frame - silence_state.silence_start_frame;
                    println!(
                        "[{}] SILENCE END  : LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {} ({:04.3}% of total)",
                        frame_label,
                        lufs,
                        loudness.loudness_global().unwrap_or(-f64::INFINITY),
                        frame_to_time(frame_counter, sample_rate),
                        (silence_count as f32 / num_frames as f32) * 100.0
                    );
                }

                silence_state.previous_lufs = lufs;
                if args.debug {
                    println!(
                        "[{}] DEBUG        : LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {}",
                        frame_label,
                        lufs,
                        loudness.loudness_global().unwrap_or(-f64::INFINITY),
                        frame_to_time(frame_counter, sample_rate)
                    );
                }
            }
        }

        // Detect underruns
        if args.underrun {
            for (channel_index, sample) in frame.iter().enumerate() {
                assert!(channel_index < states.len());
                let state = &mut states[channel_index];
                if *sample == 0 {
                    if (frame_counter - state.underrun_prev_index) > 1 {
                        state.underrun_count = 0;
                    }

                    state.underrun_count += 1;
                    if args.debug {
                        println!(
                            "[{}] DEBUG        : 0-crossing @ {}",
                            frame_label,
                            frame_to_time(frame_counter, sample_rate),
                        );
                    }

                    state.underrun_prev_index = frame_counter;
                } else {
                    if state.underrun_count >= args.samples {
                        return_code |= ERR_CONTAINS_UNDERRUN;
                        let underrun_start =
                            frame_to_time(frame_counter - state.underrun_count, sample_rate);
                        let underrun_end = frame_to_time(frame_counter, sample_rate);
                        let underrun_duration = state.underrun_count as f32 / sample_rate as f32;
                        println!(
                            "[{}] UNDERRUN     : CH:{} - {} samples ({:06.3}s) {} -> {}",
                            frame_label,
                            channel_index,
                            state.underrun_count,
                            underrun_duration,
                            underrun_start,
                            underrun_end
                        );
                    }
                    state.underrun_count = 0;
                }
            }
        }
    }

    if args.underrun {
        for (channel_index, state) in states.iter().enumerate() {
            if state.underrun_count >= args.samples {
                return_code |= ERR_CONTAINS_UNDERRUN;
                let frame_label = fmt_frame(num_frames, digits);
                let underrun_start = frame_to_time(num_frames - state.underrun_count, sample_rate);
                let underrun_end = frame_to_time(num_frames, sample_rate);
                let underrun_duration = state.underrun_count as f32 / sample_rate as f32;
                println!(
                    "[{}] UNDERRUN     : CH:{} - {} samples ({:06.3}s) {} -> {}",
                    frame_label,
                    channel_index,
                    state.underrun_count,
                    underrun_duration,
                    underrun_start,
                    underrun_end
                );
            }
        }
    }

    if args.silence && silence_state.previous_lufs < args.lufs {
        silence_state.silence_end_frame = num_frames;
        silence_count += silence_state.silence_end_frame - silence_state.silence_start_frame;
        println!(
            "[{}] SILENCE END  : LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {} ({:04.3}% of total)",
            fmt_frame(num_frames, digits),
            silence_state.previous_lufs,
            loudness.loudness_global().unwrap_or(-f64::INFINITY),
            frame_to_time(num_frames, sample_rate),
            (silence_count as f32 / num_frames as f32) * 100.0
        );

        if (silence_count as f32 / num_frames as f32) * 100.0 >= args.silence_percentage as f32 {
            return_code |= ERR_CONTAINS_SILENCE;
        }
    }

    output.finish();
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
    println!("[+] sample rate:        {}", &spec.fmt_chunk.sample_rate);
    println!("[+] channels:           {}", wav.n_channels());
    println!("[+] total samples:      {}", wav.n_samples());
    if args.silence {
        println!("[+] silence threshold:  {} LUFS-S", &args.lufs);
    }
    if args.underrun {
        println!("[+] underrun threshold: {} samples", &args.samples);
    }

    let code = analyse(&args, &mut wav);
    ExitCode::from(code)
}
