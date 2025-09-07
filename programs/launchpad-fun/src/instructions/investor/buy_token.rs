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
    calc_token_amount_out, initial_virtual_asset_reserve, LaunchPadConfig, LaunchPadErrorCode,
    LaunchPadToken, LaunchPadTokenStatus, ProtocolStatus, MAX_TOKEN_NAME_LENGTH,
    MAX_TOKEN_SYMBOL_LENGTH, MAX_TOKEN_URI_LENGTH, MIN_TOKEN_NAME_LENGTH, MIN_TOKEN_SYMBOL_LENGTH,
    MIN_TOKEN_URI_LENGTH, TOKEN_GRADUATION_AMOUNT, TOKEN_TOTAL_SUPPLY,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct BuyTokenArgs {
    pub amount: u64,
}

#[derive(Accounts)]
pub struct BuyToken<'info> {
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

impl<'info> BuyToken<'info> {
    pub fn create(&mut self, args: BuyTokenArgs, bumps: BuyTokenBumps) -> Result<()> {
        require!(
            self.launch_pad_config.status == ProtocolStatus::Active,
            LaunchPadErrorCode::ProtocolConfigNotActive
        );
        require!(
            self.launch_pad_token.status == LaunchPadTokenStatus::TradingEnabled,
            LaunchPadErrorCode::LaunchPadTokenTradingNotEnabled
        );

        let BuyTokenArgs { amount } = args;
        let launch_pad_config_bump = bumps.launch_pad_config;
        let current_asset_supply = self.launch_pad_token.virtual_asset_amount;
        let current_token_supply = self.launch_pad_token.virtual_token_amount;
        let current_k = self.launch_pad_token.current_k;

        let buy_fee = self.launch_pad_config.calculate_buy_fee(amount)?;
        let amount = amount
            .checked_sub(buy_fee)
            .ok_or(LaunchPadErrorCode::MathOverflow)?;

        let token_amount_out = calc_token_amount_out(
            amount,
            current_k,
            current_asset_supply as u128,
            current_token_supply as u128,
        )? as u64;

        let total_supply_minus_graduation = current_token_supply
            .checked_sub(TOKEN_GRADUATION_AMOUNT as u64)
            .ok_or(LaunchPadErrorCode::MathOverflow)?;

        require!(
            token_amount_out <= total_supply_minus_graduation,
            LaunchPadErrorCode::InsufficientTokenLiquidity
        );

        self.transfer_tokens_to_investor(token_amount_out, launch_pad_config_bump)?;
        self.transfer_assets_from_investor_to(amount, &self.vault_graduation.to_account_info())?;
        self.transfer_assets_from_investor_to(buy_fee, &self.vault.to_account_info())?;

        self.launch_pad_token.update_virtual_reserves(
            current_token_supply
                .checked_sub(token_amount_out)
                .ok_or(LaunchPadErrorCode::MathOverflow)?,
            current_asset_supply
                .checked_add(amount)
                .ok_or(LaunchPadErrorCode::MathOverflow)?,
        )?;

        self.launch_pad_token
            .increase_virtual_graduation_amount(amount)?;

        if self.launch_pad_token.virtual_graduation_amount
            >= self.launch_pad_config.graduate_threshold
        {
            self.launch_pad_token
                .update_status(LaunchPadTokenStatus::ReadyToGraduate)?;
        }

        Ok(())
    }

    fn transfer_assets_from_investor_to(
        &self,
        amount: u64,
        destination: &AccountInfo<'info>,
    ) -> Result<()> {
        // Transfer additional lamports to account
        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.investor.to_account_info(),
                    to: destination.clone(),
                },
            ),
            amount,
        )?;

        Ok(())
    }

    fn transfer_tokens_to_investor(&self, amount: u64, launch_pad_config_bump: u8) -> Result<()> {
        let signer: &[&[&[u8]]] = &[&[LaunchPadConfig::SEED, &[launch_pad_config_bump]]];

        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.launch_pad_token_account.to_account_info(),
                    to: self.investor_token_account.to_account_info(),
                    authority: self.launch_pad_config.to_account_info(),
                    mint: self.mint.to_account_info(),
                },
                signer,
            ),
            amount,
            9,
        )?;
        Ok(())
    }
}

pub fn handler(ctx: Context<BuyToken>, args: BuyTokenArgs) -> Result<()> {
    ctx.accounts.create(args, ctx.bumps)?;
    msg!("Launch Pad token bought successfully");
    Ok(())
}
