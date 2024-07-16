use reqwest;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};

#[derive(Deserialize, Serialize, Debug)]
struct ScriptPubKey {
    #[serde(default)]
    asm: Option<String>,
    #[serde(default)]
    hex: Option<String>,
    #[serde(rename = "type", default)]
    script_type: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Vout {
    #[serde(default)]
    value: Option<f64>,
    #[serde(default)]
    n: Option<u32>,
    #[serde(default)]
    script_pub_key: Option<ScriptPubKey>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ScriptSig {
    #[serde(default)]
    asm: Option<String>,
    #[serde(default)]
    hex: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Vin {
    #[serde(default)]
    coinbase: Option<String>,
    #[serde(default)]
    txid: Option<String>,
    #[serde(default)]
    vout: Option<u32>,
    #[serde(default)]
    script_sig: Option<ScriptSig>,
    #[serde(default)]
    sequence: Option<u64>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Transaction {
    hash: String,
    #[serde(default)]
    hex: Option<String>,
    #[serde(default)]
    vin: Vec<Vin>,
    #[serde(default)]
    vout: Vec<Vout>,
}

#[derive(Deserialize, Debug)]
struct Block {
    tx: Vec<Transaction>,
}

#[derive(Deserialize)]
struct ApiResponse {
    blocks: Vec<Block>,
}

async fn fetch_block_data(block_height: &str) -> Result<Block, Box<dyn std::error::Error>> {
    let url = format!("https://blockchain.info/block-height/{}?format=json", block_height);
    let resp = reqwest::get(&url).await?.json::<ApiResponse>().await?;
    let block = resp.blocks.into_iter().next().ok_or("No blocks found")?;
    Ok(block)
}

fn is_printable_ascii(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii() && !c.is_ascii_control())
}

fn extract_hidden_message(hex_data: &str) -> Option<String> {
    let data = hex::decode(hex_data).ok()?;
    let message = String::from_utf8_lossy(&data);
    if is_printable_ascii(&message) {
        Some(message.to_string())
    } else {
        None
    }
}

fn check_transaction_for_messages(tx: &Transaction) -> Vec<String> {
    let mut messages = Vec::new();

    // Check vin for coinbase and scriptSig
    for vin in &tx.vin {
        if let Some(coinbase) = &vin.coinbase {
            if let Some(message) = extract_hidden_message(coinbase) {
                messages.push(format!("Coinbase: {}", message));
            }
        }

        if let Some(script_sig) = &vin.script_sig {
            if let Some(hex) = &script_sig.hex {
                if let Some(message) = extract_hidden_message(hex) {
                    messages.push(format!("ScriptSig: {}", message));
                }
            }
        }
    }

    // Check vout for OP_RETURN and scriptPubKey
    for vout in &tx.vout {
        if let Some(script_pub_key) = &vout.script_pub_key {
            if let Some(hex) = &script_pub_key.hex {
                if script_pub_key.script_type.as_deref() == Some("nulldata") {
                    if let Some(message) = extract_hidden_message(hex) {
                        messages.push(format!("OP_RETURN: {}", message));
                    }
                } else {
                    if let Some(message) = extract_hidden_message(hex) {
                        messages.push(format!("ScriptPubKey: {}", message));
                    }
                }
            }
        }
    }

    messages
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: cargo run <block_height>");
        return;
    }
    let block_height = &args[1];

    match fetch_block_data(block_height).await {
        Ok(block) => {
            let tx_count = block.tx.len();
            println!("Block {} contains {} transactions.", block_height, tx_count);

            print!("Enter the transaction number (0 to {}): ", tx_count - 1);
            io::stdout().flush().unwrap();

            let mut tx_num = String::new();
            io::stdin().read_line(&mut tx_num).unwrap();
            let tx_num: usize = match tx_num.trim().parse() {
                Ok(num) if num < tx_count => num,
                _ => {
                    eprintln!("Invalid transaction number.");
                    return;
                }
            };

            let selected_tx = &block.tx[tx_num];
            let tx_json = serde_json::to_string_pretty(&selected_tx).unwrap();
            println!("Transaction details:\n{}", tx_json);

            let messages = check_transaction_for_messages(selected_tx);
            if !messages.is_empty() {
                println!("Hidden messages found:");
                for msg in messages {
                    println!("{}", msg);
                }
            } else {
                println!("No hidden messages found in this transaction.");
            }
        }
        Err(e) => eprintln!("Error fetching block data: {}", e),
    }
}
