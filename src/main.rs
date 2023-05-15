use crate::clap::AppSettings;
use anyhow::Context;
use concordium_rust_sdk::types::smart_contracts::ContractContext;

use concordium_rust_sdk::{
    common::{self, types::TransactionTime},
    smart_contracts::{
        common as concordium_std,
        common::Amount,
        types::{OwnedContractName, OwnedReceiveName},
    },
    types::{
        smart_contracts::{ModuleReference, OwnedParameter, WasmModule},
        transactions::{send, BlockItem, InitContractPayload, UpdateContractPayload},
        AccountInfo, AccountTransactionDetails, AccountTransactionEffects, BlockItemSummary,
        BlockItemSummaryDetails, ContractAddress, WalletAccount,
    },
    v2,
    v2::BlockIdentifier,
};
use std::path::PathBuf;
use structopt::*;
use strum_macros::EnumString;
use warp::path::param;

#[derive(StructOpt, EnumString)]

enum TransactionType {
    #[structopt(about = "Mint")]
    Mint,
    #[structopt(about = "Transfer")]
    Transfer,
    #[structopt(about = "TokenMetadata")]
    TokenMetadata,
}

#[derive(StructOpt)]
enum Action {
    #[structopt(about = "Deploy the module")]
    Deploy {
        #[structopt(long = "module", help = "Path to the contract module.")]
        module_path: PathBuf,
    },
    #[structopt(about = "Initialize the CIS-2 NFT contract")]
    Init {
        #[structopt(
            long,
            help = "The module reference used for initializing the contract instance."
        )]
        module_ref: ModuleReference,
    },
    #[structopt(
        about = "Update the contract and set the provided  using JSON parameters and a \
                 schema."
    )]
    UpdateWithSchema {
        #[structopt(short, long, help = "Path of the JSON parameter.")]
        parameter: Option<PathBuf>,
        #[structopt(long, help = "Path to the schema.")]
        schema: PathBuf,
        #[structopt(long, help = "The contract to update.")]
        address: ContractAddress,
        #[structopt(long, help = "Transaction Type")]
        transaction_type_: TransactionType,
    },
}
///
///
/// Node connection, key path and the action input struct
#[derive(StructOpt)]
struct App {
    #[structopt(
        long = "node",
        help = "GRPC interface of the node.",
        default_value = "http://node.testnet.concordium.com:20000"
    )]
    endpoint: v2::Endpoint,
    #[structopt(long = "account", help = "Path to the account key file.")]
    keys_path: PathBuf,
    #[structopt(subcommand, help = "The action you want to perform.")]
    action: Action,
}

////
///
///
use std::{fmt, println};
pub struct BlockDetails(BlockItemSummary);
impl fmt::Display for BlockDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?})", self.0.details)
    }
}

use concordium_rust_sdk::types::transactions::AccountTransaction;
use concordium_rust_sdk::types::transactions::EncodedPayload;
// pub use endpoints::{QueryError, QueryResult, RPCError, RPCResult};
#[derive(Debug)]
enum TransactionResult {
    StateChanging(AccountTransaction<EncodedPayload>),
    None,
}
////
///
///
///
#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    use base64::{engine::general_purpose, Engine as _};
    let app = {
        let app = App::clap().global_setting(AppSettings::ColoredHelp);
        let matches = app.get_matches();
        App::from_clap(&matches)
    };

    let mut client = v2::Client::new(app.endpoint)
        .await
        .context("Cannot connect.")?;

    // load account keys and sender address from a file
    let keys: WalletAccount =
        WalletAccount::from_json_file(app.keys_path).context("Could not read the keys file.")?;

    // Get the initial nonce at the last finalized block.
    let acc_info: AccountInfo = client
        .get_account_info(&keys.address.into(), &v2::BlockIdentifier::Best)
        .await?
        .response;

    let nonce = acc_info.account_nonce;
    // set expiry to now + 5min
    let expiry: TransactionTime =
        TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);

    let tx = match app.action {
        Action::Init {
            module_ref: mod_ref,
        } => {
            let param = OwnedParameter::empty();
            //                 .expect("Known to not exceed parameter size limit.");
            let payload = InitContractPayload {
                amount: Amount::zero(),
                mod_ref,
                init_name: OwnedContractName::new_unchecked(
                    "init_rust_sdk_minting_tutorial".to_string(),
                ),
                param,
            };
            TransactionResult::StateChanging(send::init_contract(
                &keys,
                keys.address,
                nonce,
                expiry,
                payload,
                10000u64.into(),
            ))
        }
        Action::UpdateWithSchema {
            parameter,
            schema,
            address,
            transaction_type_,
        } => {
            let parameter: serde_json::Value = serde_json::from_slice(
                &std::fs::read(parameter.unwrap()).context("Unable to read parameter file.")?,
            )
            .context("Unable to parse parameter JSON.")?;

            let schemab64 = std::fs::read(schema).context("Unable to read the schema file.")?;
            let schema_source = general_purpose::STANDARD_NO_PAD.decode(schemab64);

            let schema = concordium_std::from_bytes::<concordium_std::schema::VersionedModuleSchema>(
                &schema_source?,
            )?;
            // schema_global = schema;
            match transaction_type_ {
                TransactionType::Mint => {
                    let param_schema =
                        schema.get_receive_param_schema("rust_sdk_minting_tutorial", "mint")?;
                    let serialized_parameter = param_schema.serial_value(&parameter)?;
                    let message = OwnedParameter::try_from(serialized_parameter).unwrap();
                    let payload = UpdateContractPayload {
                        amount: Amount::zero(),
                        address,
                        receive_name: OwnedReceiveName::new_unchecked(
                            "rust_sdk_minting_tutorial.mint".to_string(),
                        ),
                        message,
                    };

                    TransactionResult::StateChanging(send::update_contract(
                        &keys,
                        keys.address,
                        nonce,
                        expiry,
                        payload,
                        10000u64.into(),
                    ))
                }
                //// Transfer Transaction which changes the state
                TransactionType::Transfer => {
                    let param_schema =
                        schema.get_receive_param_schema("rust_sdk_minting_tutorial", "transfer")?;
                    let serialized_parameter = param_schema.serial_value(&parameter)?;
                    let message = OwnedParameter::try_from(serialized_parameter).unwrap();
                    let payload = UpdateContractPayload {
                        amount: Amount::zero(),
                        address,
                        receive_name: OwnedReceiveName::new_unchecked(
                            "rust_sdk_minting_tutorial.transfer".to_string(),
                        ),
                        message,
                    };
                    //// call update contract with the payload
                    TransactionResult::StateChanging(send::update_contract(
                        &keys,
                        keys.address,
                        nonce,
                        expiry,
                        payload,
                        10000u64.into(),
                    ))
                }
                /// Token Metadata function with no state change
                TransactionType::TokenMetadata => {
                    let param_schema = schema
                        .get_receive_param_schema("rust_sdk_minting_tutorial", "tokenMetadata")?;
                    let rv_schema = schema.get_receive_return_value_schema(
                        "rust_sdk_minting_tutorial",
                        "tokenMetadata",
                    )?;

                    let serialized_parameter = param_schema.serial_value(&parameter)?;
                    let context = ContractContext {
                        invoker: None, //Account(AccountAddress),
                        contract: address,
                        amount: Amount::zero(),
                        method: OwnedReceiveName::new_unchecked(
                            "rust_sdk_minting_tutorial.tokenMetadata".to_string(),
                        ),
                        parameter: OwnedParameter::try_from(serialized_parameter).unwrap(), //Default::default(),
                        energy: 1000000.into(),
                    };
                    // invoke instance
                    let info = client
                        .invoke_instance(&BlockIdentifier::Best, &context)
                        .await?;

                    match info.response {
                            concordium_rust_sdk::types::smart_contracts::InvokeContractResult::Success { return_value, .. } => {
                                let bytes: concordium_rust_sdk::types::smart_contracts::ReturnValue = return_value.unwrap();
                                // deserialize and print return value
                                println!( "{}",rv_schema.to_json_string_pretty(&bytes.value)?);//jsonxf::pretty_print(&param_schema.to_json_string_pretty(&bytes.value)?).unwrap());
                            }
                            _ => {
                                println!("Could'nt succesfully invoke the instance. Check the parameters.")
                            }
                        }
                    TransactionResult::None

                    // info
                }
            }
        }
        Action::Deploy { module_path } => {
            let contents = std::fs::read(module_path).context("Could not read contract module.")?;
            let payload: WasmModule =
                common::Deserial::deserial(&mut std::io::Cursor::new(contents))?;
            TransactionResult::StateChanging(send::deploy_module(
                &keys,
                keys.address,
                nonce,
                expiry,
                payload,
            ))
        }
    };
    // let mut a;
    match tx {
        TransactionResult::StateChanging(result) => {
            let item = BlockItem::AccountTransaction(result);
            // submit the transaction to the chain
            let transaction_hash = client.send_block_item(&item).await?;
            println!(
                "Transaction {} submitted (nonce = {}).",
                transaction_hash, nonce,
            );
            let (bh, bs) = client.wait_until_finalized(&transaction_hash).await?;
            println!("Transaction finalized in block {}.", bh);

            match bs.details {
                BlockItemSummaryDetails::AccountTransaction(ad) => {
                    match ad.effects {
                        AccountTransactionEffects::ModuleDeployed { module_ref } => {
                            println!("module ref is {}", module_ref);
                        }
                        AccountTransactionEffects::ContractInitialized { data } => {
                            println!("Contract address is {}", data.address);
                        }
                        AccountTransactionEffects::None {
                            transaction_type,
                            reject_reason,
                        } => {
                            println!("The Rejection Outcome is {:#?}", reject_reason);
                        }
                        _ => (),
                    };
                }
                BlockItemSummaryDetails::AccountCreation(_) => (),
                BlockItemSummaryDetails::Update(_) => (),
            };
        }
        TransactionResult::None => {
            println!("No state changes, gracefully exiting.");
        }
    }

    Ok(())
}
