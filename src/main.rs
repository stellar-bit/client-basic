use clap::{Command, Arg};
use stellar_bit_client::{run, run_headless};
use std::path::PathBuf;

fn main() {
    let matches = Command::new("Stellar Bit Client")
        .version("1.0")
        .author("Your Name")
        .about("A client for Stellar Bit")
        .arg(
            Arg::new("server-id")
                .long("server-id")
                .value_name("ID")
                .help("Server ID to join")
                .value_parser(clap::value_parser!(i64))
                .requires("headless")
        )
        .arg(
            Arg::new("username")
                .long("username")
                .value_name("USERNAME")
                .help("Central hub username")
                .requires("headless")
        )
        .arg(
            Arg::new("password")
                .long("password")
                .value_name("PASSWORD")
                .help("Central hub password")
                .requires("headless")
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
        let computer_path = matches.get_one::<String>("computer")
            .expect("Computer path is required for headless mode").to_owned();
        let server_id = matches.get_one::<i64>("server-id")
            .expect("Server ID is required for headless mode");
        let username = matches.get_one::<String>("username")
            .expect("Username is required for headless mode").to_owned();
        let password = matches.get_one::<String>("password")
            .expect("Password is required for headless mode").to_owned();
        
        run_headless(
            *server_id,
            username,
            password,
            PathBuf::from(computer_path)
        );
    } else {
        run();
    }
}
