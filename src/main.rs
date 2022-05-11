extern crate log;

use clap::Parser;
use dns_agent::config::Config;
use dns_agent::update;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::fs;

/// Updates DNS records using interface IPs and external IP discovery services
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[clap(short, long, default_value = "/etc/dns-agent/config.toml")]
    configuration: String,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    let log_level = match args.verbose {
        true => LevelFilter::Info,
        false => LevelFilter::Warn,
    };
    SimpleLogger::new().with_level(log_level).init().unwrap();
    let raw_config = fs::read_to_string(args.configuration).expect("Error loading configuration");
    let config: Config = toml::from_str(&raw_config).unwrap();
    update::update_dns(config);
}
