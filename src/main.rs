use clap::{Command, Arg};
use stellar_bit_client::{run, run_headless_direct, run_headless_hub};
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
                .help("Direct server connection string (format: \"address access_token user_id\")")
                .conflicts_with_all(["server-id", "username", "password"])
        )
        .arg(
            Arg::new("server-id")
                .long("server-id")
                .value_name("ID")
                .help("Server ID to join through central hub")
                .value_parser(clap::value_parser!(i64))
                .requires_all(["username", "password"])
        )
        .arg(
            Arg::new("username")
                .long("username")
                .value_name("USERNAME")
                .help("Central hub username")
        )
        .arg(
            Arg::new("password")
                .long("password")
                .value_name("PASSWORD")
                .help("Central hub password")
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
            .expect("Computer path is required for headless mode");

        if let Some(join_data) = matches.get_one::<String>("join") {
            // Direct connection mode
            let mut parts = join_data.split_whitespace();
            let addr = parts.next().expect("Missing address in join string");
            let token = parts.next().expect("Missing token in join string");
            let user_id = parts.next()
                .expect("Missing user_id in join string")
                .parse::<i64>()
                .expect("Invalid user_id format");
            
            run_headless_direct(
                addr.to_string(),
                token.to_string(),
                user_id,
                PathBuf::from(computer_path)
            );
        } else {
            // Central hub connection mode
            let server_id = matches.get_one::<i64>("server-id")
                .expect("Either --join or --server-id is required for headless mode");
            let username = matches.get_one::<String>("username")
                .expect("Username is required for central hub connection").to_owned();
            let password = matches.get_one::<String>("password")
                .expect("Password is required for central hub connection").to_owned();
            
            run_headless_hub(
                *server_id,
                username,
                password,
                PathBuf::from(computer_path)
            );
        }
    } else {
        run();
    }
}
