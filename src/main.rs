use crate::clap::AppSettings;
use crate::concordium_std::hashes::ModuleReferenceMarker;
use crate::concordium_std::schema::VersionedModuleSchema;
use anyhow::Context;

use crate::concordium_std::schema::Type;
use concordium_rust_sdk::types::hashes::HashBytes;
use concordium_rust_sdk::{
    common::{self, types::TransactionTime, SerdeDeserialize, SerdeSerialize},
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
};
use std::fs;
use std::path::PathBuf;
use structopt::*;
use strum_macros::EnumString;

#[derive(StructOpt, EnumString, PartialEq)]

enum TransactionType {
    #[structopt(about = "Mint")]
    Mint,
    #[structopt(about = "Transfer")]
    Transfer,
    #[structopt(about = "View")]
    View,
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
        #[structopt(long, help = "Path of the JSON parameter.")]
        parameter: PathBuf,
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
use std::fmt;
pub struct BlockDetails(BlockItemSummary);
impl fmt::Display for BlockDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?})", self.0.details)
    }
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

    let param_schema_for_view: Type = Type::ByteArray(128); //

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

            send::init_contract(&keys, keys.address, nonce, expiry, payload, 10000u64.into())
        }
        Action::UpdateWithSchema {
            parameter,
            schema,
            address,
            transaction_type_,
        } => {
            let parameter: serde_json::Value = serde_json::from_slice(
                &std::fs::read(parameter).context("Unable to read parameter file.")?,
            )
            .context("Unable to parse parameter JSON.")?;

            let schemab64 = std::fs::read(schema).context("Unable to read the schema file.")?;
            let schema_source = general_purpose::STANDARD_NO_PAD.decode(schemab64);

            let schema = concordium_std::from_bytes::<concordium_std::schema::VersionedModuleSchema>(
                &schema_source?,
            )?;

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

                    send::update_contract(
                        &keys,
                        keys.address,
                        nonce,
                        expiry,
                        payload,
                        10000u64.into(),
                    )
                }
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

                    send::update_contract(
                        &keys,
                        keys.address,
                        nonce,
                        expiry,
                        payload,
                        10000u64.into(),
                    )
                }
                TransactionType::View => {
                    let param_schema = schema
                        .get_receive_return_value_schema("rust_sdk_minting_tutorial", "view")
                        .unwrap();

                    // param_schema.to_json_string_pretty(&param_schema);

                    //let serialized_parameter = param_schema.serial_value(&parameter)?;
                    let message = OwnedParameter::empty(); //OwnedParameter::try_from(serialized_parameter).unwrap();
                    let payload = UpdateContractPayload {
                        amount: Amount::zero(),
                        address,
                        receive_name: OwnedReceiveName::new_unchecked(
                            "rust_sdk_minting_tutorial.view".to_string(),
                        ),
                        message,
                    };

                    send::update_contract(
                        &keys,
                        keys.address,
                        nonce,
                        expiry,
                        payload,
                        10000u64.into(),
                    )
                }
            }
        }

        Action::Deploy { module_path } => {
            let contents = std::fs::read(module_path).context("Could not read contract module.")?;
            let payload: WasmModule =
                common::Deserial::deserial(&mut std::io::Cursor::new(contents))?;
            send::deploy_module(&keys, keys.address, nonce, expiry, payload)
        }
    };

    let item = BlockItem::AccountTransaction(tx);
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
                AccountTransactionEffects::ContractUpdateIssued { effects } => {
                    // println!("Contract updated {:#?}", effects);
                    println!(
                        "Contract updated {:#?}",
                        param_schema_for_view.to_json_string_pretty(&bh.bytes)?
                    );
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

    Ok(())
}
