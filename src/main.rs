#![feature(rustc_private)]

#[tokio::main]
async fn main() {
    minira::run().await;
}
