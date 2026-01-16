use std::error::Error;

use clap::Parser;

use rust_digger::ElapsedTimer;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Limit the number of repos we process."
    )]
    limit: u32,
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let start_time = std::time::Instant::now();

    match run() {
        Ok(()) => {}
        Err(err) => log::error!("Error: {err}"),
    }

    log::info!("Elapsed time: {} sec.", start_time.elapsed().as_secs());
    log::info!("Ending the clone process");
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let _a = ElapsedTimer::new("feed.rs");
    log::info!(
        "Starting fetching the feed for up to {} crates.",
        args.limit
    );

    Ok(())
}
