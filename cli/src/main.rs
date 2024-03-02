use {
    anchor_lang::{InstructionData, ToAccountMetas},
    anchor_spl::{
        associated_token::{
            self, get_associated_token_address, get_associated_token_address_with_program_id,
        },
        token::{
            spl_token::{
                self,
                instruction::{close_account, initialize_account},
                native_mint,
            },
            TokenAccount,
        },
        token_2022,
        token_interface::spl_token_2022::{
            extension::{
                confidential_transfer::{
                    instruction::{apply_pending_balance, inner_configure_account, inner_withdraw},
                    ConfidentialTransferAccount,
                },
                BaseStateWithExtensions, ExtensionType, StateWithExtensions,
            },
            instruction::reallocate,
            proof::ProofLocation,
            solana_zk_token_sdk::{
                encryption::{
                    auth_encryption::AeKey,
                    elgamal::{ElGamalCiphertext, ElGamalKeypair},
                },
                zk_token_proof_instruction::{
                    verify_pubkey_validity, verify_withdraw, PubkeyValidityData, WithdrawData,
                },
            },
            state::{Account, Mint},
        },
    },
    clap::{Parser, Subcommand},
    ctw_token::{
        accounts::{Initialize, Unwrap, Wrap},
        derive_confidential_mint,
    },
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        instruction::Instruction,
        pubkey::{ParsePubkeyError, Pubkey},
        signature::{read_keypair_file, Keypair, Signature},
        signer::Signer,
        system_instruction::create_account,
        system_program,
        transaction::Transaction,
    },
    spl_associated_token_account::instruction::create_associated_token_account,
    std::{num::NonZeroI8, path::PathBuf, str::FromStr},
    thiserror::Error,
};

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Client(#[from] solana_client::client_error::ClientError),
    #[error("Loading keypair. {:?}", self)]
    LoadingKeypair(Box<dyn std::error::Error>),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    rpc_client: String,

    #[arg(short, long)]
    keypair_path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Initialize a Confidential Wrapped Token Mint for an existing SPL Token Mint.
    Initialize {
        /// The SPL Token Mint for which to create a Confidential Wrapped Token.
        #[arg(short, long)]
        token_mint: String,
    },
    /// Wrap a given token amount into the corresponding Confidential Wrapped Token.
    Wrap {
        /// The SPL Token Mint to wrap into a Confidential Wrapped Token.
        #[arg(short, long)]
        token_mint: String,
        /// The amount to wrap.
        #[arg(short, long)]
        amount: u64,
    },
    /// Unwrap a given token amount of Confidential Wrapped Token into the corresponding SPL Token.
    Unwrap {
        /// The SPL Token Mint to unwrap from a Confidential Wrapped Token.
        #[arg(short, long)]
        token_mint: String,
        /// The amount to unwrap.
        #[arg(short, long)]
        amount: u64,
    },
}

fn parse_pubkey(value: &str) -> Result<Pubkey, ParsePubkeyError> {
    Pubkey::from_str(value)
}

fn load_keypair(path: PathBuf) -> Result<Keypair, Error> {
    match read_keypair_file(path) {
        Ok(k) => Ok(k),
        Err(e) => Err(Error::LoadingKeypair(e)),
    }
}

fn main() {
    let cli = Args::parse();

    let rpc_client = RpcClient::new(cli.rpc_client);

    let signer = match load_keypair(cli.keypair_path) {
        Ok(k) => k,
        Err(e) => {
            println!("Could not load the given keypair.\nError: {:?}", e);
            return;
        }
    };

    match cli.command {
        Commands::Initialize { token_mint } => {
            println!("Initializing Confidential Wrapped Token Mint..");

            let token_mint = match parse_pubkey(&token_mint) {
                Ok(p) => p,
                Err(e) => {
                    println!("Failed to parse token mint pubkey.\nError: {:?}", e);
                    return;
                }
            };

            println!("SPL Token Mint: {}", token_mint);

            match initialize(&rpc_client, &signer, &token_mint) {
                Ok(s) => {
                    println!("Successfully initialized confidential wrapped token..\nTransaction signature: https://solana.fm/tx/{}", s);
                }
                Err(e) => {
                    println!(
                        "Failed to initialize confidential wrapped token.\nError: {:?}",
                        e
                    );
                    return;
                }
            };
        }
        Commands::Wrap { token_mint, amount } => {
            println!(
                "Wrapping {} of {} into the equivalent Confidential Wrapped Token Mint..",
                amount, token_mint
            );

            let token_mint = match parse_pubkey(&token_mint) {
                Ok(p) => p,
                Err(e) => {
                    println!("Failed to parse token mint pubkey.\nError: {:?}", e);
                    return;
                }
            };

            println!("SPL Token Mint: {}", token_mint);

            match create_and_configure_confidential_token_account(&rpc_client, &signer, &token_mint)
            {
                Ok(s) => {
                    println!("Successfully created and configured token account for confidential token usage..\nTransaction signature: https://solana.fm/tx/{}", s);
                }
                Err(e) => {
                    println!("Failed to create and configure token account for confidential token.\nError: {:?}",e);
                    return;
                }
            };

            match wrap(&rpc_client, &signer, &token_mint, amount) {
                Ok(s) => {
                    println!(
                        "Successfully wrapped...\nTransaction signature: https://solana.fm/tx/{}",
                        s
                    );
                }
                Err(e) => {
                    println!("Failed to wrap.\nError: {:?}", e);
                    return;
                }
            };

            match post_wrap(&rpc_client, &signer, &token_mint, amount) {
                Ok(s) => {
                    println!(
                        "Wrapped amount is now available for confidential transfers!\nTransaction signature: https://solana.fm/tx/{}",
                        s
                    );
                }
                Err(e) => {
                    println!(
                        "Failed to deposit and apply pending balance.\nError: {:?}",
                        e
                    );
                    return;
                }
            };
        }
        Commands::Unwrap { token_mint, amount } => {
            println!(
                "Unwrapping {} into {} into SPL Token Mint..",
                amount, token_mint
            );

            let token_mint = match parse_pubkey(&token_mint) {
                Ok(p) => p,
                Err(e) => {
                    println!("Failed to parse token mint pubkey.\nError: {:?}", e);
                    return;
                }
            };

            println!("SPL Token Mint: {}", token_mint);

            match withdraw_and_verify(&rpc_client, &signer, &token_mint, amount) {
                Ok(s) => {
                    println!(
                        "Successfully processed withdrawawl from confidential balance..\nTransaction signature: https://solana.fm/tx/{}",
                        s
                    );
                }
                Err(e) => {
                    println!("Failed to wrap.\nError: {:?}", e);
                    return;
                }
            };

            match unwrap(&rpc_client, &signer, &token_mint, amount) {
                Ok(s) => {
                    println!(
                        "Successfully unwrapped!\nTransaction signature: https://solana.fm/tx/{}",
                        s
                    );
                }
                Err(e) => {
                    println!("Failed to unwrap.\nError: {:?}", e);
                    return;
                }
            };
        }
    }
}

fn initialize(
    rpc_client: &RpcClient,
    signer: &Keypair,
    token_mint: &Pubkey,
) -> Result<Signature, Error> {
    let elgamal_keypair = ElGamalKeypair::new_from_signer(signer, "auditor".as_ref()).unwrap();

    let (confidential_mint, _) = derive_confidential_mint(token_mint);

    println!("Confidnetial Wrapped Token Mint: {}", confidential_mint);

    let token_vault = get_associated_token_address(&ctw_token::authority::ID, token_mint);

    let ix = Instruction {
        accounts: Initialize {
            token_mint: *token_mint,
            program_authority: ctw_token::authority::ID,
            confidential_mint,
            token_vault,
            payer: signer.pubkey(),
            token_program: spl_token::ID,
            associated_token_program: associated_token::ID,
            token_extensions_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        program_id: ctw_token::ID,
        data: ctw_token::instruction::Initialize {
            auditor_pubkey: elgamal_keypair.pubkey().to_bytes(),
        }
        .data(),
    };
    let latest_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(lb) => lb,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &vec![ix],
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match rpc_client.send_and_confirm_transaction_with_spinner(&tx) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::Client(e)),
    }
}

fn wrap(
    rpc_client: &RpcClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<Signature, Error> {
    let (confidential_mint, _) = derive_confidential_mint(token_mint);
    let token_vault = get_associated_token_address(&ctw_token::authority::ID, token_mint);
    let confidential_token_account = get_associated_token_address_with_program_id(
        &signer.pubkey(),
        &confidential_mint,
        &token_2022::ID,
    );

    let (token_account, ixs) = if token_mint == &native_mint::id() {
        let keypair = Keypair::new();
        let token_account = keypair.pubkey();
        let lamports = rpc_client
            .get_minimum_balance_for_rent_exemption(TokenAccount::LEN)
            .unwrap();
        (
            Some(keypair),
            vec![
                create_account(
                    &signer.pubkey(),
                    &token_account,
                    lamports + amount,
                    TokenAccount::LEN as u64,
                    &spl_token::id(),
                ),
                initialize_account(
                    &spl_token::id(),
                    &token_account,
                    token_mint,
                    &signer.pubkey(),
                )
                .unwrap(),
                Instruction {
                    accounts: Wrap {
                        token_mint: *token_mint,
                        token_account,
                        program_authority: ctw_token::authority::ID,
                        confidential_mint,
                        confidential_token_account,
                        token_vault,
                        authority: signer.pubkey(),
                        payer: signer.pubkey(),
                        token_program: spl_token::ID,
                        token_extensions_program: token_2022::ID,
                    }
                    .to_account_metas(None),
                    program_id: ctw_token::ID,
                    data: ctw_token::instruction::Wrap { amount }.data(),
                },
                close_account(
                    &spl_token::id(),
                    &token_account,
                    &signer.pubkey(),
                    &signer.pubkey(),
                    &[],
                )
                .unwrap(),
            ],
        )
    } else {
        let token_account = get_associated_token_address(&signer.pubkey(), token_mint);
        (
            None,
            vec![Instruction {
                accounts: Wrap {
                    token_mint: *token_mint,
                    token_account,
                    program_authority: ctw_token::authority::ID,
                    confidential_mint,
                    confidential_token_account,
                    token_vault,
                    authority: signer.pubkey(),
                    payer: signer.pubkey(),
                    token_program: spl_token::ID,
                    token_extensions_program: token_2022::ID,
                }
                .to_account_metas(None),
                program_id: ctw_token::ID,
                data: ctw_token::instruction::Wrap { amount }.data(),
            }],
        )
    };

    let latest_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(lb) => lb,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };

    let mut tx = Transaction::new_with_payer(&ixs, Some(&signer.pubkey()));
    tx.partial_sign(&[signer], latest_blockhash);

    if let Some(signer) = token_account {
        tx.partial_sign(&[&signer], latest_blockhash);
    }

    match rpc_client.send_and_confirm_transaction_with_spinner(&tx) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::Client(e)),
    }
}

fn unwrap(
    rpc_client: &RpcClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<Signature, Error> {
    let (confidential_mint, _) = derive_confidential_mint(token_mint);
    let token_vault = get_associated_token_address(&ctw_token::authority::ID, token_mint);
    let confidential_token_account = get_associated_token_address_with_program_id(
        &signer.pubkey(),
        &confidential_mint,
        &token_2022::ID,
    );

    let (token_account, ixs) = if token_mint == &native_mint::id() {
        let keypair = Keypair::new();
        let token_account = keypair.pubkey();
        let lamports = rpc_client
            .get_minimum_balance_for_rent_exemption(TokenAccount::LEN)
            .unwrap();
        (
            Some(keypair),
            vec![
                create_account(
                    &signer.pubkey(),
                    &token_account,
                    lamports,
                    TokenAccount::LEN as u64,
                    &spl_token::id(),
                ),
                initialize_account(
                    &spl_token::id(),
                    &token_account,
                    token_mint,
                    &signer.pubkey(),
                )
                .unwrap(),
                Instruction {
                    accounts: Unwrap {
                        token_mint: *token_mint,
                        token_account,
                        program_authority: ctw_token::authority::ID,
                        confidential_mint,
                        confidential_token_account,
                        token_vault,
                        authority: signer.pubkey(),
                        payer: signer.pubkey(),
                        token_program: spl_token::ID,
                        token_extensions_program: token_2022::ID,
                    }
                    .to_account_metas(None),
                    program_id: ctw_token::ID,
                    data: ctw_token::instruction::Unwrap { amount }.data(),
                },
                close_account(
                    &spl_token::id(),
                    &token_account,
                    &signer.pubkey(),
                    &signer.pubkey(),
                    &[],
                )
                .unwrap(),
            ],
        )
    } else {
        let token_account = get_associated_token_address(&signer.pubkey(), token_mint);
        (
            None,
            vec![Instruction {
                accounts: Unwrap {
                    token_mint: *token_mint,
                    token_account,
                    program_authority: ctw_token::authority::ID,
                    confidential_mint,
                    confidential_token_account,
                    token_vault,
                    authority: signer.pubkey(),
                    payer: signer.pubkey(),
                    token_program: spl_token::ID,
                    token_extensions_program: token_2022::ID,
                }
                .to_account_metas(None),
                program_id: ctw_token::ID,
                data: ctw_token::instruction::Unwrap { amount }.data(),
            }],
        )
    };

    let latest_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(lb) => lb,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };

    let mut tx = Transaction::new_with_payer(&ixs, Some(&signer.pubkey()));
    tx.partial_sign(&[signer], latest_blockhash);

    if let Some(signer) = token_account {
        tx.partial_sign(&[&signer], latest_blockhash);
    }

    match rpc_client.send_and_confirm_transaction_with_spinner(&tx) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::Client(e)),
    }
}

fn create_and_configure_confidential_token_account(
    rpc_client: &RpcClient,
    signer: &Keypair,
    token_mint: &Pubkey,
) -> Result<Signature, Error> {
    let (confidential_mint, _) = derive_confidential_mint(token_mint);
    let confidential_token_account = get_associated_token_address_with_program_id(
        &signer.pubkey(),
        &confidential_mint,
        &token_2022::ID,
    );

    let mut ixs = vec![
        create_associated_token_account(
            &signer.pubkey(),
            &signer.pubkey(),
            &confidential_mint,
            &token_2022::ID,
        ),
        reallocate(
            &token_2022::ID,
            &confidential_token_account,
            &signer.pubkey(),
            &signer.pubkey(),
            &[],
            &[ExtensionType::ConfidentialTransferAccount],
        )
        .unwrap(),
    ];

    let elgamal_keypair = ElGamalKeypair::new_from_signer(signer, "cwtoken".as_ref()).unwrap();
    let proof_data = PubkeyValidityData::new(&elgamal_keypair).unwrap();

    println!(
        "Using ElGamal keypair with public key: {}",
        elgamal_keypair.pubkey()
    );

    println!("Building validity proofs..");

    let proof_data_location =
        ProofLocation::InstructionOffset(NonZeroI8::new(1).unwrap(), &proof_data);

    let ae_key = AeKey::new_from_signer(signer, "cwtoken".as_ref()).unwrap();
    let decryptable_zero_balance = ae_key.encrypt(0);

    ixs.extend(vec![
        inner_configure_account(
            &token_2022::ID,
            &confidential_token_account,
            &confidential_mint,
            decryptable_zero_balance,
            u64::MAX,
            &signer.pubkey(),
            &[],
            proof_data_location,
        )
        .unwrap(),
        verify_pubkey_validity(None, &proof_data),
    ]);

    println!("Submitting transaction...");

    let latest_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(lb) => lb,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match rpc_client.send_and_confirm_transaction_with_spinner(&tx) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::Client(e)),
    }
}

fn withdraw_and_verify(
    rpc_client: &RpcClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<Signature, Error> {
    let (confidential_mint, _) = derive_confidential_mint(token_mint);
    let confidential_token_account = get_associated_token_address_with_program_id(
        &signer.pubkey(),
        &confidential_mint,
        &token_2022::ID,
    );

    let elgamal_keypair = ElGamalKeypair::new_from_signer(signer, "cwtoken".as_ref()).unwrap();
    println!(
        "Using ElGamal keypair with public key: {}",
        elgamal_keypair.pubkey()
    );

    let account = match rpc_client.get_account(&confidential_token_account) {
        Ok(a) => a,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };

    println!("Building validity proofs..");

    let token_account = StateWithExtensions::<Account>::unpack(&account.data).unwrap();

    let confidential_transfer_account = token_account
        .get_extension::<ConfidentialTransferAccount>()
        .unwrap();

    let current_balance = confidential_transfer_account
        .available_balance
        .decrypt(elgamal_keypair.secret())
        .unwrap();
    println!("Current balance: {}", current_balance);

    let current_ciphertext =
        ElGamalCiphertext::from_bytes(&confidential_transfer_account.available_balance.0).unwrap();

    let proof_data = WithdrawData::new(
        amount,
        &elgamal_keypair,
        current_balance,
        &current_ciphertext,
    )
    .unwrap();

    let proof_data_location =
        ProofLocation::InstructionOffset(NonZeroI8::new(1).unwrap(), &proof_data);

    let ae_key = AeKey::new_from_signer(signer, "cwtoken".as_ref()).unwrap();

    let new_decryptable_zero_balance = ae_key.encrypt(current_balance - amount);

    let account = match rpc_client.get_account(&confidential_token_account) {
        Ok(a) => a,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };

    let confidential_mint_account = StateWithExtensions::<Mint>::unpack(&account.data).unwrap();

    println!("Proofs generated, building transaction..");

    let ixs = vec![
        inner_withdraw(
            &token_2022::ID,
            &confidential_token_account,
            &confidential_mint,
            amount,
            confidential_mint_account.base.decimals,
            new_decryptable_zero_balance.into(),
            &signer.pubkey(),
            &[],
            proof_data_location,
        )
        .unwrap(),
        verify_withdraw(None, &proof_data),
    ];

    println!("Submitting transaction...");

    let latest_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(lb) => lb,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match rpc_client.send_and_confirm_transaction_with_spinner(&tx) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::Client(e)),
    }
}

fn post_wrap(
    rpc_client: &RpcClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<Signature, Error> {
    let (confidential_mint, _) = derive_confidential_mint(token_mint);
    let confidential_token_account = get_associated_token_address_with_program_id(
        &signer.pubkey(),
        &confidential_mint,
        &token_2022::ID,
    );

    let elgamal_keypair = ElGamalKeypair::new_from_signer(signer, "cwtoken".as_ref()).unwrap();
    println!(
        "Using ElGamal keypair with public key: {}",
        elgamal_keypair.pubkey()
    );

    println!("Building validity proofs..");

    let ae_key = AeKey::new_from_signer(signer, "cwtoken".as_ref()).unwrap();
    let current_balance = 0;
    let new_decryptable_zero_balance = ae_key.encrypt(current_balance + amount);

    println!("Proofs generated, building transaction..");

    let ixs = vec![apply_pending_balance(
        &token_2022::ID,
        &confidential_token_account,
        1,
        new_decryptable_zero_balance,
        &signer.pubkey(),
        &[],
    )
    .unwrap()];

    let latest_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(lb) => lb,
        Err(e) => {
            return Err(Error::Client(e));
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match rpc_client.send_and_confirm_transaction_with_spinner(&tx) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error::Client(e)),
    }
}
