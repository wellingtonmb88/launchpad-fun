use anchor_lang::{prelude::Pubkey, solana_program::pubkey};

pub const TOKEN_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000_000; // 1 billion (1e9 * 1e9)
pub const TOKEN_GRADUATION_AMOUNT: u128 = 200_000_000_000_000_000; // 200 millions (200e6 * 1e9)
pub const K: u64 = 3_000_000_000_000;
pub const ASSET_RATE: u64 = 7;

pub const MAX_TOKEN_NAME_LENGTH: usize = 32;
pub const MIN_TOKEN_NAME_LENGTH: usize = 3;
pub const MAX_TOKEN_SYMBOL_LENGTH: usize = 10;
pub const MIN_TOKEN_SYMBOL_LENGTH: usize = 3;
pub const MAX_TOKEN_URI_LENGTH: usize = 200;
pub const MIN_TOKEN_URI_LENGTH: usize = 10;

#[cfg(feature = "devnet")]
pub const RAYDIUM_CPMM_ID: Pubkey = pubkey!("DRaycpLY18LhpbydsBWbVJtxpNv9oXPgjRSfpF2bWpYb"); // Raydium on devnet

#[cfg(not(feature = "devnet"))]
pub const RAYDIUM_CPMM_ID: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"); // Raydium on mainnet
