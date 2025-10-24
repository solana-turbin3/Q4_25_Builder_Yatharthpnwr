use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{errors::AmmError, instructions::deposit, state::Config};

#[derive(Accounts)]

///Q -> what is the difference between constratints and account types?
/// Why are we defining TOkenAccount type inside of Account? Why can't it be a different type like the
/// SystemProgram or the simple InterfaceAccount? Why is it Account<'info, TokenAccount> and not TokenAccount<'info>

// The user
//For deposit we will require, the mint address of x and y ,
// the ata of user (both x and y )
// THe config
// The vault address
//The lp Mint address
pub struct Swap<'info> {
    // TODO: Write the accounts struct
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    //LP token address.
    #[account(
        mut,
        seeds=[b"lp", config.key().as_ref()],
        bump=config.lp_bump
    )]
    pub mint_lp: Account<'info, Mint>,
    

    //This has to be init_if_needed
    #[account(
        mut,
        associated_token::mint = mint_x, 
        associated_token::authority = user
    )]
    pub user_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y, 
        associated_token::authority = user
    )]
    pub user_y: Account<'info, TokenAccount>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump= config.config_bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_x : Account<'info, TokenAccount>,


    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config
    )]
    pub vault_y : Account<'info, TokenAccount>,


    pub user_lp: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount: u64, min: u64) -> Result<()> {
        // TODO
        //The user will either have x or y tokens.
        //They will deposit those tokens into the respective vaults.
        //We need to calculate the amount of other token they must get after depositing the initial token.
        // Then we will transfer that amount to the user after checking the slippage.

        //LOGICAL STEPS::
        // 1) Check if the AMM is locked or not.
        require!(self.config.locked != false, AmmError::PoolLocked);
        // 2) Check if the amount being sent to the vault is 0.
        require!(amount != 0, AmmError::InvalidAmount);

        // 3) check the amount of swap out token the user gets after depositing the swap in token.

        let amount_out_tokens = match is_x {
            true => 
                ConstantProduct::delta_y_from_x_swap_amount(self.vault_x.amount, self.vault_y.amount, amount)
            ,
            false => 
                ConstantProduct::delta_x_from_y_swap_amount(self.vault_x.amount, self.vault_y.amount, amount)

        }.unwrap();

        // 4) check for slippage.
        require!(amount_out_tokens >= min, AmmError::SlippageExceeded);

        // 5) First deposit the input token.
        // 6) trasfer the amount_out_tokens number of output tokens to the user from the vault.
        if is_x {
            self.deposit_tokens(true, amount)?;
            self.withdraw_tokens(false, amount_out_tokens)
        }
        else{
            self.deposit_tokens(false, amount)?;
            self.withdraw_tokens(true, amount_out_tokens)
        }

    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        // TODO
        //The user will deposit token x or y.
        let (from, to) = match is_x {
            false => (
                self.user_y.to_account_info(), 
                self.vault_y.to_account_info()
            ),
            true => (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info()
            )
        };

        //Accounts involved in the transfer
        let acc = Transfer{
            from,
            to,
            authority: self.user.to_account_info()
        };

        //Create CPI_CONTEXT
        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), acc);

        //Transfer
        transfer(cpi_ctx, amount)
    }

    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        // TODO 

        //Transfer the output tokens from the vault to the user.
        let (from, to) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info()
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info()
            )
        };
        //accounts involved
        let accounts = Transfer{
            from, 
            to, 
            authority: self.user.to_account_info()
        };
        //CPI_CONTEXT
        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), accounts);

        transfer(cpi_ctx, amount)
    }
}
