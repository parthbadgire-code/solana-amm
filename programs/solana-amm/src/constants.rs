use anchor_lang::prelude::*;

pub const CONFIG_SEED: &[u8] = b"config";
pub const LP_MINT_SEED: &[u8] = b"lp";
pub const MAX_FEE_BPS: u16 = 1000; // 10.00% max total fee
pub const BPS_DENOMINATOR: u128 = 10_000;
