mod mongo;

use service_http::run;
use tokio;

#[tokio::main]
async fn main() {
   run().await;
}

