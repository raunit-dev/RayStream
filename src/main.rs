use yellowstone_grpc_client::{GeyserGrpcClient, ClientTlsConfig};
use dotenv::dotenv;
use std::env;


fn main () {
    async fn setup_client() -> Result<GeyserGrpcClient<impl Interceptor>, Box<dyn std::Error>> {
    dotenv().ok();
    let endpoint = env::var("endpoint").expect("endpoint must be set");
    let token = env::var("token").expect("token must be set");

    let client = GeyserGrpcClient::build_from_shared(endpoint.to_string())?//takes endpoint as parameter here
        .x_token(Some(token.to_string()))?// Set x-token
        .tls_config(ClientTlsConfig::new().with_native_roots())?//req formation
        .connect()//req sending
        .await?;

    Ok(client)
}
}
