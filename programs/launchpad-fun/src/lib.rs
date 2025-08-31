use anchor_lang::prelude::*;

declare_id!("BqECdxVHEDqGudnvwVRexKFXEWg3hoX5whLecTXZs6jn");

#[program]
pub mod launchpad_fun {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
