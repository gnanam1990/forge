use clap::Parser;

use forge::cli::{run, Cli};

fn main() {
    let cli = Cli::parse();
    std::process::exit(run(cli));
}
