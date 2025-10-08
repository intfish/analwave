#[derive(Debug, Clone)]
pub struct DetectorState {
    pub underrun_count: usize,
    pub underrun_prev_index: usize,
}

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
