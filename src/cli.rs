use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "rokf",
    version,
    about = "Create, inspect, and maintain Open Knowledge Format knowledge bundles"
)]
pub struct Cli;

pub fn run() {
    Cli::parse();
}
