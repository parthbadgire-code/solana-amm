use crate::constants::{CONFIG_SEED, LP_MINT_SEED};
use crate::curve::{calculate_deposit_amounts_from_lp, calculate_initial_lp};
use crate::error::AmmError;
use crate::state::Config;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(address = config.mint_x)]
    pub mint_x: Box<Account<'info, Mint>>,

    #[account(address = config.mint_y)]
    pub mint_y: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [LP_MINT_SEED, config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_x,
        token::authority = depositor,
    )]
    pub depositor_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_y,
        token::authority = depositor,
    )]
    pub depositor_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_lp,
        token::authority = depositor,
    )]
    pub depositor_lp: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,

    pub token_program: Program<'info, Token>,
}

pub fn deposit_handler(
    ctx: Context<Deposit>,
    amount_lp: u64,
    max_x: u64,
    max_y: u64,
) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);

    let reserve_x = ctx.accounts.vault_x.amount;
    let reserve_y = ctx.accounts.vault_y.amount;
    let total_lp = ctx.accounts.mint_lp.supply;

    let (dx, dy, lp_to_mint) = if total_lp == 0 && reserve_x == 0 && reserve_y == 0 {
        // Initial liquidity bootstrap
        require!(max_x > 0 && max_y > 0, AmmError::ZeroAmount);
        let initial_lp = calculate_initial_lp(max_x, max_y)?;
        if amount_lp > 0 {
            require!(initial_lp >= amount_lp, AmmError::SlippageExceeded);
        }
        (max_x, max_y, initial_lp)
    } else {
        // Subsequent liquidity provision
        require!(amount_lp > 0, AmmError::ZeroAmount);
        let (required_x, required_y) = calculate_deposit_amounts_from_lp(
            amount_lp,
            reserve_x,
            reserve_y,
            total_lp,
        )?;
        require!(
            required_x <= max_x && required_y <= max_y,
            AmmError::MaxInputExceeded
        );
        (required_x, required_y, amount_lp)
    };

    // 1. Transfer Token X from depositor to vault_x
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.depositor_x.to_account_info(),
                to: ctx.accounts.vault_x.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        dx,
    )?;

    // 2. Transfer Token Y from depositor to vault_y
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.depositor_y.to_account_info(),
                to: ctx.accounts.vault_y.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        ),
        dy,
    )?;

    // 3. Mint LP tokens to depositor
    let seed_bytes = ctx.accounts.config.seed.to_le_bytes();
    let config_bump = ctx.accounts.config.config_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        CONFIG_SEED,
        seed_bytes.as_ref(),
        &[config_bump],
    ]];

    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            MintTo {
                mint: ctx.accounts.mint_lp.to_account_info(),
                to: ctx.accounts.depositor_lp.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer_seeds,
        ),
        lp_to_mint,
    )?;

    Ok(())
}
