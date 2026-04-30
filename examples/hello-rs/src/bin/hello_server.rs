use async_trait::async_trait;
use hello_rs::hello::test_app::{Hello, HelloServant};
use tars_core::{Context, Result};
use tars_rpc::Application;

struct HelloImpl;

#[async_trait]
impl Hello for HelloImpl {
    async fn hello(&self, _ctx: &Context, no: i32, name: String) -> Result<String> {
        Ok(format!("hello#{} {}", no, name))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:10987".to_string());
    let obj = "Test.HelloServer.HelloObj";

    let app = Application::new();
    app.add_servant(obj, HelloServant::new(HelloImpl), &addr)?;
    println!("hello-server: {} listening at tcp://{}", obj, addr);

    app.run().await?;
    Ok(())
}
