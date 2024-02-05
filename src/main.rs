fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // your async code here
        stellar_bit_client_basic::run().await;
    });
}
