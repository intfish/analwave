use crate::cli::Cli;
use indicatif::{ProgressBar, ProgressStyle};

pub fn fmt_frame(frame: usize, digits: usize) -> String {
    format!("{:0width$}", frame, width = digits)
}

pub fn frame_to_time(frame: usize, sample_rate: i32) -> String {
    let seconds = frame as f32 / sample_rate as f32;
    let hours = (seconds / 3600.0).floor();
    let minutes = ((seconds % 3600.0) / 60.0).floor();
    let secs = seconds % 60.0;
    format!("{:02.0}:{:02.0}:{:06.3}", hours, minutes, secs)
}

#[derive(Debug)]
pub struct Output {
    pub progress_bar: Option<ProgressBar>,
}

impl Output {
    pub fn new(args: &Cli, num_frames: u64) -> Self {
        let progress_bar = if args.no_progress {
            None
        } else {
            Some(ProgressBar::new(num_frames))
        };

        if let Some(pb) = &progress_bar {
            pb.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.yellow/green}] {percent_precise}% ({pos}/{len})")
                .unwrap()
                .progress_chars("#>-"));
        }

        Self { progress_bar }
    }

    pub fn inc(&self) {
        if let Some(pb) = &self.progress_bar {
            pb.inc(1);
        }
    }

    pub fn finish(&self) {
        if let Some(pb) = &self.progress_bar {
            pb.finish();
        }
    }
}
