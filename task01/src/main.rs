use anyhow::{Result, Context, anyhow};
use serde::Deserialize;
use serde_yaml;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::fs;
use std::str::FromStr;
use std::sync::Arc;
use tokio::task::JoinSet;

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

#[derive(Debug, Deserialize)]
struct Config {
    wallets: Vec<String>,
}

fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL
}

async fn get_wallet_balance(rpc_client: Arc<RpcClient>, wallet_address: String) -> Result<(String, f64)> {
    let pubkey = Pubkey::from_str(&wallet_address)
        .with_context(|| format!("Failed to parse wallet address: {}", wallet_address))?;

    let balance_lamports = rpc_client.get_balance(&pubkey)
        .with_context(|| format!("Failed to get balance for wallet: {}", wallet_address))?;

    let balance_sol = lamports_to_sol(balance_lamports);
    Ok((wallet_address, balance_sol))
}

async fn get_wallet_balances(config: &Config) -> Result<Vec<(String, f64)>> {
    let rpc_client = Arc::new(RpcClient::new("https://api.mainnet-beta.solana.com"));
    let mut tasks = JoinSet::new();

    for wallet in &config.wallets {
        tasks.spawn(get_wallet_balance(rpc_client.clone(), wallet.clone()));
    }

    let mut balances = Vec::new();
    let mut errors = Vec::new();

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(balance)) => balances.push(balance),
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(anyhow!("Task failed: {}", e)),
        }
    }

    if !errors.is_empty() {
        let error_msg = errors
            .into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(anyhow!("Failed to get some wallet balances:\n{}", error_msg));
    }

    Ok(balances)
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_content = fs::read_to_string("config.yaml")?;
    let config: Config = serde_yaml::from_str(&config_content)?;

    let balances = get_wallet_balances(&config).await?;

    for (wallet, sol) in balances {
        println!("Wallet: {} - Balance: {:.9} SOL", wallet, sol);
    }

    Ok(())
}
