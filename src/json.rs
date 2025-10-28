use serde_json::{Map, Value, to_string_pretty};

use crate::{analysers::Analyser, cli::Cli, output};

pub fn write_json(args: &Cli, analysers: &Vec<Box<dyn Analyser>>) {
    let mut json_output = Map::new();

    for analyser in analysers.iter() {
        if let Some((key, value)) = analyser.json() {
            json_output.insert(key, value);
        }
    }

    if json_output.is_empty() {
        return;
    }

    let json_value = Value::Object(json_output);
    let Some(path) = args.json.as_ref() else {
        println!("No JSON output file specified");
        return;
    };

    std::fs::write(path, to_string_pretty(&json_value).unwrap())
        .expect("Could not write JSON output to file");

    output!("Wrote JSON output to {}", path);
}
