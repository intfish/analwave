use wavers::Samples;

pub mod loudness;
pub mod underruns;

pub trait Analyser {
    fn analyse(&mut self, label: &str, frame_counter: usize, frame: &Samples<i32>);
    fn finish(&mut self, label: &str) -> u8;
    fn json(&self) -> Vec<(String, serde_json::Value)> {
        Vec::new()
    }
}
