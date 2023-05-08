// //! Basic example that shows how to initialize and update a smart contract.
// //!
// //! In particular, it uses the "weather" contract which is part of the
// //! [icecream example contract](https://github.com/Concordium/concordium-rust-smart-contracts/blob/main/examples/icecream/src/lib.rs).

// mod params;
// use anyhow::Context;
// use concordium_rust_sdk::{
//     common::types::TransactionTime,
//     endpoints,
//     smart_contracts::common::{Amount, ContractAddress, OwnedReceiveName},
//     types::{
//         smart_contracts::OwnedParameter,
//         transactions::{send, BlockItem, UpdateContractPayload},
//         AccountInfo, Address, WalletAccount,
//     },
// };

// use params::{MintParams, TokenMetadata};

// use std::collections::BTreeMap;

// #[tokio::main(flavor = "multi_thread")]
// async fn main() -> anyhow::Result<()> {
//     let mut client = endpoints::Client::connect("http://localhost:10003", "rpcadmin").await?;
//     // load account keys and sender address from a file
//     let keys: WalletAccount =
//         WalletAccount::from_json_file("/home/parv0888/Documents/concordium-testnet-wallet-export/3VQCZrqCGsUnKD4DXSWDu1ynsKqfRrrfF7cN51KszryYkHytt8.export").context("Could not parse the keys file.")?;

//     let consensus_info = client.get_consensus_status().await?;
//     // Get the initial nonce at the last finalized block.
//     let acc_info: AccountInfo = client
//         .get_account_info(&keys.address, &consensus_info.last_finalized_block)
//         .await?;
//     let nonce = acc_info.account_nonce;
//     // set expiry to now + 5min
//     let expiry: TransactionTime =
//         TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);

//     let tx = {
//         let mut tokens = BTreeMap::new();
//         tokens.insert(
//             "04".to_string().try_into().unwrap(),
//             (
//                 TokenMetadata {
//                     url: "example.com".to_string(),
//                     hash: "6a4b4967916c4e9d7c1cd7d37997c11d3c22853e63d435b0e6dd7a501b1ee685"
//                         .to_string(),
//                 },
//                 1u64.into(),
//             ),
//         );
//         let message = OwnedParameter::from_serial(
//             &MintParams::new(Address::Account(keys.address), tokens).unwrap(),
//         )
//         .expect("Known to not exceed parameter size limit.");
//         let payload = UpdateContractPayload {
//             amount: Amount::zero(),
//             address: ContractAddress {
//                 index: 4288,
//                 subindex: 0,
//             },
//             receive_name: OwnedReceiveName::new_unchecked("CIS2-Multi.mint".to_string()),
//             message,
//         };

//         send::update_contract(&keys, keys.address, nonce, expiry, payload, 10000u64.into())
//     };

//     let item = BlockItem::AccountTransaction(tx);
//     // submit the transaction to the chain
//     let transaction_hash = client.send_block_item(&item).await?;
//     println!(
//         "Transaction {} submitted (nonce = {}).",
//         transaction_hash, nonce,
//     );
//     let (bh, bs) = client.wait_until_finalized(&transaction_hash).await?;
//     println!("Transaction finalized in block {}.", bh);
//     println!("The outcome is {:#?}", bs);

//     Ok(())
// }
