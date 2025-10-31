use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The file to analyse
    #[arg(short, long)]
    pub input: String,

    /// Detect underruns
    #[arg(short, long, default_value_t = false)]
    pub underrun: bool,

    /// Underrun detection minimum samples
    #[arg(long, default_value_t = 16)]
    pub samples: usize,

    /// Detect silence
    #[arg(short, long, default_value_t = false)]
    pub silence: bool,

    /// Silence threshold (LUFS-S)
    #[arg(long, default_value_t = -70.0)]
    pub lufs: f64,

    /// Silence percentage (returns error code if total silence is above this threshold)
    #[arg(long, default_value_t = 99)]
    pub silence_percentage: u16,

    /// No fancy progress-bar
    #[arg(long, default_value_t = false)]
    pub no_progress: bool,

    /// Debug output
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Silent (no output)
    #[arg(long, default_value_t = false)]
    pub silent: bool,

    /// Output results as JSON to file
    #[arg(long)]
    pub json: Option<String>,

    /// Window size for silence / loudness in seconds
    #[arg(long, default_value_t = 1.0)]
    pub window_size: f32,

    /// Track loudness to JSON (does nothing if JSON output is not enabled)
    #[arg(short, long, default_value_t = false)]
    pub loudness: bool,
}
