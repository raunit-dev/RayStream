use std::env;
use tonic::transport::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use dotenv::dotenv;
use tonic::service::Interceptor;

pub async fn setup_client() -> Result<GeyserGrpcClient<impl Interceptor>, Box<dyn std::error::Error>> {
    
    dotenv().ok();
    let endpoint = env::var("ENDPOINT").expect("ENDPOINT");
    let token = env::var("TOKEN").expect("TOKEN");

    let client = GeyserGrpcClient::build_from_shared(endpoint.to_string())?
          .x_token(Some(token.to_string()))?
          .tls_config(ClientTlsConfig::new().with_native_roots())?
          .connect()
          .await?;

        Ok(client)
}
