use hello_rs::hello::test_app::HelloProxy;
use tars_core::{Context, Result};
use tars_rpc::Communicator;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let endpoint = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tcp -h 127.0.0.1 -p 10987".to_string());
    let obj_name = format!("Test.HelloServer.HelloObj@{}", endpoint);

    let comm = Communicator::new();
    let proxy = comm.string_to_proxy(&obj_name)?;
    let client = HelloProxy::new(proxy);

    for (no, name) in [(1, "alice"), (2, "bob"), (42, "world")] {
        let reply = client
            .hello(Context::new(), no, name.to_string())
            .await?;
        println!("hello({}, {:?}) -> {:?}", no, name, reply);
    }

    Ok(())
}
