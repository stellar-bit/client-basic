use clap::{Command, Arg};

struct JoinServerArgs {
    join_address: Option<String>,
    access_token: Option<String>
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // your async code here
        stellar_bit_client::run().await;
    });
}
