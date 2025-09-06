use anchor_lang::prelude::*;

use anchor_lang::{account, prelude::Pubkey, InitSpace};

use crate::{
    LaunchPadErrorCode, LaunchPadTokenCreated, LaunchPadTokenGraduated, LaunchPadTokenStatus,
    DISC_LAUNCH_PAD_TOKEN_ACCOUNT,
};

#[derive(Default, Debug, InitSpace)]
#[account(discriminator = DISC_LAUNCH_PAD_TOKEN_ACCOUNT)]
pub struct LaunchPadToken {
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub token_amount: u64, // reserve 0
    pub asset_amount: u64, // reserve 1
    pub current_k: u128,   // liquidity pool invariant k = x * y
    pub graduated_at: i64,
    pub created_at: i64,
    pub status: LaunchPadTokenStatus,
    pub bump: u8,
}

impl LaunchPadToken {
    pub const SEED: &'static [u8] = b"launch_pad_token:";

    pub fn create(
        &mut self,
        creator: Pubkey,
        mint: Pubkey,
        token_amount: u64,
        asset_amount: u64,
        bump: u8,
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
        self.token_amount = token_amount;
        self.asset_amount = asset_amount;
        self.current_k = (token_amount as u128) * (asset_amount as u128);
        self.created_at = Clock::get()?.unix_timestamp;
        self.bump = bump;
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
