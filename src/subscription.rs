use std::collections::HashMap;
use futures::sink::SinkExt;
use yellowstone_grpc_proto::prelude::{
    CommitmentLevel,
    SubscribeRequest,
    SubscribeRequestFilterTransactions,
};
use crate::constants::RAYDIUM_LAUNCHPAD_PROGRAM;

pub async fn send_subscription_request<T>(
    mut tx: T,
) -> Result<(), Box<dyn std::error::Error>>
where 
    T: SinkExt<SubscribeRequest> + Unpin,
    <T as futures::Sink<SubscribeRequest>>::Error: std::error::Error + 'static,
    {

        let mut accounts_filter = HashMap::new();
        accounts_filter.insert(
            "account_monitor".to_string(),
            SubscribeRequestFilterTransactions {
                account_include: vec![],
                account_exclude: vec![],
                account_required: vec![
                    RAYDIUM_LAUNCHPAD_PROGRAM.to_string()
                ],
                vote: Some(false),
                failed: Some(false),
                signature: None,
            },
        );

        tx.send(SubscribeRequest {
            transactions: accounts_filter,
            commitment: Some(CommitmentLevel::Finalized as i32),
            ..Default::default()
        }).await?;

        Ok(())
    }

    