use crate::constants::RAYDIUM_LAUNCHPAD_PROGRAM;
use futures::stream::StreamExt;
use log::{error, info, warn};
use std::collections::HashMap;
use std::fmt;
use tonic::Status;
use yellowstone_grpc_proto::geyser::SubscribeUpdate;
use yellowstone_grpc_proto::prelude::SubscribeUpdatePing;
use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;
const TARGET_IX_TYPES: &[RaydiumInstructionType] = &[
    RaydiumInstructionType::Initialize,
    RaydiumInstructionType::MigrateToAmm,
];

pub async fn process_updates<S>(mut stream: S) -> Result<(), Box<dyn std::error::Error>>
where
    S: StreamExt<Item = Result<SubscribeUpdate, Status>> + Unpin,
{
    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => handle_message(msg)?,
            Err(e) => {
                error!("Error receiving message: {:?}", e);
                break;
            }
        }
    }
    Ok(())
}

fn handle_message(msg: SubscribeUpdate) -> Result<(), Box<dyn std::error::Error>> {
    match msg.update_oneof {
        Some(UpdateOneof::Transaction(transaction_update)) => {
            match TransactionParser::parse_transaction(&transaction_update) {
                Ok(parsed_tx) => {
                    let mut has_raydium_ix = false;
                    let mut found_target_ix = false;
                    let mut found_ix_types = Vec::new();

                    if TARGET_IX_TYPES.is_empty() {
                        found_target_ix = true;
                    }

                    for (i, ix) in parsed_tx.instructions.iter().enumerate() {
                        if ix.program_id == RAYDIUM_LAUNCHPAD_PROGRAM {
                            has_raydium_ix = true;
                            let raydium_ix_type = parse_raydium_instruction_type(&ix.data);
                            found_ix_types.push(raydium_ix_type.clone());

                            if TARGET_IX_TYPES.contains(&raydium_ix_type) {
                                found_target_ix = true;
                            }
                            if found_target_ix {
                                info!(
                                    "Found target instruction: {} at index {}",
                                    raydium_ix_type, i
                                );
                            }
                        }
                    }

                    for inner_ix_group in &parsed_tx.inner_instructions {
                        for (i, inner_ix) in inner_ix_group.instructions.iter().enumerate() {
                            if inner_ix.program_id == RAYDIUM_LAUNCHPAD_PROGRAM {
                                has_raydium_ix = true;
                                let raydium_ix_type =
                                    parse_raydium_instruction_type(&inner_ix.data);
                                found_ix_types.push(raydium_ix_type.clone());

                                if TARGET_IX_TYPES.contains(&raydium_ix_type) {
                                    found_target_ix = true;
                                }
                                if found_target_ix
                                    && !matches!(
                                        raydium_ix_type,
                                        RaydiumInstructionType::Unknown(_)
                                    )
                                {
                                    info!(
                                        "Found target instruction: {} at inner index {}.{}",
                                        raydium_ix_type, inner_ix_group.instruction_index, i
                                    );
                                }
                            }
                        }
                    }

                    if found_target_ix && has_raydium_ix {
                        info!("Found Raydium Launchpad transaction!");
                        info!("Parsed Transaction:\n{}", parsed_tx);
                    }
                }
                Err(e) => {
                    error!("Failed to parse transaction: {:?}", e);
                }
            }
        }
        Some(UpdateOneof::Ping(SubscribeUpdatePing {})) => {
            // Ignore pings
        }
        Some(other) => {
            info!("Unexpected update received. Type of update: {:?}", other);
        }
        None => {
            warn!("Empty update received");
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum RaydiumInstructionType {
    Initialize,
    BuyExactIn,
    BuyExactOut,
    SellExactIn,
    SellExactOut,
    ClaimPlatformFee,
    ClaimVestedToken,
    CollectFee,
    CollectMigrateFee,
    CreateConfig,
    CreatePlatformConfig,
    CreateVestingAccount,
    MigrateToAmm,
    MigrateToCpswap,
    UpdateConfig,
    UpdatePlatformConfig,
    Unknown([u8; 8]),
}

pub fn parse_raydium_instruction_type(data: &[u8]) -> RaydiumInstructionType {
    if data.len() < 8 {
        return RaydiumInstructionType::Unknown([0; 8]);
    }

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&data[0..8]);

    match discriminator {
        [175, 175, 109, 31, 13, 152, 155, 237] => RaydiumInstructionType::Initialize,
        [250, 234, 13, 123, 213, 156, 19, 236] => RaydiumInstructionType::BuyExactIn,
        [24, 211, 116, 40, 105, 3, 153, 56] => RaydiumInstructionType::BuyExactOut,
        [149, 39, 222, 155, 211, 124, 152, 26] => RaydiumInstructionType::SellExactIn,
        [95, 200, 71, 34, 8, 9, 11, 166] => RaydiumInstructionType::SellExactOut,
        [156, 39, 208, 135, 76, 237, 61, 72] => RaydiumInstructionType::ClaimPlatformFee,
        [49, 33, 104, 30, 189, 157, 79, 35] => RaydiumInstructionType::ClaimVestedToken,
        [60, 173, 247, 103, 4, 93, 130, 48] => RaydiumInstructionType::CollectFee,
        [255, 186, 150, 223, 235, 118, 201, 186] => RaydiumInstructionType::CollectMigrateFee,
        [201, 207, 243, 114, 75, 111, 47, 189] => RaydiumInstructionType::CreateConfig,
        [176, 90, 196, 175, 253, 113, 220, 20] => RaydiumInstructionType::CreatePlatformConfig,
        [129, 178, 2, 13, 217, 172, 230, 218] => RaydiumInstructionType::CreateVestingAccount,
        [207, 82, 192, 145, 254, 207, 145, 223] => RaydiumInstructionType::MigrateToAmm,
        [136, 92, 200, 103, 28, 218, 144, 140] => RaydiumInstructionType::MigrateToCpswap,
        [29, 158, 252, 191, 10, 83, 219, 99] => RaydiumInstructionType::UpdateConfig,
        [195, 60, 76, 129, 146, 45, 67, 143] => RaydiumInstructionType::UpdatePlatformConfig,
        _ => RaydiumInstructionType::Unknown(discriminator),
    }
}

impl std::fmt::Display for RaydiumInstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaydiumInstructionType::Initialize => write!(f, "Initialize"),
            RaydiumInstructionType::BuyExactIn => write!(f, "BuyExactIn"),
            RaydiumInstructionType::BuyExactOut => write!(f, "BuyExactOut"),
            RaydiumInstructionType::SellExactIn => write!(f, "SellExactIn"),
            RaydiumInstructionType::SellExactOut => write!(f, "SellExactOut"),
            RaydiumInstructionType::ClaimPlatformFee => write!(f, "ClaimPlatformFee"),
            RaydiumInstructionType::ClaimVestedToken => write!(f, "ClaimVestedToken"),
            RaydiumInstructionType::CollectFee => write!(f, "CollectFee"),
            RaydiumInstructionType::CollectMigrateFee => write!(f, "CollectMigrateFee"),
            RaydiumInstructionType::CreateConfig => write!(f, "CreateConfig"),
            RaydiumInstructionType::CreatePlatformConfig => write!(f, "CreatePlatformConfig"),
            RaydiumInstructionType::CreateVestingAccount => write!(f, "CreateVestingAccount"),
            RaydiumInstructionType::MigrateToAmm => write!(f, "MigrateToAmm"),
            RaydiumInstructionType::MigrateToCpswap => write!(f, "MigrateToCpswap"),
            RaydiumInstructionType::UpdateConfig => write!(f, "UpdateConfig"),
            RaydiumInstructionType::UpdatePlatformConfig => write!(f, "UpdatePlatformConfig"),
            RaydiumInstructionType::Unknown(discriminator) => {
                write!(f, "Unknown(discriminator={:?})", discriminator)
            }
        }
    }
}

#[derive(Debug, Default)]
struct ParsedTransaction {
    signature: String,
    is_vote: bool,
    account_keys: Vec<String>,
    recent_blockhash: String,
    instructions: Vec<ParsedInstruction>,
    success: bool,
    fee: u64,
    pre_token_balances: Vec<ParsedTokenBalance>,
    post_token_balances: Vec<ParsedTokenBalance>,
    logs: Vec<String>,
    inner_instructions: Vec<ParsedInnerInstruction>,
    slot: u64,
}

impl fmt::Display for ParsedTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Transaction: {}", self.signature)?;
        writeln!(
            f,
            "Status: {}",
            if self.success { "Success" } else { "Failed" }
        )?;
        writeln!(f, "Slot: {}", self.slot)?;
        writeln!(f, "Fee: {} lamports", self.fee)?;

        writeln!(f, "\nAccount Keys:")?;
        for (i, key) in self.account_keys.iter().enumerate() {
            writeln!(f, "  [{}] {}", i, key)?;
        }

        writeln!(f, "\nInstructions:")?;
        for (i, ix) in self.instructions.iter().enumerate() {
            writeln!(f, "  Instruction {}:", i)?;
            writeln!(
                f,
                "    Program: {} (index: {})",
                ix.program_id, ix.program_id_index
            )?;
            writeln!(f, "    Accounts:")?;
            for (idx, acc) in &ix.accounts {
                writeln!(f, "      [{}] {}", idx, acc)?;
            }
            writeln!(f, "    Data: {} bytes", ix.data.len())?;
        }

        if !self.inner_instructions.is_empty() {
            writeln!(f, "\nInner Instructions:")?;
            for inner_ix in &self.inner_instructions {
                writeln!(f, "  Instruction Index: {}", inner_ix.instruction_index)?;
                for (i, ix) in inner_ix.instructions.iter().enumerate() {
                    writeln!(f, "    Inner Instruction {}:", i)?;
                    writeln!(
                        f,
                        "      Program: {} (index: {})",
                        ix.program_id, ix.program_id_index
                    )?;
                    writeln!(f, "      Accounts:")?;
                    for (idx, acc) in &ix.accounts {
                        writeln!(f, "        [{}] {}", idx, acc)?;
                    }
                    writeln!(f, "      Data: {} bytes", ix.data.len())?;
                }
            }
        }

        if !self.pre_token_balances.is_empty() || !self.post_token_balances.is_empty() {
            writeln!(f, "\nToken Balances:")?;

            let mut balance_changes = HashMap::new();

            for balance in &self.pre_token_balances {
                let key = (balance.account_index, balance.mint.clone());
                balance_changes.insert(key, (balance.amount.clone(), "".to_string()));
            }

            for balance in &self.post_token_balances {
                let key = (balance.account_index, balance.mint.clone());

                if let Some((_, post)) = balance_changes.get_mut(&key) {
                    *post = balance.amount.clone();
                } else {
                    balance_changes.insert(key, ("".to_string(), balance.amount.clone()));
                }
            }

            for ((account_idx, mint), (pre_amount, post_amount)) in balance_changes {
                let account_key = if (account_idx as usize) < self.account_keys.len() {
                    &self.account_keys[account_idx as usize]
                } else {
                    "unknown"
                };

                if pre_amount.is_empty() {
                    writeln!(
                        f,
                        "  Account {} ({}): new balance {} (mint: {})",
                        account_idx, account_key, post_amount, mint
                    )?;
                } else if post_amount.is_empty() {
                    writeln!(
                        f,
                        "  Account {} ({}): removed balance {} (mint: {})",
                        account_idx, account_key, pre_amount, mint
                    )?;
                } else {
                    writeln!(
                        f,
                        "  Account {} ({}): {} → {} (mint: {})",
                        account_idx, account_key, pre_amount, post_amount, mint
                    )?;
                }
            }
        }

        if !self.logs.is_empty() {
            writeln!(f, "\nTransaction Logs:")?;
            for (i, log) in self.logs.iter().enumerate() {
                writeln!(f, "  [{}] {}", i, log)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ParsedInstruction {
    program_id: String,
    program_id_index: u8,
    accounts: Vec<(usize, String)>, // (index, pubkey)
    data: Vec<u8>,
}

#[derive(Debug)]
struct ParsedInnerInstruction {
    instruction_index: u8,
    instructions: Vec<ParsedInstruction>,
}

#[derive(Debug)]
struct ParsedTokenBalance {
    account_index: u32,
    mint: String,
    owner: String,
    amount: String,
}

struct TransactionParser;

impl TransactionParser {
    pub fn parse_transaction(
        tx_update: &yellowstone_grpc_proto::geyser::SubscribeUpdateTransaction,
    ) -> Result<ParsedTransaction, Box<dyn std::error::Error>> {
        let mut parsed_tx = ParsedTransaction::default();
        parsed_tx.slot = tx_update.slot;

        if let Some(tx_info) = &tx_update.transaction {
            parsed_tx.is_vote = tx_info.is_vote;

            parsed_tx.signature = bs58::encode(&tx_info.signature).into_string();

            if let Some(tx) = &tx_info.transaction {
                if let Some(msg) = &tx.message {
                    for key in &msg.account_keys {
                        parsed_tx.account_keys.push(bs58::encode(key).into_string());
                    }

                    if let Some(meta) = &tx_info.meta {
                        for addr in &meta.loaded_writable_addresses {
                            let base58_addr = bs58::encode(addr).into_string();
                            parsed_tx.account_keys.push(base58_addr);
                        }

                        for addr in &meta.loaded_readonly_addresses {
                            let base58_addr = bs58::encode(addr).into_string();
                            parsed_tx.account_keys.push(base58_addr);
                        }
                    }

                    parsed_tx.recent_blockhash = bs58::encode(&msg.recent_blockhash).into_string();

                    for ix in &msg.instructions {
                        let program_id_index = ix.program_id_index;
                        let program_id =
                            if (program_id_index as usize) < parsed_tx.account_keys.len() {
                                parsed_tx.account_keys[program_id_index as usize].clone()
                            } else {
                                "unknown".to_string()
                            };

                        let mut accounts = Vec::new();
                        for &acc_idx in &ix.accounts {
                            let account_idx = acc_idx as usize;
                            if account_idx < parsed_tx.account_keys.len() {
                                accounts.push((
                                    account_idx,
                                    parsed_tx.account_keys[account_idx].clone(),
                                ));
                            }
                        }

                        parsed_tx.instructions.push(ParsedInstruction {
                            program_id,
                            program_id_index: program_id_index as u8,
                            accounts,
                            data: ix.data.clone(),
                        });
                    }
                }
            }

            if let Some(meta) = &tx_info.meta {
                parsed_tx.success = meta.err.is_none();
                parsed_tx.fee = meta.fee;

                for balance in &meta.pre_token_balances {
                    if let Some(amount) = &balance.ui_token_amount {
                        parsed_tx.pre_token_balances.push(ParsedTokenBalance {
                            account_index: balance.account_index,
                            mint: balance.mint.clone(),
                            owner: balance.owner.clone(),
                            amount: amount.ui_amount_string.clone(),
                        });
                    }
                }

                for balance in &meta.post_token_balances {
                    if let Some(amount) = &balance.ui_token_amount {
                        parsed_tx.post_token_balances.push(ParsedTokenBalance {
                            account_index: balance.account_index,
                            mint: balance.mint.clone(),
                            owner: balance.owner.clone(),
                            amount: amount.ui_amount_string.clone(),
                        });
                    }
                }

                for inner_ix in &meta.inner_instructions {
                    let mut parsed_inner_ixs = Vec::new();

                    for ix in &inner_ix.instructions {
                        let program_id_index = ix.program_id_index;

                        let program_id =
                            if (program_id_index as usize) < parsed_tx.account_keys.len() {
                                parsed_tx.account_keys[program_id_index as usize].clone()
                            } else {
                                "unknown".to_string()
                            };

                        let mut accounts = Vec::new();
                        for &acc_idx in &ix.accounts {
                            let account_idx = acc_idx as usize;
                            if account_idx < parsed_tx.account_keys.len() {
                                accounts.push((
                                    account_idx,
                                    parsed_tx.account_keys[account_idx].clone(),
                                ));
                            }
                        }

                        parsed_inner_ixs.push(ParsedInstruction {
                            program_id,
                            program_id_index: program_id_index as u8,
                            accounts,
                            data: ix.data.clone(),
                        });
                    }

                    parsed_tx.inner_instructions.push(ParsedInnerInstruction {
                        instruction_index: inner_ix.index as u8,
                        instructions: parsed_inner_ixs,
                    });
                }

                parsed_tx.logs = meta.log_messages.clone();
            }
        }

        Ok(parsed_tx)
    }
}
