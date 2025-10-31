use serde::Serialize;
use wavers::{Samples, Wav};

use super::Analyser;
use crate::{debug, output, output::frame_to_time};

#[derive(Debug, Clone)]
pub struct DetectorState {
    pub underrun_count: usize,
    pub underrun_prev_index: usize,
}

struct InternalSegment {
    start: usize,
    end: Option<usize>,
    channel: usize,
}

#[derive(Serialize)]
pub struct UnderrunSegment {
    pub start: f32,
    pub end: f32,
    pub duration: f32,
    #[serde(rename = "startSample")]
    pub start_sample: usize,
    #[serde(rename = "endSample")]
    pub end_sample: usize,
    #[serde(rename = "durationSamples")]
    pub duration_samples: usize,
    pub channel: usize,
}

pub struct UnderrunAnalyser {
    contains_underrun: bool,
    num_frames: usize,
    states: Vec<DetectorState>,
    sample_rate: i32,
    samples: usize,
    segments: Vec<InternalSegment>,
}

impl UnderrunAnalyser {
    pub fn new(args: &crate::cli::Cli, wav: &Wav<i32>) -> Self {
        Self {
            contains_underrun: false,
            num_frames: wav.n_samples(),
            states: vec![
                DetectorState {
                    underrun_count: 0,
                    underrun_prev_index: 0,
                };
                wav.n_channels().into()
            ],
            sample_rate: wav.wav_spec().1.fmt_chunk.sample_rate,
            samples: args.samples,
            segments: Vec::new(),
        }
    }
}

impl Analyser for UnderrunAnalyser {
    fn analyse(&mut self, label: &str, frame_counter: usize, frame: &Samples<i32>) {
        for (channel_index, sample) in frame.iter().enumerate() {
            assert!(channel_index < self.states.len());
            let state = &mut self.states[channel_index];
            if *sample == 0 {
                if (frame_counter - state.underrun_prev_index) > 1 {
                    state.underrun_count = 0;
                }

                state.underrun_count += 1;
                debug!(
                    "[{}] DEBUG        : 0-crossing @ {}",
                    label,
                    frame_to_time(frame_counter, self.sample_rate),
                );

                state.underrun_prev_index = frame_counter;
            } else {
                if state.underrun_count >= self.samples {
                    self.contains_underrun = true;
                    let underrun_start =
                        frame_to_time(frame_counter - state.underrun_count, self.sample_rate);
                    let underrun_end = frame_to_time(frame_counter, self.sample_rate);
                    let underrun_duration = state.underrun_count as f32 / self.sample_rate as f32;
                    output!(
                        "[{}] UNDERRUN     : CH:{} - {} samples ({:06.3}s) {} -> {}",
                        label,
                        channel_index,
                        state.underrun_count,
                        underrun_duration,
                        underrun_start,
                        underrun_end
                    );

                    self.segments.push(InternalSegment {
                        start: frame_counter - state.underrun_count,
                        end: Some(frame_counter),
                        channel: channel_index,
                    });
                }
                state.underrun_count = 0;
            }
        }
    }

    fn finish(&mut self, label: &str) -> u8 {
        let mut contains_underrun = self.contains_underrun;
        for (channel_index, state) in self.states.iter().enumerate() {
            if state.underrun_count >= self.samples {
                contains_underrun = true;
                let underrun_start =
                    frame_to_time(self.num_frames - state.underrun_count, self.sample_rate);
                let underrun_end = frame_to_time(self.num_frames, self.sample_rate);
                let underrun_duration = state.underrun_count as f32 / self.sample_rate as f32;
                output!(
                    "[{}] UNDERRUN     : CH:{} - {} samples ({:06.3}s) {} -> {}",
                    &label,
                    channel_index,
                    state.underrun_count,
                    underrun_duration,
                    underrun_start,
                    underrun_end
                );

                self.segments.push(InternalSegment {
                    start: self.num_frames - state.underrun_count,
                    end: Some(self.num_frames),
                    channel: channel_index,
                });
            }
        }

        if contains_underrun {
            crate::ERR_CONTAINS_UNDERRUN
        } else {
            0
        }
    }

    fn json(&self) -> Option<(String, serde_json::Value)> {
        if self.segments.is_empty() {
            return None;
        }

        let segments: Vec<UnderrunSegment> = self
            .segments
            .iter()
            .map(|seg| {
                let end_frame = seg.end.unwrap_or(self.num_frames);
                let duration_samples = end_frame - seg.start;
                UnderrunSegment {
                    start: seg.start as f32 / self.sample_rate as f32,
                    end: end_frame as f32 / self.sample_rate as f32,
                    duration: duration_samples as f32 / self.sample_rate as f32,
                    start_sample: seg.start,
                    end_sample: end_frame,
                    duration_samples,
                    channel: seg.channel,
                }
            })
            .collect();

        let analysis = serde_json::json!({
            "results": segments,
            "threshold": self.samples,
        });

        Some(("underruns".to_string(), analysis))
    }
}
