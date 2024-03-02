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
            extension::confidential_transfer::instruction::{
                apply_pending_balance, inner_configure_account, inner_withdraw,
            },
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
        },
    },
    ctw_token::{
        accounts::{Initialize, Unwrap, Wrap},
        derive_confidential_mint,
    },
    solana_program::{
        instruction::Instruction, native_token::sol_to_lamports, program_option::COption,
        program_pack::Pack, pubkey::Pubkey, system_instruction::create_account, system_program,
    },
    solana_program_test::{tokio, BanksClient, BanksClientError, ProgramTest, ProgramTestContext},
    solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction},
    spl_associated_token_account::instruction::create_associated_token_account,
    spl_token_2022::{
        extension::{
            confidential_transfer::ConfidentialTransferAccount, BaseStateWithExtensions,
            ExtensionType, StateWithExtensions,
        },
        instruction::reallocate,
        state::{Account, Mint},
    },
    std::num::NonZeroI8,
};

pub async fn start_new_program_test() -> ProgramTestContext {
    // Supress some of the logs
    solana_logger::setup_with_default(
        "solana_rbpf::vm=info,\
              solana_runtime::message_processor=trace,
              solana_runtime::system_instruction_processor=info,\
              solana_program_test=info",
    );

    let mut test = ProgramTest::new("ctw_token", ctw_token::id(), None);

    test.add_program("spl_token_2022", spl_token_2022::id(), None);

    let mut account = solana_sdk::account::Account::new(
        u32::MAX as u64,
        spl_token::state::Mint::LEN,
        &spl_token::id(),
    );

    spl_token::state::Mint {
        is_initialized: true,
        mint_authority: COption::None,
        decimals: 9,
        ..spl_token::state::Mint::default()
    }
    .pack_into_slice(&mut account.data);
    test.add_account(native_mint::id(), account);

    let context = test.start_with_context().await;
    context
}

#[tokio::test]
async fn end_to_end() {
    let mut test = start_new_program_test().await;

    initialize(&mut test.banks_client, &test.payer, &native_mint::id())
        .await
        .unwrap();

    println!("OK");

    create_and_configure_confidential_token_account(
        &mut test.banks_client,
        &test.payer,
        &native_mint::id(),
    )
    .await
    .unwrap();

    println!("OK");

    let amount = sol_to_lamports(1.0);

    wrap(
        &mut test.banks_client,
        &test.payer,
        &native_mint::id(),
        amount,
    )
    .await
    .unwrap();

    println!("OK");

    post_wrap(
        &mut test.banks_client,
        &test.payer,
        &native_mint::id(),
        amount,
    )
    .await
    .unwrap();

    println!("OK");

    withdraw_and_verify(
        &mut test.banks_client,
        &test.payer,
        &native_mint::id(),
        amount,
    )
    .await
    .unwrap();

    println!("OK");

    unwrap(
        &mut test.banks_client,
        &test.payer,
        &native_mint::id(),
        amount,
    )
    .await
    .unwrap();
}

async fn initialize(
    banks_client: &mut BanksClient,
    signer: &Keypair,
    token_mint: &Pubkey,
) -> Result<(), BanksClientError> {
    let elgamal_keypair = ElGamalKeypair::new_from_signer(signer, "auditor".as_ref()).unwrap();

    let (confidential_mint, _) = derive_confidential_mint(token_mint);

    println!(
        "Creating Confidential Wrapped Token Mint: {}",
        confidential_mint
    );

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

    println!("Submitting transaction...");

    let latest_blockhash = match banks_client.get_latest_blockhash().await {
        Ok(lb) => lb,
        Err(e) => {
            return Err(e);
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &vec![ix],
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match banks_client.process_transaction(tx).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn wrap(
    banks_client: &mut BanksClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    println!("\nWrapping into Confidential Transfer Wrapped Token..");

    let (confidential_mint, _) = derive_confidential_mint(token_mint);
    let token_vault = get_associated_token_address(&ctw_token::authority::ID, token_mint);
    let confidential_token_account = get_associated_token_address_with_program_id(
        &signer.pubkey(),
        &confidential_mint,
        &token_2022::ID,
    );

    let rent = banks_client.get_rent().await.unwrap();

    println!("Building transaction..");

    let (token_account, ixs) = if token_mint == &native_mint::id() {
        let keypair = Keypair::new();
        let token_account = keypair.pubkey();
        let lamports = rent.minimum_balance(TokenAccount::LEN);
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

    println!("Submitting transaction...");

    let latest_blockhash = match banks_client.get_latest_blockhash().await {
        Ok(lb) => lb,
        Err(e) => {
            return Err(e);
        }
    };

    let mut tx = Transaction::new_with_payer(&ixs, Some(&signer.pubkey()));
    tx.partial_sign(&[signer], latest_blockhash);

    if let Some(signer) = token_account {
        tx.partial_sign(&[&signer], latest_blockhash);
    }

    match banks_client.process_transaction(tx).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn unwrap(
    banks_client: &mut BanksClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    println!("\nUnwrapping from Confidential Transfer Wrapped Token..");

    let (confidential_mint, _) = derive_confidential_mint(token_mint);
    let token_vault = get_associated_token_address(&ctw_token::authority::ID, token_mint);
    let confidential_token_account = get_associated_token_address_with_program_id(
        &signer.pubkey(),
        &confidential_mint,
        &token_2022::ID,
    );

    println!("Building transaction..");

    let rent = banks_client.get_rent().await.unwrap();

    let (token_account, ixs) = if token_mint == &native_mint::id() {
        let keypair = Keypair::new();
        let token_account = keypair.pubkey();
        let lamports = rent.minimum_balance(TokenAccount::LEN);
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

    println!("Submitting transaction...");

    let latest_blockhash = match banks_client.get_latest_blockhash().await {
        Ok(lb) => lb,
        Err(e) => {
            return Err(e);
        }
    };
    let mut tx = Transaction::new_with_payer(&ixs, Some(&signer.pubkey()));
    tx.partial_sign(&[signer], latest_blockhash);

    if let Some(signer) = token_account {
        tx.partial_sign(&[&signer], latest_blockhash);
    }

    match banks_client.process_transaction(tx).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn create_and_configure_confidential_token_account(
    banks_client: &mut BanksClient,
    signer: &Keypair,
    token_mint: &Pubkey,
) -> Result<(), BanksClientError> {
    println!("\nCreating and configuring Token Account with Confidential Transfers extension..");

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

    let latest_blockhash = match banks_client.get_latest_blockhash().await {
        Ok(lb) => lb,
        Err(e) => {
            return Err(e);
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match banks_client.process_transaction(tx).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn withdraw_and_verify(
    banks_client: &mut BanksClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    println!("\nWithdrawing from encrypted balance and verifying..");

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

    let account = match banks_client.get_account(confidential_token_account).await {
        Ok(a) => a,
        Err(e) => {
            return Err(e);
        }
    };

    let account = account.unwrap();

    println!("Account data length: {}", account.data.len());

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

    let account = banks_client
        .get_account(confidential_mint)
        .await
        .unwrap()
        .unwrap();

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

    let latest_blockhash = match banks_client.get_latest_blockhash().await {
        Ok(lb) => lb,
        Err(e) => {
            return Err(e);
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match banks_client.process_transaction(tx).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn post_wrap(
    banks_client: &mut BanksClient,
    signer: &Keypair,
    token_mint: &Pubkey,
    amount: u64,
) -> Result<(), BanksClientError> {
    println!("\nApplying pending balance..");

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

    println!("Submitting transaction...");

    let latest_blockhash = match banks_client.get_latest_blockhash().await {
        Ok(lb) => lb,
        Err(e) => {
            return Err(e);
        }
    };
    let tx = Transaction::new_signed_with_payer(
        &ixs,
        Some(&signer.pubkey()),
        &[signer],
        latest_blockhash,
    );

    match banks_client.process_transaction(tx).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
