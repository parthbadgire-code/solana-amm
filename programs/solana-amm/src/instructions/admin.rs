use crate::constants::{CONFIG_SEED, MAX_FEE_BPS};
use crate::error::AmmError;
use crate::state::Config;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Admin<'info> {
    #[account(
        mut,
        address = config.authority @ AmmError::Unauthorized,
    )]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED, config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
}

pub fn update_fee_handler(ctx: Context<Admin>, new_fee: u16, new_protocol_fee: u16) -> Result<()> {
    require!(new_fee <= MAX_FEE_BPS, AmmError::FeeTooHigh);
    require!(new_protocol_fee <= new_fee, AmmError::InvalidFee);

    let config = &mut ctx.accounts.config;
    config.fee = new_fee;
    config.protocol_fee = new_protocol_fee;

    Ok(())
}

pub fn update_treasury_handler(ctx: Context<Admin>, new_treasury: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.treasury = new_treasury;

    Ok(())
}

pub fn set_locked_handler(ctx: Context<Admin>, locked: bool) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.locked = locked;

    Ok(())
}
