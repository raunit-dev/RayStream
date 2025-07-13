use yellowstone_grpc_client::{GeyserGrpcClient, ClientTlsConfig};
use dotenv::dotenv;
use std::env;

async fn setup_client() -> Result<GeyserGrpcClient<impl Interceptor>, Box<dyn std::Error>> {
    dotenv().ok();
    let endpoint = env::var("endpoint").expect("endpoint must be set");
    let token = env::var("token").expect("token must be set");

    let client = GeyserGrpcClient::build_from_shared(endpoint.to_string())?
        .x_token(Some(token.to_string()))?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;

    Ok(client)
}