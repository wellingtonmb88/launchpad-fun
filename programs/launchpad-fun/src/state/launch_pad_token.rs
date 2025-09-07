use anchor_lang::prelude::*;

use anchor_lang::{account, prelude::Pubkey, InitSpace};

use crate::{
    LaunchPadErrorCode, LaunchPadTokenCreated, LaunchPadTokenGraduated, LaunchPadTokenStatus,
    DISC_LAUNCH_PAD_TOKEN_ACCOUNT,
};

#[derive(Default, Debug, InitSpace)]
#[account(discriminator = DISC_LAUNCH_PAD_TOKEN_ACCOUNT)]
pub struct LaunchPadToken {
    /// The creator of the launch pad token
    pub creator: Pubkey,
    /// The mint address of the launch pad token
    pub mint: Pubkey,
    /// Total virtual reserve of the token
    pub virtual_token_amount: u64,
    /// Total virtual reserve of the asset
    pub virtual_asset_amount: u64,
    /// The liquidity pool invariant k = x * y
    pub current_k: u128,
    /// Total virtual reserve amount for graduation
    pub virtual_graduation_amount: u64,
    /// The timestamp when the token graduated
    pub graduated_at: i64,
    /// The timestamp when the token was created
    pub created_at: i64,
    /// The current status of the launch pad token
    pub status: LaunchPadTokenStatus,
    /// The bump seed for the PDA
    pub bump: u8,
    // The vault graduation bump seed for the PDA
    pub vault_bump: u8,
}

impl LaunchPadToken {
    pub const SEED: &'static [u8] = b"launch_pad_token:";
    pub const VAULT_SEED: &'static [u8] = b"vault_graduation:";

    pub fn create(
        &mut self,
        creator: Pubkey,
        mint: Pubkey,
        token_amount: u64,
        asset_amount: u64,
        bump: u8,
        vault_bump: u8,
    ) -> Result<()> {
        require!(
            self.status == LaunchPadTokenStatus::Unknown,
            LaunchPadErrorCode::LaunchPadTokenAlreadyCreated
        );
        require!(
            creator != Pubkey::default(),
            LaunchPadErrorCode::InvalidCreator
        );
        require!(mint != Pubkey::default(), LaunchPadErrorCode::InvalidMint);
        self.creator = creator;
        self.mint = mint;
        self.virtual_token_amount = token_amount;
        self.virtual_asset_amount = asset_amount;
        self.current_k = (token_amount as u128)
            .checked_mul(asset_amount as u128)
            .ok_or(LaunchPadErrorCode::MathOverflow)?;
        self.virtual_graduation_amount = 0;
        self.created_at = Clock::get()?.unix_timestamp;
        self.bump = bump;
        self.vault_bump = vault_bump;
        self.status = LaunchPadTokenStatus::TradingEnabled;

        emit!(LaunchPadTokenCreated {
            creator: self.creator,
            mint: self.mint,
            status: self.status,
            timestamp: self.created_at,
        });
        Ok(())
    }

    pub fn graduate(&mut self) -> Result<()> {
        require!(
            self.status == LaunchPadTokenStatus::TradingEnabled,
            LaunchPadErrorCode::LaunchPadTokenAlreadyGraduated
        );
        self.status = LaunchPadTokenStatus::Graduated;
        self.graduated_at = Clock::get()?.unix_timestamp;

        emit!(LaunchPadTokenGraduated {
            mint: self.mint,
            status: self.status,
            timestamp: self.graduated_at,
        });
        Ok(())
    }
}
