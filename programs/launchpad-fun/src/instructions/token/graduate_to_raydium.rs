#![allow(deprecated, unexpected_cfgs)]
use anchor_lang::{prelude::*, system_program};

use anchor_spl::token::spl_token::instruction::AuthorityType;
use anchor_spl::token::{self, close_account, set_authority, CloseAccount, SetAuthority, Token};
use anchor_spl::token_interface::Mint;

use anchor_lang::solana_program::rent::{
    DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE_YEAR,
};
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022,
    token_interface::{
        token_metadata_initialize, Token2022, TokenAccount, TokenMetadataInitialize,
    },
};
use spl_token_metadata_interface::state::TokenMetadata;
use spl_type_length_value::variable_len_pack::VariableLenPack;

use crate::{
    calc_token_amount_out, initial_virtual_asset_reserve, LaunchPadConfig, LaunchPadErrorCode,
    LaunchPadToken, LaunchPadTokenGraduated, LaunchPadTokenStatus, ProtocolStatus,
    MAX_TOKEN_NAME_LENGTH, MAX_TOKEN_SYMBOL_LENGTH, MAX_TOKEN_URI_LENGTH, MIN_TOKEN_NAME_LENGTH,
    MIN_TOKEN_SYMBOL_LENGTH, MIN_TOKEN_URI_LENGTH, RAYDIUM_CPMM_ID, TOKEN_GRADUATION_AMOUNT,
    TOKEN_TOTAL_SUPPLY,
};

use raydium_cpmm_cpi::{
    cpi,
    program::RaydiumCpmm,
    states::{AmmConfig, OBSERVATION_SEED, POOL_LP_MINT_SEED, POOL_SEED, POOL_VAULT_SEED},
};

#[derive(Accounts)]
pub struct GraduateToRaydium<'info> {
    #[account(mut)]
    pub investor: Signer<'info>,

    #[account(
        seeds = [LaunchPadConfig::SEED],
        bump
    )]
    pub launch_pad_config: Box<Account<'info, LaunchPadConfig>>,

    #[account(
        mut,
        seeds = [LaunchPadConfig::VAULT_SEED],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    #[account()]
    pub mint: Box<InterfaceAccount<'info, Mint>>,

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
    pub launch_pad_token: Box<Account<'info, LaunchPadToken>>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = launch_pad_config,
        associated_token::token_program = token_program_2022,
    )]
    pub launch_pad_token_account: InterfaceAccount<'info, TokenAccount>,

    //// Raydium related accounts
    #[account(address = RAYDIUM_CPMM_ID)]
    pub cpmm_program: Program<'info, RaydiumCpmm>,

    /// Which config the pool belongs to.
    pub amm_config: Box<Account<'info, AmmConfig>>,

    /// CHECK: pool vault and lp mint authority
    #[account(
        seeds = [
            raydium_cpmm_cpi::AUTH_SEED.as_bytes(),
        ],
        seeds::program = cpmm_program,
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// CHECK: Initialize an account to store the pool state, init by cp-swap
    #[account(
        mut,
        // seeds = [
        //     POOL_SEED.as_bytes(),
        //     amm_config.key().as_ref(),
        //     token_0_mint.key().as_ref(),
        //     token_1_mint.key().as_ref(),
        // ],
        // seeds::program = cpmm_program,
        // bump,
    )]
    pub pool_state: UncheckedAccount<'info>,

    #[account(
        mint::token_program = token_program,
    )]
    pub wsol_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: pool lp mint, init by cp-swap
    #[account(
        mut,
        seeds = [
            POOL_LP_MINT_SEED.as_bytes(),
            pool_state.key().as_ref(),
        ],
        seeds::program = cpmm_program,
        bump,
    )]
    pub lp_mint: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = investor,
        seeds = [
            LaunchPadToken::VAULT_TOKEN_GRADUATION_SEED,
            launch_pad_token.key().as_ref(),
        ],
        bump,
        token::mint = mint,
        token::authority = investor,
        token::token_program = token_program_2022,
    )]
    pub vault_graduation_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = investor,
        seeds = [
            LaunchPadToken::VAULT_ASSET_GRADUATION_SEED,
            launch_pad_token.key().as_ref(),
        ],
        bump,
        token::mint = wsol_mint,
        token::authority = investor,
        token::token_program = token_program,
    )]
    pub vault_asset_graduation_token_account: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: creator lp ATA token account, init by cp-swap
    #[account(mut)]
    pub lp_token: UncheckedAccount<'info>,

    /// CHECK: Token_0 vault for the pool, init by cp-swap
    #[account(
        mut,
        // seeds = [
        //     POOL_VAULT_SEED.as_bytes(),
        //     pool_state.key().as_ref(),
        //     token_0_mint.key().as_ref()
        // ],
        // seeds::program = cpmm_program,
        // bump,
    )]
    pub token_0_vault: UncheckedAccount<'info>,

    /// CHECK: Token_1 vault for the pool, init by cp-swap
    #[account(
        mut,
        // seeds = [
        //     POOL_VAULT_SEED.as_bytes(),
        //     pool_state.key().as_ref(),
        //     token_1_mint.key().as_ref()
        // ],
        // seeds::program = cpmm_program,
        // bump,
    )]
    pub token_1_vault: UncheckedAccount<'info>,

    /// create pool fee account
    #[account(
        mut,
        address = raydium_cpmm_cpi::create_pool_fee_reveiver::id(),
    )]
    pub create_pool_fee: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: an account to store oracle observations, init by cp-swap
    #[account(
        mut,
        seeds = [
            OBSERVATION_SEED.as_bytes(),
            pool_state.key().as_ref(),
        ],
        seeds::program = cpmm_program,
        bump,
    )]
    pub observation_state: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_program_2022: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> GraduateToRaydium<'info> {
    pub fn graduate(&mut self) -> Result<()> {
        require!(
            self.launch_pad_config.status == ProtocolStatus::Active,
            LaunchPadErrorCode::ProtocolConfigNotActive
        );
        if self.launch_pad_token.status == LaunchPadTokenStatus::Graduated {
            return Err(LaunchPadErrorCode::LaunchPadTokenTradingNotEnabled.into());
        }
        if self.launch_pad_token.status != LaunchPadTokenStatus::ReadyToGraduate {
            return Ok(());
        }
        require!(
            self.launch_pad_token.status == LaunchPadTokenStatus::ReadyToGraduate,
            LaunchPadErrorCode::LaunchPadTokenTradingNotEnabled
        );
        self.wrap_sol_to_graduation()?;
        self.transfer_tokens_to_graduation(self.launch_pad_config.bump)?;
        self.vault_asset_graduation_token_account.reload()?;
        self.vault_graduation_token_account.reload()?;
        let asset_amount = self.vault_asset_graduation_token_account.amount;
        let token_amount = self.vault_graduation_token_account.amount;
        self.create_pool()?;
        self.launch_pad_token.graduate()?;
        self.vault_asset_graduation_token_account.reload()?;
        self.vault_graduation_token_account.reload()?;
        self.launch_pad_token_account.reload()?;

        self.close_token_account_2022(
            &self.vault_graduation_token_account.to_account_info(),
            &self.investor.to_account_info(),
            &self.investor.to_account_info(),
            &[],
        )?;
        self.close_token_account(
            &self.vault_asset_graduation_token_account.to_account_info(),
            &self.investor.to_account_info(),
            &self.investor.to_account_info(),
            &[],
        )?;
        let launch_pad_config_bump = self.launch_pad_config.bump;
        let signer: &[&[&[u8]]] = &[&[LaunchPadConfig::SEED, &[launch_pad_config_bump]]];
        self.close_token_account_2022(
            &self.launch_pad_token_account.to_account_info(),
            &self.launch_pad_config.to_account_info(),
            &self.vault.to_account_info(),
            signer,
        )?;

        emit!(LaunchPadTokenGraduated {
            mint: self.mint.key(),
            lp_mint: self.lp_mint.key(),
            lp_token: self.lp_token.key(),
            pool_state: self.pool_state.key(),
            asset_amount: asset_amount,
            token_amount: token_amount,
            status: LaunchPadTokenStatus::Graduated,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    fn create_pool(&self) -> Result<()> {
        let mut creator_token_0 = self.vault_asset_graduation_token_account.to_account_info();
        let mut creator_token_1 = self.vault_graduation_token_account.to_account_info();
        let mut token_0_mint = self.wsol_mint.to_account_info();
        let mut token_1_mint = self.mint.to_account_info();
        let mut token_0_program = self.token_program.to_account_info();
        let mut token_1_program = self.token_program_2022.to_account_info();
        let mut init_amount_0 = self.vault_asset_graduation_token_account.amount;
        let mut init_amount_1 = self.vault_graduation_token_account.amount;

        // Token_0 mint, the key must smaller then token_1 mint.
        if self.wsol_mint.key() > self.mint.key() {
            creator_token_0 = self.vault_graduation_token_account.to_account_info();
            creator_token_1 = self.vault_asset_graduation_token_account.to_account_info();
            token_0_mint = self.mint.to_account_info();
            token_1_mint = self.wsol_mint.to_account_info();
            token_0_program = self.token_program_2022.to_account_info();
            token_1_program = self.token_program.to_account_info();
            init_amount_0 = self.vault_graduation_token_account.amount;
            init_amount_1 = self.vault_asset_graduation_token_account.amount;
        }

        let cpi_accounts = cpi::accounts::Initialize {
            creator: self.investor.to_account_info(),
            amm_config: self.amm_config.to_account_info(),
            authority: self.authority.to_account_info(),
            pool_state: self.pool_state.to_account_info(),
            token_0_mint: token_0_mint,
            token_1_mint: token_1_mint,
            lp_mint: self.lp_mint.to_account_info(),
            creator_token_0: creator_token_0,
            creator_token_1: creator_token_1,
            creator_lp_token: self.lp_token.to_account_info(),
            token_0_vault: self.token_0_vault.to_account_info(),
            token_1_vault: self.token_1_vault.to_account_info(),
            create_pool_fee: self.create_pool_fee.to_account_info(),
            observation_state: self.observation_state.to_account_info(),
            token_program: self.token_program.to_account_info(),
            token_0_program: token_0_program,
            token_1_program: token_1_program,
            associated_token_program: self.associated_token_program.to_account_info(),
            system_program: self.system_program.to_account_info(),
            rent: self.rent.to_account_info(),
        };
        let cpi_context = CpiContext::new(self.cpmm_program.to_account_info(), cpi_accounts);
        cpi::initialize(cpi_context, init_amount_0, init_amount_1, 0)?;

        set_authority(
            CpiContext::new(
                self.token_program.to_account_info(),
                SetAuthority {
                    current_authority: self.investor.to_account_info(),
                    account_or_mint: self.lp_token.to_account_info(),
                },
            ),
            AuthorityType::AccountOwner,
            Some(self.launch_pad_config.key()),
        )?;
        Ok(())
    }

    fn wrap_sol_to_graduation(&self) -> Result<()> {
        let launch_pad_vault_bump = self.launch_pad_token.vault_bump;
        let signer: &[&[&[u8]]] = &[&[
            LaunchPadToken::VAULT_SEED,
            self.mint.to_account_info().key.as_ref(),
            &[launch_pad_vault_bump],
        ]];
        let lamports = self.vault_graduation.to_account_info().lamports();
        let cpi_context = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            system_program::Transfer {
                from: self.vault_graduation.to_account_info(),
                to: self.vault_asset_graduation_token_account.to_account_info(),
            },
            signer,
        );
        system_program::transfer(cpi_context, lamports)?;

        // Sync the native token to reflect the new SOL balance as wSOL
        let cpi_accounts = token::SyncNative {
            account: self.vault_asset_graduation_token_account.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::sync_native(cpi_ctx)?;

        // Zero out account data
        {
            let mut data = self.vault_graduation.try_borrow_mut_data()?;
            for byte in data.iter_mut() {
                *byte = 0;
            }
        }

        // Set the closed account discriminator (8 bytes of 0xFF)
        {
            let mut data = self.vault_graduation.try_borrow_mut_data()?;
            if data.len() >= 8 {
                let closed_discriminator = [0xFF; 8]; // Standard closed account marker
                data[0..8].copy_from_slice(&closed_discriminator);
            }
        }
        Ok(())
    }

    fn transfer_tokens_to_graduation(&self, launch_pad_config_bump: u8) -> Result<()> {
        let signer: &[&[&[u8]]] = &[&[LaunchPadConfig::SEED, &[launch_pad_config_bump]]];
        let amount = self.launch_pad_token_account.amount;
        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                self.token_program_2022.to_account_info(),
                token_2022::TransferChecked {
                    from: self.launch_pad_token_account.to_account_info(),
                    to: self.vault_graduation_token_account.to_account_info(),
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

    fn close_token_account(
        &self,
        account_to_close: &AccountInfo<'info>,
        owner: &AccountInfo<'info>,
        beneficiary: &AccountInfo<'info>,
        signer: &[&[&[u8]]],
    ) -> Result<()> {
        let close_accounts = CloseAccount {
            account: account_to_close.to_account_info(),
            destination: beneficiary.to_account_info(),
            authority: owner.to_account_info(),
        };

        let close_cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            close_accounts,
            signer,
        );

        close_account(close_cpi_ctx)?;
        Ok(())
    }

    fn close_token_account_2022(
        &self,
        account_to_close: &AccountInfo<'info>,
        owner: &AccountInfo<'info>,
        beneficiary: &AccountInfo<'info>,
        signer: &[&[&[u8]]],
    ) -> Result<()> {
        let close_accounts = token_2022::CloseAccount {
            account: account_to_close.to_account_info(),
            destination: beneficiary.to_account_info(),
            authority: owner.to_account_info(),
        };

        let close_cpi_ctx = CpiContext::new_with_signer(
            self.token_program_2022.to_account_info(),
            close_accounts,
            signer,
        );

        token_2022::close_account(close_cpi_ctx)?;
        Ok(())
    }
}

pub fn handler(ctx: Context<GraduateToRaydium>) -> Result<()> {
    ctx.accounts.graduate()?;
    msg!("Launch Pad token graduated successfully");
    Ok(())
}
