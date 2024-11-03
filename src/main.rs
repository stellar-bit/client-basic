use clap::{Command, Arg};
use stellar_bit_client::{run, run_headless};
use std::path::PathBuf;

fn main() {
    let matches = Command::new("Stellar Bit Client")
        .version("1.0")
        .author("Your Name")
        .about("A client for Stellar Bit")
        .arg(
            Arg::new("join")
                .long("join")
                .value_name("ADDRESS")
                .help("Server address to join. Format: \"address access_token user_id\"")
        )
        .arg(
            Arg::new("headless")
                .long("headless")
                .action(clap::ArgAction::SetTrue)
                .help("Run in headless mode without display")
        )
        .arg(
            Arg::new("computer")
                .long("computer")
                .value_name("PATH")
                .help("Path to the computer unit file")
                .requires("headless")
        )
        .get_matches();

    if matches.get_flag("headless") {
        if let Some(computer_path) = matches.get_one::<String>("computer") {
            if let Some(join_data) = matches.get_one::<String>("join") {
                let mut join_parts = join_data.split_whitespace();
                let addr = join_parts.next().unwrap_or("");
                let token = join_parts.next().unwrap_or("");
                let user_id = join_parts.next().unwrap_or("0").parse().unwrap_or(0);
                
                run_headless(addr.to_string(), token.to_string(), user_id, PathBuf::from(computer_path));
            } else {
                eprintln!("Error: Headless mode requires --join argument");
            }
        } else {
            eprintln!("Error: Headless mode requires --computer argument");
        }
    } else {
        run();
    }
}
