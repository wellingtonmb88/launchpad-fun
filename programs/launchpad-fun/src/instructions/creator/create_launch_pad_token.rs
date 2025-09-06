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
    LaunchPadConfig, LaunchPadErrorCode, LaunchPadToken, ProtocolStatus, MAX_TOKEN_NAME_LENGTH,
    MAX_TOKEN_SYMBOL_LENGTH, MAX_TOKEN_URI_LENGTH, MIN_TOKEN_NAME_LENGTH, MIN_TOKEN_SYMBOL_LENGTH,
    MIN_TOKEN_URI_LENGTH, TOKEN_GRADUATION_AMOUNT, TOKEN_TOTAL_SUPPLY,
};

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct CreateLaunchPadTokenArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[derive(Accounts)]
pub struct CreateLaunchPadToken<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [LaunchPadConfig::SEED],
        bump
    )]
    pub launch_pad_config: Account<'info, LaunchPadConfig>,

    #[account(
        init,
        payer = creator,
        space = LaunchPadToken::DISCRIMINATOR.len() + LaunchPadToken::INIT_SPACE,
        seeds = [LaunchPadToken::SEED, mint.key().as_ref()],
        bump
    )]
    pub launch_pad_token: Account<'info, LaunchPadToken>,

    #[account(
        init,
        payer = creator,
        mint::decimals = 9,
        mint::authority = launch_pad_config,
        extensions::metadata_pointer::authority = launch_pad_config,
        extensions::metadata_pointer::metadata_address = mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = launch_pad_config,
        associated_token::token_program = token_program,
    )]
    pub launch_pad_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = launch_pad_config,
        associated_token::token_program = token_program,
    )]
    pub launch_pad_token_reserve_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> CreateLaunchPadToken<'info> {
    pub fn create(
        &mut self,
        args: CreateLaunchPadTokenArgs,
        bumps: CreateLaunchPadTokenBumps,
    ) -> Result<()> {
        require!(
            self.launch_pad_config.status == ProtocolStatus::Active,
            LaunchPadErrorCode::ProtocolConfigNotActive
        );
        require!(
            args.name.len() >= MIN_TOKEN_NAME_LENGTH && args.name.len() <= MAX_TOKEN_NAME_LENGTH,
            LaunchPadErrorCode::InvalidTokenNameLength
        );
        require!(
            args.symbol.len() >= MIN_TOKEN_SYMBOL_LENGTH
                && args.symbol.len() <= MAX_TOKEN_SYMBOL_LENGTH,
            LaunchPadErrorCode::InvalidTokenSymbolLength
        );
        require!(
            args.uri.len() >= MIN_TOKEN_URI_LENGTH && args.uri.len() <= MAX_TOKEN_URI_LENGTH,
            LaunchPadErrorCode::InvalidTokenUriLength
        );
        let launch_pad_config_bump = bumps.launch_pad_config;
        self.init_mint_account(&args)?;
        self.init_token_metadata(&args, launch_pad_config_bump)?;
        self.mint_tokens(launch_pad_config_bump)?;
        self.reserve_tokens_for_graduation(launch_pad_config_bump)?;

        // TODO: decide initial liquidity amounts
        // For now, set both to 0 
        self.launch_pad_token.create(
            self.creator.key(),
            self.mint.key(),
            0, // initial token amount
            0, // initial asset amount
            bumps.launch_pad_token,
        )?;

        Ok(())
    }

    fn init_mint_account(&self, args: &CreateLaunchPadTokenArgs) -> Result<()> {
        let CreateLaunchPadTokenArgs { name, symbol, uri } = args;

        // Define token metadata
        let token_metadata = TokenMetadata {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            ..Default::default()
        };

        // Add 4 extra bytes for size of MetadataExtension (2 bytes for type, 2 bytes for length)
        let len = token_metadata
            .get_packed_len()
            .map_err(|_| ProgramError::InvalidAccountData)?;
        let data_len = 4 + len;

        // Calculate lamports required for the additional metadata
        let lamports =
            data_len as u64 * DEFAULT_LAMPORTS_PER_BYTE_YEAR * DEFAULT_EXEMPTION_THRESHOLD as u64;

        // Transfer additional lamports to mint account
        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.creator.to_account_info(),
                    to: self.mint.to_account_info(),
                },
            ),
            lamports,
        )?;

        Ok(())
    }

    fn init_token_metadata(
        &self,
        args: &CreateLaunchPadTokenArgs,
        launch_pad_config_bump: u8,
    ) -> Result<()> {
        let CreateLaunchPadTokenArgs { name, symbol, uri } = args;
        let signer: &[&[&[u8]]] = &[&[LaunchPadConfig::SEED, &[launch_pad_config_bump]]];
        token_metadata_initialize(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TokenMetadataInitialize {
                    program_id: self.token_program.to_account_info(),
                    mint: self.mint.to_account_info(),
                    metadata: self.mint.to_account_info(),
                    mint_authority: self.launch_pad_config.to_account_info(),
                    update_authority: self.launch_pad_config.to_account_info(),
                },
                signer,
            ),
            name.clone(),
            symbol.clone(),
            uri.clone(),
        )?;
        Ok(())
    }

    fn mint_tokens(&self, launch_pad_config_bump: u8) -> Result<()> {
        let signer: &[&[&[u8]]] = &[&[LaunchPadConfig::SEED, &[launch_pad_config_bump]]];

        token_2022::mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                token_2022::MintTo {
                    mint: self.mint.to_account_info(),
                    to: self.launch_pad_token_account.to_account_info(),
                    authority: self.launch_pad_config.to_account_info(),
                },
                signer,
            ),
            TOKEN_TOTAL_SUPPLY as u64,
        )?;

        // Freeze the mint authority so no more tokens can be minted to make it an NFT
        token_2022::set_authority(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                token_2022::SetAuthority {
                    current_authority: self.launch_pad_config.to_account_info(),
                    account_or_mint: self.mint.to_account_info(),
                },
                signer,
            ),
            AuthorityType::MintTokens,
            None,
        )?;

        Ok(())
    }

    fn reserve_tokens_for_graduation(&self, launch_pad_config_bump: u8) -> Result<()> {
        let signer: &[&[&[u8]]] = &[&[LaunchPadConfig::SEED, &[launch_pad_config_bump]]];

        token_2022::transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                token_2022::TransferChecked {
                    from: self.launch_pad_token_account.to_account_info(),
                    to: self.launch_pad_token_reserve_account.to_account_info(),
                    authority: self.launch_pad_config.to_account_info(),
                    mint: self.mint.to_account_info(),
                },
                signer,
            ),
            TOKEN_GRADUATION_AMOUNT as u64,
            9,
        )?;
        Ok(())
    }
}

pub fn handler(ctx: Context<CreateLaunchPadToken>, args: CreateLaunchPadTokenArgs) -> Result<()> {
    ctx.accounts.create(args, ctx.bumps)?;
    msg!("Launch pad token created");
    Ok(())
}
