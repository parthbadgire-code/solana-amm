use crate::constants::CONFIG_SEED;
use crate::curve::calculate_swap_output;
use crate::error::AmmError;
use crate::state::Config;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(address = config.mint_x)]
    pub mint_x: Box<Account<'info, Mint>>,

    #[account(address = config.mint_y)]
    pub mint_y: Box<Account<'info, Mint>>,

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
        token::authority = user,
    )]
    pub user_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = mint_y,
        token::authority = user,
    )]
    pub user_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = treasury_x.mint == config.mint_x @ AmmError::InvalidTreasury,
        constraint = treasury_x.owner == config.treasury @ AmmError::InvalidTreasury,
    )]
    pub treasury_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = treasury_y.mint == config.mint_y @ AmmError::InvalidTreasury,
        constraint = treasury_y.owner == config.treasury @ AmmError::InvalidTreasury,
    )]
    pub treasury_y: Box<Account<'info, TokenAccount>>,

    #[account(
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Box<Account<'info, Config>>,

    pub token_program: Program<'info, Token>,
}

pub fn swap_handler(
    ctx: Context<Swap>,
    amount_in: u64,
    min_amount_out: u64,
    is_x_to_y: bool,
) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);
    require!(amount_in > 0, AmmError::ZeroAmount);

    let (reserve_in, reserve_out) = if is_x_to_y {
        (ctx.accounts.vault_x.amount, ctx.accounts.vault_y.amount)
    } else {
        (ctx.accounts.vault_y.amount, ctx.accounts.vault_x.amount)
    };

    let swap_res = calculate_swap_output(
        amount_in,
        reserve_in,
        reserve_out,
        ctx.accounts.config.fee,
        ctx.accounts.config.protocol_fee,
    )?;

    require!(
        swap_res.amount_out >= min_amount_out,
        AmmError::SlippageExceeded
    );

    let seed_bytes = ctx.accounts.config.seed.to_le_bytes();
    let config_bump = ctx.accounts.config.config_bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        CONFIG_SEED,
        seed_bytes.as_ref(),
        &[config_bump],
    ]];

    if is_x_to_y {
        // 1. Transfer protocol fee from user_x to treasury_x if fee > 0
        if swap_res.protocol_fee > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.key(),
                    Transfer {
                        from: ctx.accounts.user_x.to_account_info(),
                        to: ctx.accounts.treasury_x.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                swap_res.protocol_fee,
            )?;
        }

        // 2. Transfer remaining input (effective_in + lp_fee) from user_x to vault_x
        let pool_in = amount_in
            .checked_sub(swap_res.protocol_fee)
            .ok_or(AmmError::MathOverflow)?;

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_x.to_account_info(),
                    to: ctx.accounts.vault_x.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            pool_in,
        )?;

        // 3. Transfer output Token Y from vault_y to user_y
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault_y.to_account_info(),
                    to: ctx.accounts.user_y.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                signer_seeds,
            ),
            swap_res.amount_out,
        )?;
    } else {
        // 1. Transfer protocol fee from user_y to treasury_y if fee > 0
        if swap_res.protocol_fee > 0 {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.key(),
                    Transfer {
                        from: ctx.accounts.user_y.to_account_info(),
                        to: ctx.accounts.treasury_y.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                swap_res.protocol_fee,
            )?;
        }

        // 2. Transfer remaining input (effective_in + lp_fee) from user_y to vault_y
        let pool_in = amount_in
            .checked_sub(swap_res.protocol_fee)
            .ok_or(AmmError::MathOverflow)?;

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.user_y.to_account_info(),
                    to: ctx.accounts.vault_y.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            pool_in,
        )?;

        // 3. Transfer output Token X from vault_x to user_x
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.vault_x.to_account_info(),
                    to: ctx.accounts.user_x.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                signer_seeds,
            ),
            swap_res.amount_out,
        )?;
    }

    Ok(())
}
