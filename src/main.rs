mod cli;
mod error;
mod math;

use clap::Parser;
use cli::Args;

fn main() {
    let args = Args::parse();
    match args.validate() {
        Ok(config) => {
            println!("Input: {:?}", config.input);
            println!("Bbox: {:?}", config.bbox);
            println!("Resolution: {}", config.resolution);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
