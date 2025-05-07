use anyhow::{Result, Context, anyhow};
use serde::Deserialize;
use serde_yaml;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use std::{
    fs,
    str::FromStr,
    sync::Arc,
    time::Instant,
};
use tokio::task::JoinSet;
use hex;
use serde_json;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[derive(Debug, Deserialize)]
struct Sender {
    key: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    senders: Vec<Sender>,
    recipients: Vec<String>,
    amount_sol: f64,
}

#[derive(Debug)]
struct TransferResult {
    sender: String,
    recipient: String,
    signature: String,
    status: String,
    time_taken: f64,
}

fn create_keypair(private_key: &str) -> Result<Keypair> {
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

    Err(anyhow!("Invalid private key format. Expected base58 string, JSON array, or hex string"))
}

async fn send_sol(
    rpc_client: Arc<RpcClient>,
    sender_key: String,
    recipient: String,
    amount_lamports: u64,
) -> Result<TransferResult> {
    let start_time = Instant::now();
    
    let keypair = create_keypair(&sender_key)
        .with_context(|| format!("Failed to create keypair for sender"))?;
    
    let recipient_pubkey = Pubkey::from_str(&recipient)
        .with_context(|| format!("Failed to parse recipient address: {}", recipient))?;
    
    let balance = rpc_client
        .get_balance(&keypair.pubkey())
        .with_context(|| format!("Failed to get balance for sender {}", keypair.pubkey()))?;
    
    if balance < amount_lamports {
        return Err(anyhow!(
            "Insufficient balance for sender {}. Required: {} SOL, Available: {} SOL",
            keypair.pubkey(),
            amount_lamports as f64 / LAMPORTS_PER_SOL as f64,
            balance as f64 / LAMPORTS_PER_SOL as f64
        ));
    }
    
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
    
    let signature = match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(sig) => sig,
        Err(e) => {
            if let Some(err) = e.get_transaction_error() {
                return Err(anyhow!(
                    "Transaction failed for sender {} to recipient {}: {}",
                    keypair.pubkey(),
                    recipient,
                    err
                ));
            }
            return Err(anyhow!(
                "Failed to send transaction from {} to {}: {}",
                keypair.pubkey(),
                recipient,
                e
            ));
        }
    };
    
    let status = match rpc_client.get_signature_status(&signature)? {
        Some(status) => {
            if let Some(err) = status.err() {
                format!("Failed: {}", err)
            } else {
                "Success".to_string()
            }
        }
        None => "Unknown".to_string(),
    };
    
    let time_taken = start_time.elapsed().as_secs_f64();
    
    Ok(TransferResult {
        sender: keypair.pubkey().to_string(),
        recipient,
        signature: signature.to_string(),
        status,
        time_taken,
    })
}

async fn process_transfers(config: &Config) -> Result<Vec<TransferResult>> {
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        "https://api.mainnet-beta.solana.com",
        CommitmentConfig::confirmed(),
    ));
    
    let amount_lamports = (config.amount_sol * LAMPORTS_PER_SOL as f64) as u64;
    let mut tasks = JoinSet::new();
    
    for sender in &config.senders {
        for recipient in &config.recipients {
            tasks.spawn(send_sol(
                rpc_client.clone(),
                sender.key.clone(),
                recipient.clone(),
                amount_lamports,
            ));
        }
    }
    
    let mut results = Vec::new();
    let mut errors = Vec::new();
    
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(transfer_result)) => {
                if transfer_result.status == "Success" {
                    results.push(transfer_result);
                } else {
                    errors.push(anyhow!(
                        "Transfer failed: From {} to {} - {}",
                        transfer_result.sender,
                        transfer_result.recipient,
                        transfer_result.status
                    ));
                }
            }
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(anyhow!("Task failed: {}", e)),
        }
    }
    
    println!("\nTransfer Summary:");
    println!("=================");
    println!("Successful transfers: {}", results.len());
    if !errors.is_empty() {
        println!("Failed transfers: {}", errors.len());
        println!("\nFailed transfers details:");
        for error in &errors {
            println!("- {}", error);
        }
    }
    
    Ok(results)
}

#[tokio::main]
async fn main() -> Result<()> {
    let start_time = Instant::now();
    
    let config_content = fs::read_to_string("config.yaml")?;
    let config: Config = serde_yaml::from_str(&config_content)?;
    
    let results = process_transfers(&config).await?;
    
    if !results.is_emty() {
        println!("\nSuccessful Transfer Details:");
        println!("=========================");
        for result in &results {
            println!(
                "From: {}\nTo: {}\nSignature: {}\nTime: {:.3}s\n",
                result.sender,
                result.recipient,
                result.signature,
                result.time_taken
            );
        }
    }
    
    let total_time = start_time.elapsed().as_secs_f64();
    println!("\nTotal processing time: {:.3}s", total_time);
    
    Ok(())
}
