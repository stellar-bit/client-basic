use clap::{Command, Arg};

struct JoinServerArgs {
    join_address: Option<String>,
    access_token: Option<String>
}

fn main() {
    stellar_bit_client::run();
}
