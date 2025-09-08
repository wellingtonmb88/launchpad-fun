#![allow(deprecated, unexpected_cfgs)]
use anchor_lang::prelude::*;

use anchor_spl::token_interface::Mint;

use anchor_lang::solana_program::rent::{
    DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE_YEAR,
};
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022,
    token_interface::{
        spl_token_2022::instruction::AuthorityType, token_metadata_initialize, Token2022,
        TokenAccount, TokenMetadataInitialize,
    },
};
use spl_token_metadata_interface::state::TokenMetadata;
use spl_type_length_value::variable_len_pack::VariableLenPack;

use crate::{
    calc_asset_amount_out, calc_token_amount_out, initial_virtual_asset_reserve, LaunchPadConfig,
    LaunchPadErrorCode, LaunchPadToken, LaunchPadTokenStatus, ProtocolStatus,
    MAX_TOKEN_NAME_LENGTH, MAX_TOKEN_SYMBOL_LENGTH, MAX_TOKEN_URI_LENGTH, MIN_TOKEN_NAME_LENGTH,
    MIN_TOKEN_SYMBOL_LENGTH, MIN_TOKEN_URI_LENGTH, TOKEN_GRADUATION_AMOUNT, TOKEN_TOTAL_SUPPLY,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct SellTokenArgs {
    pub amount: u64,
}

#[derive(Accounts)]
pub struct SellToken<'info> {
    #[account(mut)]
    pub investor: Signer<'info>,

    #[account(
        seeds = [LaunchPadConfig::SEED],
        bump
    )]
    pub launch_pad_config: Account<'info, LaunchPadConfig>,

    #[account(
        mut,
        seeds = [LaunchPadConfig::VAULT_SEED],
        bump = launch_pad_config.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    #[account()]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [LaunchPadToken::VAULT_SEED, mint.key().as_ref()],
        bump = launch_pad_token.vault_bump,
    )]
    pub vault_graduation: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [LaunchPadToken::SEED, mint.key().as_ref()],
        bump = launch_pad_token.bump,
    )]
    pub launch_pad_token: Account<'info, LaunchPadToken>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = launch_pad_config,
        associated_token::token_program = token_program,
    )]
    pub launch_pad_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = investor,
        associated_token::mint = mint,
        associated_token::authority = investor,
        associated_token::token_program = token_program,
    )]
    pub investor_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> SellToken<'info> {
    pub fn sell_token(&mut self, args: SellTokenArgs) -> Result<()> {
        require!(
            self.launch_pad_config.status == ProtocolStatus::Active,
            LaunchPadErrorCode::ProtocolConfigNotActive
        );
        require!(
            self.launch_pad_token.status == LaunchPadTokenStatus::TradingEnabled,
            LaunchPadErrorCode::LaunchPadTokenTradingNotEnabled
        );

        let SellTokenArgs {
            amount: token_amount_in,
        } = args;
        let launch_pad_vault_bump = self.launch_pad_token.vault_bump;
        let current_asset_supply = self.launch_pad_token.virtual_asset_amount;
        let current_token_supply = self.launch_pad_token.virtual_token_amount;
        let current_k = self.launch_pad_token.current_k;

        let asset_amount_out = calc_asset_amount_out(
            token_amount_in,
            current_k,
            current_token_supply as u128,
            current_asset_supply as u128,
        )? as u64;

        let sell_fee = self
            .launch_pad_config
            .calculate_sell_fee(asset_amount_out)?;
        let asset_amount_out_with_fee = asset_amount_out
            .checked_sub(sell_fee)
            .ok_or(LaunchPadErrorCode::MathOverflow)?;

        require!(
            asset_amount_out <= current_asset_supply,
            LaunchPadErrorCode::InsufficientAssetLiquidity
        );

        self.transfer_tokens_from_investor(token_amount_in)?;
        self.transfer_assets_to_investor(asset_amount_out_with_fee, launch_pad_vault_bump)?;
        self.transfer_sell_fee(sell_fee, launch_pad_vault_bump)?;

        self.launch_pad_token.update_virtual_reserves(
            current_token_supply
                .checked_add(token_amount_in)
                .ok_or(LaunchPadErrorCode::MathOverflow)?,
            current_asset_supply
                .checked_sub(asset_amount_out)
                .ok_or(LaunchPadErrorCode::MathOverflow)?,
        )?;

        self.launch_pad_token
            .decrease_virtual_graduation_amount(asset_amount_out)?;

        Ok(())
    }

    fn transfer_assets_to_investor(&self, amount: u64, launch_pad_vault_bump: u8) -> Result<()> {
        let signer: &[&[&[u8]]] = &[&[
            LaunchPadToken::VAULT_SEED,
            self.mint.to_account_info().key.as_ref(),
            &[launch_pad_vault_bump],
        ]];
        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.vault_graduation.to_account_info(),
                    to: self.investor.to_account_info(),
                },
                signer,
            ),
            amount,
        )?;

        Ok(())
    }

    fn transfer_tokens_from_investor(&self, amount: u64) -> Result<()> {
        token_2022::transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.investor_token_account.to_account_info(),
                    to: self.launch_pad_token_account.to_account_info(),
                    authority: self.investor.to_account_info(),
                    mint: self.mint.to_account_info(),
                },
            ),
            amount,
            9,
        )?;
        Ok(())
    }

    fn transfer_sell_fee(&self, amount: u64, launch_pad_vault_bump: u8) -> Result<()> {
        let signer: &[&[&[u8]]] = &[&[
            LaunchPadToken::VAULT_SEED,
            self.mint.to_account_info().key.as_ref(),
            &[launch_pad_vault_bump],
        ]];
        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.vault_graduation.to_account_info(),
                    to: self.vault.to_account_info(),
                },
                signer,
            ),
            amount,
        )?;

        Ok(())
    }
}

pub fn handler(ctx: Context<SellToken>, args: SellTokenArgs) -> Result<()> {
    ctx.accounts.sell_token(args)?;
    msg!("Launch Pad token sold successfully");
    Ok(())
}
