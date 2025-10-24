use wavers::Samples;

pub mod silence;
pub mod underruns;

pub trait Analyser {
    fn analyse(&mut self, label: &str, frame_counter: usize, frame: &Samples<i32>);
    fn finish(&self, label: &str) -> u8;
}
