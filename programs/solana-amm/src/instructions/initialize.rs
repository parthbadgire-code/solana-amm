use crate::constants::{CONFIG_SEED, LP_MINT_SEED, MAX_FEE_BPS};
use crate::error::AmmError;
use crate::state::Config;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub initializer: Signer<'info>,

    pub mint_x: Box<Account<'info, Mint>>,
    pub mint_y: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        seeds = [LP_MINT_SEED, config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub mint_lp: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = initializer,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Box<Account<'info, TokenAccount>>,

    /// CHECK: Treasury account (wallet or address that receives protocol fees)
    pub treasury: UncheckedAccount<'info>,

    #[account(
        init,
        payer = initializer,
        seeds = [CONFIG_SEED, seed.to_le_bytes().as_ref()],
        bump,
        space = 8 + Config::INIT_SPACE,
    )]
    pub config: Box<Account<'info, Config>>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(
    ctx: Context<Initialize>,
    seed: u64,
    fee: u16,
    protocol_fee: u16,
    authority: Pubkey,
) -> Result<()> {
    require!(
        ctx.accounts.mint_x.key() != ctx.accounts.mint_y.key(),
        AmmError::IdenticalMints
    );
    require!(fee <= MAX_FEE_BPS, AmmError::FeeTooHigh);
    require!(protocol_fee <= fee, AmmError::InvalidFee);

    let config = &mut ctx.accounts.config;
    config.seed = seed;
    config.authority = authority;
    config.mint_x = ctx.accounts.mint_x.key();
    config.mint_y = ctx.accounts.mint_y.key();
    config.fee = fee;
    config.protocol_fee = protocol_fee;
    config.treasury = ctx.accounts.treasury.key();
    config.locked = false;
    config.config_bump = ctx.bumps.config;
    config.lp_bump = ctx.bumps.mint_lp;

    Ok(())
}
