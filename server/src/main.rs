mod mongo;

use service_http::run;
use mongo::IOUServiceDB;
use tokio;

#[tokio::main]
async fn main() {
   let mongo = IOUServiceDB::init().await;
   run().await;
}

