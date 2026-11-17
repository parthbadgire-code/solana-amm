pub mod constants;
pub mod curve;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use curve::*;
pub use error::*;
pub use instructions::*;
pub use state::*;

declare_id!("3DrD6CHNhDJPvV1u1twzbmUGqcD6JHXJ6Xu3TmvxRsNQ");

#[program]
pub mod solana_amm {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        fee: u16,
        protocol_fee: u16,
        authority: Pubkey,
    ) -> Result<()> {
        instructions::initialize_handler(ctx, seed, fee, protocol_fee, authority)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount_lp: u64,
        max_x: u64,
        max_y: u64,
    ) -> Result<()> {
        instructions::deposit_handler(ctx, amount_lp, max_x, max_y)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount_lp: u64,
        min_x: u64,
        min_y: u64,
    ) -> Result<()> {
        instructions::withdraw_handler(ctx, amount_lp, min_x, min_y)
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        is_x_to_y: bool,
    ) -> Result<()> {
        instructions::swap_handler(ctx, amount_in, min_amount_out, is_x_to_y)
    }

    pub fn update_fee(
        ctx: Context<Admin>,
        new_fee: u16,
        new_protocol_fee: u16,
    ) -> Result<()> {
        instructions::update_fee_handler(ctx, new_fee, new_protocol_fee)
    }

    pub fn update_treasury(
        ctx: Context<Admin>,
        new_treasury: Pubkey,
    ) -> Result<()> {
        instructions::update_treasury_handler(ctx, new_treasury)
    }

    pub fn set_locked(
        ctx: Context<Admin>,
        locked: bool,
    ) -> Result<()> {
        instructions::set_locked_handler(ctx, locked)
    }
}
