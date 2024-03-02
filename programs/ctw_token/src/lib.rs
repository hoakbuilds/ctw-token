use {
    anchor_lang::{
        __private::bytemuck::{self, Pod},
        prelude::*,
    },
    anchor_spl::{
        associated_token::AssociatedToken,
        token::{Token},
        token_2022::{mint_to, MintTo},
        token_interface::{initialize_mint2, Mint, TokenAccount, transfer_checked, TransferChecked},
    },
    solana_program::{instruction::Instruction, program::invoke},
    spl_token_2022::{
        check_program_account,
        extension::{
            confidential_transfer::instruction::{
                ConfidentialTransferInstruction, InitializeMintData,
            },
            ExtensionType,
        },
        instruction::{burn, TokenInstruction},
        solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalPubkey,
        state::{Mint as MintWithExtensions},
    }, 
};

/// Utility function for encoding instruction data
pub(crate) fn encode_instruction<T: Into<u8>, D: Pod>(
    token_program_id: &Pubkey,
    accounts: Vec<AccountMeta>,
    token_instruction_type: TokenInstruction,
    instruction_type: T,
    instruction_data: &D,
) -> Instruction {
    let mut data = token_instruction_type.pack();
    data.push(T::into(instruction_type));
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: *token_program_id,
        accounts,
        data,
    }
}

/// Create a `InitializeMint` instruction
/// This fn within spl-token-2022 is marked with target not os = solana,
/// which makes it impossible for programs to initialize confidential transfers via cpi.
pub fn initialize_confidential_transfer(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    auto_approve_new_accounts: bool,
    auditor_elgamal_pubkey: Option<ElGamalPubkey>,
) -> Result<Instruction> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::InitializeMint,
        &InitializeMintData {
            authority: authority.try_into()?,
            auto_approve_new_accounts: auto_approve_new_accounts.into(),
            auditor_elgamal_pubkey: auditor_elgamal_pubkey.try_into()?,
        },
    ))
}

declare_id!("cwTokjpVjxBeytEXomNe5B38EesYsNsXCm3JZC6tmvB");

/// The authority of the Confidential Transfer Wrapped Token Program.
pub mod authority {
    use ellipsis_macros::declare_pda;
    use solana_program::pubkey::Pubkey;

    declare_pda!(
        "5txHjtUXKw716ZY4M5uCU7MG51htjMewqWr91uR8jyBz",
        "cwTokjpVjxBeytEXomNe5B38EesYsNsXCm3JZC6tmvB",
        "AUTHORITY"
    );
}

#[derive(Clone)]
pub struct TokenExtensions;

impl Id for TokenExtensions {
    fn id() -> Pubkey {
        spl_token_2022::id()
    }
}

pub const AUTHORITY_SEED: &'static str = "AUTHORITY";
pub const MINT_SEED: &'static str = "MINT";

#[program]
pub mod ctw_token {
    use solana_program::program_option::COption;
    use spl_token_2022::extension::confidential_transfer::instruction::deposit;

    use super::*;

    /// Initialize a Confidential Transfer enabled Token Extensions Mint for an existing SPL Token Mint.
    /// This Confidential Transfer enabled Token Extensions Mint, or Confidential Wrapped Token Mint,
    /// effectively represents the same underlying SPL Token but with the ability to use Token Extensions'
    /// zk-powered confidential transfers which mask the amount being transferred.
    ///
    /// # Notes
    ///
    /// This implementation does not require any new CT-enabled Token Accounts to be approved and
    /// are 1:1 equivalents of the SPL Token.
    pub fn initialize(
        ctx: Context<Initialize>,
        auditor_pubkey: [u8; 32], // solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalPubkey length is 32 but it doesn't impl Borsh
    ) -> Result<()> {
        // Calculate space for the new mint with extensions
        let space = ExtensionType::try_calculate_account_len::<MintWithExtensions>(&[
            ExtensionType::ConfidentialTransferMint,
        ])
        .unwrap();
        let rent = Rent::get()?.minimum_balance(space);

        // Create the account for the new mint with extensions
        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.confidential_mint.to_account_info(),
                },
                &[&[
                    ctx.accounts.token_mint.key().as_ref(),
                    MINT_SEED.as_ref(),
                    &[ctx.bumps.confidential_mint],
                ]],
            ),
            rent,
            space as u64,
            ctx.accounts.token_extensions_program.key,
        )?;

        // Initialize the confidential transfer extension
        anchor_lang::solana_program::program::invoke(
            &initialize_confidential_transfer(
                &ctx.accounts.token_extensions_program.key(),
                &ctx.accounts.confidential_mint.key(),
                Some(ctx.accounts.program_authority.key()),
                true, // By default we do not require approval of new token accounts
                Some(
                    spl_token_2022::solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalPubkey(
                        auditor_pubkey,
                    ),
                ),
            )?,
            &[
                ctx.accounts.token_extensions_program.to_account_info(),
                ctx.accounts.confidential_mint.to_account_info(),
            ],
        )?;

        let freeze_authority = if let COption::Some(fa) = ctx.accounts.token_mint.freeze_authority {
            Some(fa)
        } else {
            None
        };
 
        // Initialize the new mint
        initialize_mint2(
            CpiContext::new_with_signer(
                ctx.accounts.token_extensions_program.to_account_info(),
                anchor_spl::token_interface::InitializeMint2 {
                    mint: ctx.accounts.confidential_mint.to_account_info(),
                },
                &[&[
                    ctx.accounts.token_mint.key().as_ref(),
                    MINT_SEED.as_ref(),
                    &[ctx.bumps.confidential_mint],
                ]],
            ),
            ctx.accounts.token_mint.decimals,
            &ctx.accounts.program_authority.key(),
            freeze_authority.as_ref(),
        )?;
        Ok(())
    }

    /// Wrap the given token amount of an SPL Token into an equivalent amount of a Confidential Wrapped Token Mint.
    ///
    /// # Notes
    ///
    /// The integrator is responsible for passing in a TokenAccount for the `confidential_token_account` param
    /// that has already been initialized and for which the [`ConfigureAccount`] as well as, if necessary,
    /// the [`ApproveAccount`] instructions have been executed.
    ///
    /// After this instruction is called, the integrator is then free to call [`Deposit`] and [`ApplyPendingBalance`]
    /// in order to roll the token amount into the available balance of the Confidential Token Account.
    pub fn wrap(ctx: Context<Wrap>, amount: u64) -> Result<()> {
        // Transfer tokens from the source to the program's vault
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.token_account.to_account_info(),
                    mint: ctx.accounts.token_mint.to_account_info(),
                    to: ctx.accounts.token_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.token_mint.decimals,
        )?;

        // Mint equivalent amount of tokens to the confidential wrapper token account
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_extensions_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.confidential_mint.to_account_info(),
                    to: ctx.accounts.confidential_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info(),
                },
                &[&[AUTHORITY_SEED.as_ref(), &[authority::bump()]]],
            ),
            amount,
        )?;

        // Deposit the minted tokens into the confidential balance of the account
        // OBS: This will still require integrations to call [`ApplyPendingBalance`] afterwards.
        invoke(
            &deposit(
                &ctx.accounts.token_extensions_program.key(),
                &ctx.accounts.confidential_token_account.key(),
                &ctx.accounts.confidential_mint.key(),
                amount,
                ctx.accounts.confidential_mint.decimals,
                &ctx.accounts.authority.key(),
                &[],
            )
            .unwrap(),
            &[
                ctx.accounts.confidential_token_account.to_account_info(),
                ctx.accounts.confidential_mint.to_account_info(),
                ctx.accounts.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }

    /// Unwrap the given token amount of a Confidential Wrapped Token back into it's corresponding
    /// SPL Token Mint.
    ///
    /// # Notes
    ///
    /// The integrator is responsible for assuring that the user has enough non-confidential
    /// balance in order to unwrap and redeem for the underlying token.
    /// This can be achieved by having the [`Withdraw`] instruction being successfully executed beforehand.
    pub fn unwrap(ctx: Context<Unwrap>, amount: u64) -> Result<()> {
        // Burn the desired amount of tokens from the user's confidential token account
        invoke(
            &burn(
                &ctx.accounts.token_extensions_program.key(),
                &ctx.accounts.confidential_token_account.key(),
                &ctx.accounts.confidential_mint.key(),
                &ctx.accounts.authority.key(),
                &[],
                amount,
            )
            .unwrap(),
            &[
                ctx.accounts.confidential_token_account.to_account_info(),
                ctx.accounts.confidential_mint.to_account_info(),
                ctx.accounts.authority.to_account_info(),
            ],
        )?;

        // Transfer tokens from the program's vault to the destination account
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.token_vault.to_account_info(),
                    mint: ctx.accounts.token_mint.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info(),
                },
                &[&[AUTHORITY_SEED.as_ref(), &[authority::bump()]]],
            ),
            amount,
            ctx.accounts.token_mint.decimals,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The SPL Token Mint for which we want to create a Confidential Transfers Mint Wrapper.
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [
            token_mint.key().as_ref(),
            MINT_SEED.as_ref()
        ],
        bump
    )]
    /// The SPL Token Extensions Mint.
    /// CHECK: Seeds are checked.
    pub confidential_mint: AccountInfo<'info>,

    #[account(
        seeds = [
            AUTHORITY_SEED.as_ref()
        ],
        bump
    )]
    /// The authority of the Confidential Wrapper Token Program.
    /// CHECK: Seeds are checked.
    pub program_authority: AccountInfo<'info>,

    /// The token vault.
    #[account(
        init,
        associated_token::mint = token_mint,
        associated_token::authority = program_authority,
        payer = payer,
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// The fee and rent payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The Token Program.
    pub token_program: Program<'info, Token>,

    /// The Associated Token Program.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// The Token Extensions Program.
    pub token_extensions_program: Program<'info, TokenExtensions>,

    /// The System Program.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Wrap<'info> {
    /// The mint of the token being wrapped.
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        token::authority = authority,
        token::mint = token_mint
    )]
    pub token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::authority = program_authority,
        token::mint = token_mint
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [
            token_mint.key().as_ref(),
            MINT_SEED.as_ref()
        ], 
        bump,
    )]
    /// The mint of the token being wrapped.
    pub confidential_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub confidential_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [
            AUTHORITY_SEED.as_ref()
        ],
        bump
    )]
    /// The authority of the Confidential Wrapper Token Program.
    /// CHECK: Seeds are checked.
    pub program_authority: AccountInfo<'info>,

    /// The authority of the source token account.
    pub authority: Signer<'info>,

    /// The fee and rent payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The Token Interface.
    pub token_program: Program<'info, Token>,

    /// The Token Interface.
    pub token_extensions_program: Program<'info, TokenExtensions>,
}

#[derive(Accounts)]
pub struct Unwrap<'info> {
    /// The mint of the token being wrapped.
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        token::authority = authority,
        token::mint = token_mint
    )]
    pub token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        token::authority = program_authority,
        token::mint = token_mint
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [
            token_mint.key().as_ref(),
            MINT_SEED.as_ref()
        ],
        bump
    )]
    /// The mint of the token being wrapped.
    pub confidential_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub confidential_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        seeds = [
            AUTHORITY_SEED.as_ref()
        ],
        bump
    )]
    /// The authority of the Confidential Wrapper Token Program.
    /// CHECK: Seeds are checked.
    pub program_authority: AccountInfo<'info>,

    /// The authority of the source token account.
    pub authority: Signer<'info>,

    /// The fee and rent payer.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The Token Interface.
    pub token_program: Program<'info, Token>,

    /// The Token Interface.
    pub token_extensions_program: Program<'info, TokenExtensions>,
}

#[cfg(feature = "client")]
pub fn derive_confidential_mint(token_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[token_mint.as_ref(), MINT_SEED.as_ref()], &crate::id())
}
