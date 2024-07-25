pub mod mongo;
pub mod routes;
use service_http::run;
use tokio;

#[tokio::main]
async fn main() {
   run().await;
}

