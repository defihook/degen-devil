use anchor_lang::prelude::*;
use anchor_spl::token::{Approve, Mint, SetAuthority, Token, TokenAccount, Transfer};
use std::mem::size_of;
mod utils;
pub use utils::*;

declare_id!("H2LCFgiKNFwdZyVQoJFhhhygvvuV8twbfzJ8nJpJHgG1");

/**
 * The degendevil Program (Variable Probability Coin - P2P Heads and Tails)
 *
 * Accounts:
 * requester: PDA owned by the degenrand Program used to store data
 * oracle: The Oracle's account. Refer to Published Addresses.
 * oracle_vault: PDA owned by the degenrand Program for paying Oracle
 * degenrand_program: The Program Address for the degenrand Program
 * coin: PDA owned by degendevil used for storing data
 * vault: PDA owned by degendevil used for escrowing sol and paying winner
 * initiator: The account creating the coin
 * acceptor: The account accepting the offer to flip
 * rent: The Rent Program
 * system_program: The System Program
 *
 * Considerations:
 * 1. The CPI call to RequestRandom should happen only after or all funds are locked into the contract.
 * 2. Once a CPI call to RequestRandom is made, no funds should be allowed to be withdrawn.
 *
 */

const COIN_PREFIX: &str = "DEGENDEVIL_COIN_SEED_V1.0";
const VAULT_PREFIX: &str = "DEGENDEVIL_VAULT_SEED_V1.0";
const WINNER_PREFIX: &str = "DEGENDEVIL_WINNER_SEED_V1.0";
const ORACLE_FEE: u64 = 495000;

#[program]
pub mod degendevil {
    use std::ops::DerefMut;

    use spl_token::instruction::AuthorityType;

    use super::*;

    pub fn fallback<'info>(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo<'info>],
        _data: &[u8],
    ) -> Result<()> {
        return Err(DegenErrorCode::FallBacked.into());
    }

    pub fn create_coin(
        ctx: Context<CreateCoin>,
        coin_bump: u8,
        vault_bump: u8,
        amount: u64,
    ) -> Result<()> {
        let authority_key = ctx.accounts.initiator.key();
        // Set data for PDAs
        {
            let coin = &mut ctx.accounts.coin.load_init()?;
            let clock: Clock = Clock::get()?;

            coin.initiator = authority_key;
            coin.is_flipping = false;
            coin.created_at = clock.unix_timestamp;
            coin.bump = coin_bump;

            let vault = &mut ctx.accounts.vault;

            vault.coin_info = CoinInfo {
                amount,
                mint_token: ctx.accounts.mint.key(),
            };

            vault.bump = vault_bump;
        }

        degenrand::cpi::transfer_authority(ctx.accounts.coin_transfer_authority_ctx())?;

        // Delegate the Vault to be able to transfer SPL token from initiator and acceptor atas.
        // Assume authority over ata of initiator
        anchor_spl::token::set_authority(
            ctx.accounts.token_set_authority_ctx(),
            AuthorityType::AccountOwner,
            Some(ctx.accounts.vault.key()),
        )?;

        let cpi_accounts = degenrand::cpi::accounts::RequestRandom {
            requester: ctx.accounts.requester.to_account_info(),
            vault: ctx.accounts.oracle_vault.clone(),
            authority: ctx.accounts.coin.to_account_info(),
            oracle: ctx.accounts.oracle.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        let (_coin_authority, coin_bump) = coin_pda(ctx.accounts.initiator.key);

        let coin_seeds = &[
            COIN_PREFIX.as_bytes(),
            ctx.accounts.initiator.key.as_ref(),
            &[coin_bump],
        ];

        let signer = &[&coin_seeds[..]];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.degenrand_program.clone(),
            cpi_accounts,
            signer,
        );

        degenrand::cpi::request_random(cpi_context)?;

        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.initiator.key(),
            &ctx.accounts.oracle_vault.key(),
            ORACLE_FEE,
        );

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.initiator.to_account_info(),
                ctx.accounts.oracle_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        Ok(())
    }

    pub fn reveal_coin<'key, 'accounts, 'remaining, 'info>(
        ctx: Context<'key, 'accounts, 'remaining, 'info, RevealCoin<'info>>,
    ) -> Result<()> {
        {
            if ctx.accounts.authority.key() != ctx.accounts.initiator.key() {
                return Err(DegenErrorCode::Unauthorized.into());
            }
        }
        // Determine winner from random number
        {
            let requester_loader: AccountLoader<degenrand::Requester> =
                AccountLoader::try_from_unchecked(ctx.program_id, &ctx.accounts.requester)?;

            let requester = requester_loader.load()?;

            if requester.active_request {
                return Err(DegenErrorCode::OracleNotCompleted.into());
            }

            let status =
                calculate_probability(ctx.accounts.vault.coin_info.amount, &requester.random) != 0;

            {
                let winner_pda = &mut ctx.accounts.winner;
                winner_pda.status = status;
                winner_pda.winner = ctx.accounts.initiator.key();
            }

            let (_, vault_bump) = vault_pda(
                &ctx.accounts.vault.coin_info.mint_token,
                ctx.accounts.initiator.key,
            );

            let signer_seeds = &[
                VAULT_PREFIX.as_bytes(),
                ctx.accounts.vault.coin_info.mint_token.as_ref(),
                ctx.accounts.initiator.key.as_ref(),
                ctx.program_id.as_ref(),
                &[vault_bump],
            ];

            anchor_spl::token::transfer(
                ctx.accounts
                    .token_transfer_ctx(
                        ctx.accounts.vault.to_account_info(),
                        ctx.accounts.initiator_ata.to_account_info(),
                        ctx.accounts.admin_ata.to_account_info(),
                    )
                    .with_signer(&[signer_seeds]),
                ctx.accounts.vault.coin_info.amount,
            )?;

            anchor_spl::token::set_authority(
                ctx.accounts
                    .token_reset_initiator_authority_ctx()
                    .with_signer(&[signer_seeds]),
                AuthorityType::AccountOwner,
                Some(ctx.accounts.initiator.key()),
            )?;
        }

        // Transfer back ownership of requester
        let coin_acc = &ctx.remaining_accounts[0];

        let cpi_accounts = degenrand::cpi::accounts::TransferAuthority {
            requester: ctx.accounts.requester.to_account_info(),
            authority: coin_acc.to_account_info(),
            new_authority: ctx.accounts.initiator.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };

        let (_coin_authority, coin_bump) = Pubkey::find_program_address(
            &[COIN_PREFIX.as_bytes(), ctx.accounts.initiator.key.as_ref()],
            &ctx.program_id,
        );

        let coin_seeds = &[
            COIN_PREFIX.as_bytes(),
            ctx.accounts.initiator.key.as_ref(),
            &[coin_bump],
        ];

        let signer = &[&coin_seeds[..]];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.degenrand_program.clone(),
            cpi_accounts,
            signer,
        );

        degenrand::cpi::transfer_authority(cpi_context)?;

        **ctx
            .accounts
            .initiator
            .to_account_info()
            .try_borrow_mut_lamports()?
            .deref_mut() +=
            ctx.accounts.vault.to_account_info().lamports() + coin_acc.to_account_info().lamports();

        **ctx
            .accounts
            .vault
            .to_account_info()
            .try_borrow_mut_lamports()?
            .deref_mut() = 0;

        **coin_acc
            .to_account_info()
            .try_borrow_mut_lamports()?
            .deref_mut() = 0;

        return Ok(());
    }

    pub fn remove_pdas(ctx: Context<Cleanup>) -> Result<()> {
        let winner = &mut ctx.accounts.winner;
        if winner.winner != ctx.accounts.initiator.key() {
            return Err(DegenErrorCode::Unauthorized.into());
        }

        let (winner_pda, _) = winner_pda(&ctx.accounts.initiator.key());

        if winner_pda != ctx.accounts.winner.key() {
            return Err(DegenErrorCode::Unauthorized.into());
        }

        **ctx
            .accounts
            .initiator
            .to_account_info()
            .try_borrow_mut_lamports()?
            .deref_mut() += ctx.accounts.winner.to_account_info().lamports();

        **ctx
            .accounts
            .winner
            .to_account_info()
            .try_borrow_mut_lamports()?
            .deref_mut() = 0;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateCoin<'info> {
    #[account(
        init,
        payer = initiator,
        seeds = [COIN_PREFIX.as_bytes(), initiator.key().as_ref()],
        bump,
        space = 8 + size_of::<Coin>(),
    )]
    pub coin: AccountLoader<'info, Coin>,

    #[account(
        init,
        seeds = [VAULT_PREFIX.as_bytes(), mint.key().as_ref(), initiator.key().as_ref(), crate::id().as_ref(),],
        bump,
        payer = initiator,
        space = 8 + size_of::<Vault>()
    )]
    pub vault: Account<'info, Vault>,

    /// CHECK: PDA for calling the Oracle for random number
    #[account(mut)]
    pub requester: AccountInfo<'info>,

    /// CHECK: Initiator of the flip
    #[account(mut)]
    pub initiator: Signer<'info>,

    /// CHECK: Initiator Token ATA
    #[account(mut)]
    pub initiator_ata: Account<'info, TokenAccount>,

    /// CHECK: Account making the random request
    #[account(mut)]
    pub oracle: AccountInfo<'info>,

    /// CHECK: Token A mint
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    /// CHECK: PDA holding the coin toss info.
    #[account(mut)]
    pub oracle_vault: AccountInfo<'info>,

    /// CHECK: The program responsible for generating randomness and holding the random number.
    pub degenrand_program: AccountInfo<'info>,

    /// CHECK: System Variable for getting rent to create a PDA.
    pub rent: Sysvar<'info, Rent>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

impl<'info> CreateCoin<'info> {
    pub fn coin_transfer_authority_ctx(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, degenrand::cpi::accounts::TransferAuthority<'info>> {
        // Transfer authority for the oracle requester to the Coin PDA
        let cpi_accounts = degenrand::cpi::accounts::TransferAuthority {
            requester: self.requester.to_account_info(),
            authority: self.initiator.to_account_info(),
            new_authority: self.coin.to_account_info(),
            system_program: self.system_program.to_account_info(),
        };

        CpiContext::new(self.degenrand_program.clone(), cpi_accounts)
    }

    pub fn token_approve_ctx<'b, 'c>(&self) -> CpiContext<'_, 'b, 'c, 'info, Approve<'info>> {
        let cpi_accounts = Approve {
            delegate: self.vault.to_account_info(),
            authority: self.initiator.to_account_info(),
            to: self.initiator_ata.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    pub fn token_set_authority_ctx<'b, 'c>(
        &self,
    ) -> CpiContext<'_, 'b, 'c, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            current_authority: self.initiator.to_account_info(),
            account_or_mint: self.initiator_ata.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

//     pub fn token_approve_ctx<'b, 'c>(&self) -> CpiContext<'_, 'b, 'c, 'info, Approve<'info>> {
//         let cpi_accounts = Approve {
//             delegate: self.vault.to_account_info(),
//             authority: self.acceptor.to_account_info(),
//             to: self.acceptor_ata.to_account_info(),
//         };
//         CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
//     }
// }
#[derive(Accounts)]
pub struct RevealCoin<'info> {
    #[account(
        init,
        payer = authority,
        seeds = [WINNER_PREFIX.as_bytes(), authority.key().as_ref()],
        bump,
        space = 8 + size_of::<Winner>(),
    )]
    /// CHECK: winner
    pub winner: Account<'info, Winner>,

    /// CHECK: The account flipping
    #[account(mut, signer)]
    pub authority: AccountInfo<'info>,

    /// CHECK: The account creating the flip
    #[account(mut)]
    pub initiator: AccountInfo<'info>,

    /// CHECK: Initiator Token A ATA
    #[account(mut)]
    pub initiator_ata: Box<Account<'info, TokenAccount>>,

    /// CHECK: Admin Token ATA to receive tokens.
    #[account(mut)]
    admin_ata: Box<Account<'info, TokenAccount>>,

    /// CHECK: Token A mint
    #[account(mut)]
    pub mint: Box<Account<'info, Mint>>,

    /// CHECK: PDA storing which is the authority for both ATAs
    #[account(mut)]
    pub vault: Box<Account<'info, Vault>>,

    /// CHECK: PDA for calling the Oracle for random number
    #[account(mut)]
    pub requester: AccountInfo<'info>,

    /// CHECK: degenrand program
    pub degenrand_program: AccountInfo<'info>,

    // pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

impl<'info> RevealCoin<'info> {
    pub fn token_transfer_ctx(
        &self,
        authority: AccountInfo<'info>,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            authority: authority.to_account_info(),
            from,
            to,
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }

    pub fn token_reset_initiator_authority_ctx<'b, 'c>(
        &self,
    ) -> CpiContext<'_, 'b, 'c, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            current_authority: self.vault.to_account_info(),
            account_or_mint: self.initiator_ata.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct Cleanup<'info> {
    /// CHECK: initiator to return amount to
    #[account(mut)]
    pub initiator: Signer<'info>,

    /// PDA holding the winner's status
    #[account(mut)]
    pub winner: Account<'info, Winner>,

    pub system_program: Program<'info, System>,
}

// Used for signing CPI to oracle
#[account(zero_copy)]
#[derive(Debug, Default)]
pub struct Coin {
    pub initiator: Pubkey,
    pub is_flipping: bool,
    pub is_cross: bool,
    pub created_at: i64,
    pub bump: u8,
}
#[derive(Debug, Default, AnchorDeserialize, AnchorSerialize, Clone)]
pub struct CoinInfo {
    mint_token: Pubkey,
    amount: u64,
}

#[account]
#[derive(Debug, Default)]
pub struct Winner {
    winner: Pubkey,
    status: bool,
}

// Used for holding the sol balance and transfering to winner
#[account]
#[derive(Debug, Default)]
pub struct Vault {
    pub coin_info: CoinInfo,
    pub bump: u8,
}

#[error_code]
pub enum DegenErrorCode {
    #[msg("You are not authorized to complete this transaction")]
    Unauthorized,

    #[msg("The coin is has already been flipped")]
    AlreadyCompleted,

    #[msg("A coin is already flipping. Only one flip may be made at a time")]
    InflightRequest,

    #[msg("The Oracle has not provided a response yet")]
    OracleNotCompleted,

    #[msg("Admin Token Pubkey Invalid")]
    InvalidAdminPubkey,

    #[msg("Failed to understand Instruction")]
    FallBacked,
}
