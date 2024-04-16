use std::collections::HashMap;

use anyhow::Result;
use futures::FutureExt;
use log::info;
use solana_sdk::pubkey::Pubkey;
use tokio::task::JoinSet;
use solana_client::rpc_client::RpcClient;
use MEV_Bot_Solana::arbitrage::strategies::{run_arbitrage_strategy, sorted_interesting_path_strategy};
use MEV_Bot_Solana::markets::pools::load_all_pools;
use MEV_Bot_Solana::transactions::create_transaction::{create_ata_extendlut_transaction, ChainType, SendOrSimulate};
use MEV_Bot_Solana::{common::constants::Env, transactions::create_transaction::create_and_send_swap_transaction};
use MEV_Bot_Solana::common::utils::{from_str, get_tokens_infos, setup_logger};
use MEV_Bot_Solana::arbitrage::types::{SwapPathResult, SwapRouteSimulation, TokenInArb, TokenInfos};
use rust_socketio::{Payload, asynchronous::{Client, ClientBuilder},};


// use MEV_Bot_Solana::common::pools::{load_all_pools, Pool};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    // log4rs::init_file("logging_config.yaml", Default::default()).unwrap();
    setup_logger().unwrap();

    info!("Starting MEV_Bot_Solana");
    info!("⚠️⚠️ New fresh pools fetched on METEORA and RAYDIUM are excluded because a lot of time there have very low liquidity, potentially can be used on subscribe log strategy");
    info!("⚠️⚠️ Liquidity is fetch to API and can be outdated on Radyium Pool");

    let mut set: JoinSet<()> = JoinSet::new();
    
    info!("🏊 Launch pools fetching infos...");
    let dexs = load_all_pools(false).await;
    info!("🏊 {} Dexs are loaded", dexs.len());
    
    // // The first token is the base token (here SOL)
    // let tokens_to_arb: Vec<TokenInArb> = vec![
    //     TokenInArb{address: String::from("So11111111111111111111111111111111111111112"), symbol: String::from("SOL")}, // Base token here
    //     TokenInArb{address: String::from("25hAyBQfoDhfWx9ay6rarbgvWGwDdNqcHsXS3jQ3mTDJ"), symbol: String::from("MANEKI")},
    //     TokenInArb{address: String::from("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"), symbol: String::from("JUP")},
    //     TokenInArb{address: String::from("EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm"), symbol: String::from("WIF")},
    // ];
    // The first token is the base token (here SOL)
    let tokens_to_arb: Vec<TokenInArb> = vec![
        TokenInArb{address: String::from("So11111111111111111111111111111111111111112"), symbol: String::from("SOL")}, // Base token here
        // TokenInArb{address: String::from("8NH3AfwkizHmbVd83SSxc2YbsFmFL4m2BeepvL6upump"), symbol: String::from("TOPG")},
        TokenInArb{address: String::from("4GJ3TCt5mTgQT5BRKb14AkjddpFQqKVfphxzS3t4foZ9"), symbol: String::from("jenner")},
    ];

    let tokens_infos: HashMap<String, TokenInfos> = get_tokens_infos(tokens_to_arb.clone()).await;

    info!("🪙🪙 Tokens Infos: {:?}", tokens_to_arb);
    info!("📈 Launch arbitrage process...");
    
    info!("Open Socket IO channel...");
    let env = Env::new();
    
    let callback = |payload: Payload, socket: Client| {
        async move {
            match payload {
                Payload::String(data) => println!("Received: {}", data),
                Payload::Binary(bin_data) => println!("Received bytes: {:#?}", bin_data),
                Payload::Text(data) => println!("Received Text: {:?}", data),
            }
        }
        .boxed()
    };
    
    let mut socket = ClientBuilder::new("http://localhost:3000")
        .namespace("/")
        .on("connection", callback)
        .on("error", |err, _| {
            async move { eprintln!("Error: {:#?}", err) }.boxed()
        })
        .on("orca_quote", callback)
        .on("orca_quote_res", callback)
        .connect()
        .await
        .expect("Connection failed");


    
    let swap_path_selected = run_arbitrage_strategy(socket, dexs, tokens_to_arb.clone(), tokens_infos.clone()).await;
    let _ = sorted_interesting_path_strategy(swap_path_selected, tokens_to_arb, tokens_infos).await;
    
        
    // let spr = SwapPathResult{ 
    //     path_id: 1,
    //     hops: 2,
    //     tokens_path: "SOL-AMC-GME-SOL".to_string(),
    //     route_simulations: vec![
    //         SwapRouteSimulation{
    //             id_route: 17,
    //             pool_address: "HZZofxusqKaA9JqaeXW8PtUALRXUwSLLwnt4eBFiyEdC".to_string(),
    //             dex_label: MEV_Bot_Solana::markets::types::DexLabel::RAYDIUM,
    //             token_0to1: false,
    //             token_in: "So11111111111111111111111111111111111111112".to_string(),
    //             token_out: "9jaZhJM6nMHTo4hY9DGabQ1HNuUWhJtm7js1fmKMVpkN".to_string(),
    //             amount_in: 300000000,
    //             // 8703355798604
    //             estimated_amount_out: "8703355798".to_string(),
    //             estimated_min_amount_out: "8617183959013".to_string()
    //         },
    //         SwapRouteSimulation{ 
    //             id_route: 26,
    //             pool_address: "9kbAydmdxuqrJGvaCmmnJaGnaC96zAkBHZ9dQn3cm9PZ".to_string(),
    //             dex_label: MEV_Bot_Solana::markets::types::DexLabel::METEORA,
    //             token_0to1: true,
    //             token_in: "9jaZhJM6nMHTo4hY9DGabQ1HNuUWhJtm7js1fmKMVpkN".to_string(),
    //             token_out: "8wXtPeU6557ETkp9WHFY1n1EcU6NxDvbAggHGsMYiHsB".to_string(),
    //             amount_in: 8703355798, // 0.001 SOL
    //             //4002500590682
    //             estimated_amount_out:"4002500".to_string(),
    //             estimated_min_amount_out: "3998498090091".to_string()
    //         },
    //         SwapRouteSimulation{ 
    //             id_route: 13,
    //             pool_address: "2qKjGUBdgLcGVt1JbjLfXtphPQNkq4ujd6PyrTBWkeJ5".to_string(),
    //             dex_label: MEV_Bot_Solana::markets::types::DexLabel::ORCA_WHIRLPOOLS,
    //             token_0to1: false,
    //             token_in: "8wXtPeU6557ETkp9WHFY1n1EcU6NxDvbAggHGsMYiHsB".to_string(),
    //             token_out: "So11111111111111111111111111111111111111112".to_string(),
    //             amount_in: 4002500, // 0.001 SOL
    //             estimated_amount_out:"300776562".to_string(),
    //             estimated_min_amount_out: "297798576".to_string()
    //         }
    //     ],
    //     token_in: "So11111111111111111111111111111111111111112".to_string(),
    //     token_in_symbol: "SOL".to_string(),
    //     token_out: "So11111111111111111111111111111111111111112".to_string(),
    //     token_out_symbol: "SOL".to_string(),
    //     amount_in: 300000000,
    //     estimated_amount_out: "300776562".to_string(),
    //     estimated_min_amount_out: "297798576".to_string(),
    //     result: 776562.0
    // };
    // 6nGymM5X1djYERKZtoZ3Yz3thChMVF6jVRDzhhcmxuee
    // let tokens: Vec<Pubkey> = tokens_to_arb.into_iter().map(|tok| from_str(tok.address.as_str()).unwrap()).collect();
    // let _ = create_ata_extendlut_transaction(
    //     ChainType::Mainnet,
    //     SendOrSimulate::Send,
    //     spr.clone(),
    //     from_str("6nGymM5X1djYERKZtoZ3Yz3thChMVF6jVRDzhhcmxuee").unwrap(),
    //     tokens
    // ).await;
    // let _ = create_and_send_swap_transaction(
    //     SendOrSimulate::Simulate,
    //     ChainType::Mainnet, 
    //     spr.clone()
    // ).await;
    
    while let Some(res) = set.join_next().await {
        info!("{:?}", res);
    }

    println!("End");
    Ok(())
}