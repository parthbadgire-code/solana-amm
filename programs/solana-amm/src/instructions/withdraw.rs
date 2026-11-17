use crate::constants::{CONFIG_SEED, LP_MINT_SEED};
use crate::curve::calculate_withdraw_amounts;
use crate::error::AmmError;
use crate::state::Config;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub withdrawer: Signer<'info>,

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
        token::authority = withdrawer,
    )]
    pub withdrawer_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_y,
        token::authority = withdrawer,
    )]
    pub withdrawer_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_lp,
        token::authority = withdrawer,
    )]
    pub withdrawer_lp: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,

    pub token_program: Program<'info, Token>,
}

pub fn withdraw_handler(
    ctx: Context<Withdraw>,
    amount_lp: u64,
    min_x: u64,
    min_y: u64,
) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);
    require!(amount_lp > 0, AmmError::ZeroAmount);

    let reserve_x = ctx.accounts.vault_x.amount;
    let reserve_y = ctx.accounts.vault_y.amount;
    let total_lp = ctx.accounts.mint_lp.supply;

    let (dx, dy) = calculate_withdraw_amounts(amount_lp, reserve_x, reserve_y, total_lp)?;

    require!(dx >= min_x && dy >= min_y, AmmError::SlippageExceeded);

    // 1. Burn LP tokens from withdrawer
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            Burn {
                mint: ctx.accounts.mint_lp.to_account_info(),
                from: ctx.accounts.withdrawer_lp.to_account_info(),
                authority: ctx.accounts.withdrawer.to_account_info(),
            },
        ),
        amount_lp,
    )?;

    // Signer seeds for config PDA
    let seed_bytes = ctx.accounts.config.seed.to_le_bytes();
    let config_bump = ctx.accounts.config.config_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        CONFIG_SEED,
        seed_bytes.as_ref(),
        &[config_bump],
    ]];

    // 2. Transfer Token X from vault_x to withdrawer
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.vault_x.to_account_info(),
                to: ctx.accounts.withdrawer_x.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer_seeds,
        ),
        dx,
    )?;

    // 3. Transfer Token Y from vault_y to withdrawer
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            Transfer {
                from: ctx.accounts.vault_y.to_account_info(),
                to: ctx.accounts.withdrawer_y.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer_seeds,
        ),
        dy,
    )?;

    Ok(())
}
