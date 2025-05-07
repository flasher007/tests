use anyhow::{Context, Result};
use futures::{sink::SinkExt, stream::StreamExt};
use log::{error, info};
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::{fs, str::FromStr, sync::Arc, time::Duration};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    prelude::{
        subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest, SubscribeRequestPing,
        SubscribeRequestFilterBlocks, SubscribeRequestFilterSlots, UnixTimestamp,
    },
};

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[derive(Debug, Deserialize)]
struct Config {
    sender_key: String,
    recipient: String,
    amount_sol: f64,
    grpc_endpoint: String,
    grpc_api_key: String,
}

async fn create_keypair(private_key: &str) -> Result<Keypair> {
    if private_key.starts_with('[') && private_key.ends_with(']') {
        let bytes: Vec<u8> = serde_json::from_str(private_key)
            .with_context(|| format!("Failed to parse private key as JSON array: {}", private_key))?;
        return Ok(Keypair::from_bytes(&bytes)
            .with_context(|| "Failed to create keypair from bytes")?);
    }

    if let Ok(bytes) = hex::decode(private_key.trim_start_matches("0x")) {
        return Ok(Keypair::from_bytes(&bytes)
            .with_context(|| "Failed to create keypair from hex bytes")?);
    }

    if let Ok(keypair) = std::panic::catch_unwind(|| Keypair::from_base58_string(private_key)) {
        return Ok(keypair);
    }

    Err(anyhow::anyhow!("Invalid private key format. Expected base58 string, JSON array, or hex string"))
}

async fn send_sol(
    rpc_client: Arc<RpcClient>,
    sender_key: &str,
    recipient: &str,
    amount_lamports: u64,
) -> Result<String> {
    let keypair = create_keypair(sender_key).await?;
    let recipient_pubkey = Pubkey::from_str(recipient)
        .with_context(|| format!("Failed to parse recipient address: {}", recipient))?;

    info!("Sending {} SOL from {} to {}", 
        amount_lamports as f64 / LAMPORTS_PER_SOL as f64,
        keypair.pubkey(),
        recipient_pubkey
    );

    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .context("Failed to get recent blockhash")?;

    let transfer_instruction = system_instruction::transfer(
        &keypair.pubkey(),
        &recipient_pubkey,
        amount_lamports,
    );

    let mut transaction = Transaction::new_with_payer(
        &[transfer_instruction],
        Some(&keypair.pubkey()),
    );
    transaction.sign(&[&keypair], recent_blockhash);

    // Check sender's balance before sending
    let balance = rpc_client
        .get_balance(&keypair.pubkey())
        .context("Failed to get sender's balance")?;
    
    info!("Sender's balance: {} SOL", balance as f64 / LAMPORTS_PER_SOL as f64);
    
    if balance < amount_lamports {
        return Err(anyhow::anyhow!(
            "Insufficient balance. Required: {} SOL, Available: {} SOL",
            amount_lamports as f64 / LAMPORTS_PER_SOL as f64,
            balance as f64 / LAMPORTS_PER_SOL as f64
        ));
    }

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .context("Failed to send transaction")?;

    Ok(signature.to_string())
}

async fn subscribe_to_blocks(
    grpc_endpoint: String,
    api_key: String,
    rpc_client: Arc<RpcClient>,
    sender_key: String,
    recipient: String,
    amount_sol: f64,
) -> Result<()> {
    let mut client = GeyserGrpcClient::build_from_shared(grpc_endpoint)?
        .x_token(Some(api_key))?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(10))
        .max_decoding_message_size(1024 * 1024 * 1024)
        .connect()
        .await?;

    info!("Connected to Geyser GRPC");

    let mut blocks = std::collections::HashMap::new();
    blocks.insert("client".to_owned(), SubscribeRequestFilterBlocks {
        include_transactions: Some(true),
        include_accounts: Some(false),
        include_entries: Some(false),
        account_include: vec![],
    });

    let request = SubscribeRequest {
        accounts: std::collections::HashMap::default(),
        slots: std::collections::HashMap::default(),
        transactions: std::collections::HashMap::default(),
        transactions_status: std::collections::HashMap::default(),
        blocks,
        blocks_meta: std::collections::HashMap::default(),
        entry: std::collections::HashMap::default(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: Vec::default(),
        ping: None,
    };

    let (mut subscribe_tx, mut stream) = client.subscribe_with_request(Some(request)).await?;
    info!("Stream opened");

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => match msg.update_oneof {
                Some(UpdateOneof::Block(update)) => {
                    let timestamp = chrono::DateTime::from_timestamp(
                        update.block_time.unwrap_or(UnixTimestamp { timestamp: 0 }).timestamp,
                        0,
                    ).unwrap_or_default();
                    
                    info!(
                        "Received block: slot={}, timestamp={}, parent_slot={}, blockhash={}",
                        update.slot,
                        timestamp.format("%Y-%m-%d %H:%M:%S"),
                        update.parent_slot,
                        bs58::encode(&update.blockhash).into_string()
                    );
                    
                    // Send SOL transaction
                    let amount_lamports = (amount_sol * LAMPORTS_PER_SOL as f64) as u64;
                    match send_sol(
                        rpc_client.clone(),
                        &sender_key,
                        &recipient,
                        amount_lamports,
                    ).await {
                        Ok(signature) => info!("Transaction sent successfully. Signature: {}", signature),
                        Err(e) => error!("Failed to send transaction: {}", e),
                    }
                }
                Some(UpdateOneof::Ping(_)) => {
                    subscribe_tx
                        .send(SubscribeRequest {
                            ping: Some(SubscribeRequestPing { id: 1 }),
                            ..Default::default()
                        })
                        .await?;
                }
                Some(UpdateOneof::Pong(_)) => {}
                None => {
                    error!("Update not found in the message");
                    break;
                }
                _ => {}
            },
            Err(error) => {
                error!("Error: {:?}", error);
                break;
            }
        }
    }

    info!("Stream closed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var(
        env_logger::DEFAULT_FILTER_ENV,
        std::env::var_os(env_logger::DEFAULT_FILTER_ENV).unwrap_or_else(|| "info".into()),
    );
    env_logger::init();

    let config_content = fs::read_to_string("config.yaml")?;
    let config: Config = serde_yaml::from_str(&config_content)?;

    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        "https://api.mainnet-beta.solana.com",
        CommitmentConfig::confirmed(),
    ));

    subscribe_to_blocks(
        config.grpc_endpoint,
        config.grpc_api_key,
        rpc_client,
        config.sender_key,
        config.recipient,
        config.amount_sol,
    ).await?;

    Ok(())
} 