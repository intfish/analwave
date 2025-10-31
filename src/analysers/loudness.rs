use std::vec;

use ebur128::{EbuR128, Error as EbuR128Error, Mode};
use serde::Serialize;
use wavers::{Samples, Wav};

use super::Analyser;
use crate::{debug, output, output::frame_to_time};

#[derive(Debug, Clone)]
pub struct SilenceState {
    pub previous_lufs: f64,
    pub silence_start_frame: usize,
    pub silence_end_frame: usize,
}

impl SilenceState {
    pub fn new() -> Self {
        Self {
            previous_lufs: 0.0,
            silence_start_frame: 0,
            silence_end_frame: 0,
        }
    }
}

struct InternalSegment {
    start: usize,
    end: Option<usize>,
}

#[derive(Serialize)]
pub struct SilenceSegment {
    pub start: f32,
    pub end: f32,
    pub duration: f32,
    #[serde(rename = "startSample")]
    pub start_sample: usize,
    #[serde(rename = "endSample")]
    pub end_sample: usize,
    #[serde(rename = "durationSamples")]
    pub duration_samples: usize,
}

struct Silence {
    count: usize,
    lufs: f64,
    percentage: f32,
    segments: Vec<InternalSegment>,
    state: SilenceState,
}

pub struct LoudnessAnalyser {
    frame_buf: Vec<i32>,
    frame_buf_iter: usize,
    loudness: EbuR128,
    num_frames: usize,
    sample_rate: i32,
    window_size: usize,
    silence: Option<Silence>,
}

impl LoudnessAnalyser {
    pub fn new(args: &crate::cli::Cli, wav: &Wav<i32>) -> Result<Self, EbuR128Error> {
        let (_, spec) = wav.wav_spec();
        let sample_rate = spec.fmt_chunk.sample_rate;
        let loudness = EbuR128::new(
            wav.n_channels().into(),
            sample_rate as u32,
            Mode::S | Mode::I,
        )?;

        let window_size =
            ((sample_rate as usize * wav.n_channels() as usize) as f32 * args.window_size) as usize;

        let silence = if args.silence {
            Some(Silence {
                count: 0,
                lufs: args.lufs,
                percentage: args.silence_percentage as f32,
                segments: Vec::new(),
                state: SilenceState::new(),
            })
        } else {
            None
        };

        Ok(Self {
            frame_buf: vec![0; window_size],
            frame_buf_iter: 0,
            loudness,
            num_frames: wav.n_samples(),
            sample_rate,
            window_size,
            silence,
        })
    }
}

impl Analyser for LoudnessAnalyser {
    fn analyse(&mut self, label: &str, frame_counter: usize, frame: &Samples<i32>) {
        for sample in frame.iter() {
            self.frame_buf[self.frame_buf_iter] = *sample;
            self.frame_buf_iter += 1;
        }

        if self.frame_buf_iter >= self.window_size {
            self.frame_buf_iter = 0;
            self.loudness.reset();

            if let Err(err) = self.loudness.add_frames_i32(&self.frame_buf) {
                println!(
                    "Warning: error adding frame to loudness measurement: {:?}",
                    &err
                );
            }

            let lufs = self
                .loudness
                .loudness_shortterm()
                .unwrap_or(f64::NEG_INFINITY);

            if let Some(silence) = &mut self.silence {
                if lufs < silence.lufs && silence.state.previous_lufs >= silence.lufs {
                    silence.state.silence_start_frame = frame_counter;
                    output!(
                        "[{}] SILENCE START: LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {}",
                        label,
                        lufs,
                        self.loudness.loudness_global().unwrap_or(-f64::INFINITY),
                        frame_to_time(frame_counter, self.sample_rate)
                    );

                    silence.segments.push(InternalSegment {
                        start: silence.state.silence_start_frame,
                        end: None,
                    });
                }

                if lufs >= silence.lufs && silence.state.previous_lufs < silence.lufs {
                    silence.state.silence_end_frame = frame_counter;
                    silence.count +=
                        silence.state.silence_end_frame - silence.state.silence_start_frame;

                    output!(
                        "[{}] SILENCE END  : LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {} ({:04.3}% of total)",
                        label,
                        lufs,
                        self.loudness.loudness_global().unwrap_or(-f64::INFINITY),
                        frame_to_time(frame_counter, self.sample_rate),
                        (silence.count as f32 / self.num_frames as f32) * 100.0
                    );

                    if let Some(segment) = silence.segments.last_mut() {
                        segment.end = Some(silence.state.silence_end_frame);
                    }
                }

                silence.state.previous_lufs = lufs;
            }

            debug!(
                "[{}] DEBUG        : LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {}",
                label,
                lufs,
                self.loudness.loudness_global().unwrap_or(-f64::INFINITY),
                frame_to_time(frame_counter, self.sample_rate)
            );
        }
    }

    fn finish(&mut self, label: &str) -> u8 {
        if let Some(silence) = &mut self.silence {
            if silence.state.previous_lufs < silence.lufs {
                let end_frame = self.num_frames;
                let count = silence.count + end_frame - silence.state.silence_start_frame;
                output!(
                    "[{}] SILENCE END  : LUFS-S: {:04.3}; LUFS-I: {:04.3} @ {} ({:04.3}% of total)",
                    label,
                    silence.state.previous_lufs,
                    self.loudness.loudness_global().unwrap_or(-f64::INFINITY),
                    frame_to_time(self.num_frames, self.sample_rate),
                    (count as f32 / self.num_frames as f32) * 100.0
                );

                if let Some(segment) = silence.segments.last_mut() {
                    segment.end = Some(end_frame);
                }

                if (count as f32 / self.num_frames as f32) * 100.0 >= silence.percentage {
                    return crate::ERR_CONTAINS_SILENCE;
                }
            }
        }

        0
    }

    fn json(&self) -> Vec<(String, serde_json::Value)> {
        let mut results = vec![];

        if let Some(silence) = &self.silence
            && !silence.segments.is_empty()
        {
            let segments: Vec<SilenceSegment> = silence
                .segments
                .iter()
                .map(|seg| {
                    let end_frame = seg.end.unwrap_or(self.num_frames);
                    let duration_samples = end_frame - seg.start;
                    SilenceSegment {
                        start: seg.start as f32 / self.sample_rate as f32,
                        end: end_frame as f32 / self.sample_rate as f32,
                        duration: duration_samples as f32 / self.sample_rate as f32,
                        start_sample: seg.start,
                        end_sample: end_frame,
                        duration_samples,
                    }
                })
                .collect();

            let analysis = serde_json::json!({
                "results": segments,
                "threshold": silence.lufs,
                "windowSize": self.window_size as f32 / self.sample_rate as f32,
            });

            results.push(("silence".to_string(), analysis));
        }

        results
    }
}
