#![allow(unused_imports)]

use anchor_lang::{accounts::interface_account, prelude::*, system_program::Transfer};

use crate::{make, Escrow};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

#[derive(Accounts)]
// #[instruction(seed: u64)]
pub struct Take<'info> {
    //  TODO: Implement Take Accounts
    //The maker of the escrow
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    //This will be the Taker account
    #[account(mut)]
    pub taker: Signer<'info>,

    //mint token addresses
    #[account(
        mint::token_program=token_program
    )]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mint::token_program=token_program
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,

    //ATA of the users
    #[account(
        mut,
        associated_token::mint=mint_b,
        associated_token::authority=taker,
        associated_token::token_program=token_program,
    )]
    pub taker_ata_b: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer=maker,
        associated_token::mint=mint_b,
        associated_token::authority=maker,
        associated_token::token_program=token_program,
    )]
    pub maker_ata_b: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer=taker,
        associated_token::mint=mint_a,
        associated_token::authority=taker,
        associated_token::token_program=token_program,
    )]
    pub taker_ata_a: InterfaceAccount<'info, TokenAccount>,

    //The escrow PDA
    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    escrow: Account<'info, Escrow>,

    //The vault account owned by Token Program
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    //  TODO: Implement Take Instruction
    //  Includes Deposit, Withdraw and Close Vault
    //Send the tokens from the taker to the user.
    pub fn deposit(&mut self) -> Result<()> {
        let amount = self.escrow.receive;
        let transfer_accounts = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };
        let cpi_context = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);
        transfer_checked(cpi_context, amount, self.mint_b.decimals)?;
        msg!("deposit complete!");
        Ok(())
    }

    pub fn withdraw(&mut self) -> Result<()> {
        //Transfer from the vault to the taker.
        let amount_to_transfer = self.vault.amount;
        let transfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            mint: self.mint_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };
        let taker_seeds: &[&[&[u8]]] = &[&[
            b"escrow",
            self.maker.to_account_info().key.as_ref(),
            &self.escrow.seed.to_le_bytes(),
            &[self.escrow.bump],
        ]];
        let cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            transfer_accounts,
            taker_seeds,
        );
        transfer_checked(cpi_context, amount_to_transfer, self.mint_a.decimals)?;

        let accounts: CloseAccount<'_> = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.taker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            taker_seeds,
        );
        msg!("withdraw complete!");

        close_account(ctx)?;
        msg!("close account complete!");
        Ok(())
    }
    //Close the vaut after the withdraw is done.
    // pub fn vault(&mut self) {}
}
