use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
    system_instruction,
};
use solana_program::pubkey;

declare_id!("11111111111111111111111111111111");

/// Token-2022. Confidential balances live in this program, not in ours.
pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// A mint account whose only extension is `ConfidentialTransferMint`.
/// Token-2022 rejects any other length at initialize.
const CONFIDENTIAL_MINT_LEN: usize = 235;

pub const CSOL_DECIMALS: u8 = 9;

/// Largest amount Token-2022 will accept in one confidential deposit.
const MAX_SHIELD_LAMPORTS: u64 = (1u64 << 48) - 1;

const IX_CONFIDENTIAL_EXTENSION: u8 = 27;
const IX_CONFIDENTIAL_INITIALIZE_MINT: u8 = 0;
const IX_CONFIDENTIAL_UPDATE_MINT: u8 = 1;
const IX_MINT_TO_CHECKED: u8 = 14;
const IX_BURN_CHECKED: u8 = 15;
const IX_INITIALIZE_MINT2: u8 = 20;

#[program]
pub mod sealed {
    use super::*;

    /// Creates the vault and the cSOL mint. `auditor` is an ElGamal public
    /// key (32 bytes). It can read later transfer amounts and cannot move
    /// funds. The payer becomes the admin who may rotate that key.
    pub fn initialize(ctx: Context<Initialize>, auditor: [u8; 32]) -> Result<()> {
        require!(auditor != [0u8; 32], SealedError::AuditorRequired);

        let payer_key = ctx.accounts.payer.key();
        let mint_key = ctx.accounts.mint.key();
        let vault_key = ctx.accounts.vault.key();
        ctx.accounts.vault.admin = payer_key;
        ctx.accounts.vault.mint = mint_key;
        ctx.accounts.vault.bump = ctx.bumps.vault;

        let lamports = ctx.accounts.rent.minimum_balance(CONFIDENTIAL_MINT_LEN);
        invoke(
            &system_instruction::create_account(
                &payer_key,
                &mint_key,
                lamports,
                CONFIDENTIAL_MINT_LEN as u64,
                &TOKEN_2022_PROGRAM_ID,
            ),
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        invoke(
            &confidential_initialize_mint(&mint_key, &vault_key, &auditor),
            &[ctx.accounts.mint.to_account_info()],
        )?;

        invoke(
            &initialize_mint2(&mint_key, &vault_key),
            &[ctx.accounts.mint.to_account_info()],
        )?;

        msg!("sealed initialize");
        Ok(())
    }

    /// Locks `amount` lamports in the vault and mints that many cSOL into
    /// the user's public token balance. Depositing into the encrypted
    /// balance is a separate Token-2022 instruction the wallet sends after.
    pub fn shield(ctx: Context<MoveSol>, amount: u64) -> Result<()> {
        check_amount(amount)?;
        check_token_account(&ctx.accounts)?;

        invoke(
            &system_instruction::transfer(
                ctx.accounts.user.key,
                &ctx.accounts.vault.key(),
                amount,
            ),
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        let bump = ctx.accounts.vault.bump;
        let bump_seed = [bump];
        let seeds: &[&[u8]] = &[b"vault", &bump_seed];
        invoke_signed(
            &mint_to_checked(
                ctx.accounts.mint.key,
                ctx.accounts.token_account.key,
                &ctx.accounts.vault.key(),
                amount,
            ),
            &[
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.token_account.to_account_info(),
                ctx.accounts.vault.to_account_info(),
            ],
            &[seeds],
        )?;

        msg!("sealed shield {}", amount);
        Ok(())
    }

    /// Burns `amount` cSOL from the user's public balance and returns that
    /// many lamports. Confidential tokens have to be withdrawn to the public
    /// balance first, which is the step that prints the amount.
    pub fn unshield(ctx: Context<MoveSol>, amount: u64) -> Result<()> {
        check_amount(amount)?;
        check_token_account(&ctx.accounts)?;

        let public_balance = read_u64(&ctx.accounts.token_account, 64)?;
        require!(
            public_balance >= amount,
            SealedError::InsufficientClaim
        );

        invoke(
            &burn_checked(
                ctx.accounts.token_account.key,
                ctx.accounts.mint.key,
                ctx.accounts.user.key,
                amount,
            ),
            &[
                ctx.accounts.token_account.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.user.to_account_info(),
            ],
        )?;

        move_lamports(
            &ctx.accounts.vault.to_account_info(),
            &ctx.accounts.user.to_account_info(),
            amount,
        )?;

        msg!("sealed unshield {}", amount);
        Ok(())
    }

    /// Points the mint's auditor key at a new ElGamal public key.
    /// Already-recorded transfers stay encrypted to the previous key.
    pub fn update_auditor(ctx: Context<UpdateAuditor>, auditor: [u8; 32]) -> Result<()> {
        require!(auditor != [0u8; 32], SealedError::AuditorRequired);

        let bump = ctx.accounts.vault.bump;
        let bump_seed = [bump];
        let seeds: &[&[u8]] = &[b"vault", &bump_seed];
        invoke_signed(
            &confidential_update_mint(ctx.accounts.mint.key, &ctx.accounts.vault.key(), &auditor),
            &[
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.vault.to_account_info(),
            ],
            &[seeds],
        )?;

        msg!("sealed update auditor");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1,
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,
    /// CHECK: created in this instruction as a Token-2022 mint. Must be an
    /// unused keypair and a signer so the system program can create it.
    #[account(mut)]
    pub mint: Signer<'info>,
    /// CHECK: address is constrained to Token-2022.
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MoveSol<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault"],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
    /// CHECK: must be the mint recorded on the vault, and owned by Token-2022.
    #[account(mut, address = vault.mint)]
    pub mint: AccountInfo<'info>,
    /// CHECK: user's Token-2022 account for that mint. Checked in the handler.
    #[account(mut)]
    pub token_account: AccountInfo<'info>,
    /// CHECK: address is constrained to Token-2022.
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateAuditor<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault"],
        bump = vault.bump,
        has_one = admin,
    )]
    pub vault: Account<'info, Vault>,
    /// CHECK: the cSOL mint. The vault PDA signs the Token-2022 update.
    #[account(mut, address = vault.mint)]
    pub mint: AccountInfo<'info>,
    /// CHECK: address is constrained to Token-2022.
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: AccountInfo<'info>,
}

#[account]
pub struct Vault {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
}

#[error_code]
pub enum SealedError {
    #[msg("auditor public key is required")]
    AuditorRequired,
    #[msg("amount must be greater than zero")]
    AmountZero,
    #[msg("amount is above the confidential deposit limit")]
    AmountTooLarge,
    #[msg("mint is not the sealed cSOL mint")]
    BadMint,
    #[msg("token account is not a cSOL account owned by the user")]
    BadTokenAccount,
    #[msg("wallet does not have that much SOL")]
    InsufficientSol,
    #[msg("vault cannot cover that withdrawal")]
    VaultShortfall,
    #[msg("public cSOL balance is too small")]
    InsufficientClaim,
}

fn check_amount(amount: u64) -> Result<()> {
    require!(amount > 0, SealedError::AmountZero);
    require!(amount <= MAX_SHIELD_LAMPORTS, SealedError::AmountTooLarge);
    Ok(())
}

fn check_token_account(accounts: &MoveSol<'_>) -> Result<()> {
    let token = &accounts.token_account;
    require_keys_eq!(
        *token.owner,
        TOKEN_2022_PROGRAM_ID,
        SealedError::BadTokenAccount
    );
    require_keys_eq!(
        *accounts.mint.owner,
        TOKEN_2022_PROGRAM_ID,
        SealedError::BadMint
    );

    let mint_on_account = read_pubkey(token, 0)?;
    require_keys_eq!(mint_on_account, accounts.vault.mint, SealedError::BadTokenAccount);

    let owner_on_account = read_pubkey(token, 32)?;
    require_keys_eq!(owner_on_account, accounts.user.key(), SealedError::BadTokenAccount);

    let decimals = accounts
        .mint
        .try_borrow_data()?
        .get(44)
        .copied()
        .ok_or(SealedError::BadMint)?;
    require!(decimals == CSOL_DECIMALS, SealedError::BadMint);
    Ok(())
}

fn move_lamports(from: &AccountInfo<'_>, to: &AccountInfo<'_>, amount: u64) -> Result<()> {
    let from_balance = from.lamports();
    require!(from_balance >= amount, SealedError::InsufficientSol);

    if *from.owner == crate::ID {
        let keep = Rent::get()?.minimum_balance(from.data_len());
        require!(
            from_balance - amount >= keep,
            SealedError::VaultShortfall
        );
    }

    **from.try_borrow_mut_lamports()? = from_balance
        .checked_sub(amount)
        .ok_or(SealedError::InsufficientSol)?;
    **to.try_borrow_mut_lamports()? = to
        .lamports()
        .checked_add(amount)
        .ok_or(SealedError::AmountTooLarge)?;
    Ok(())
}

fn read_pubkey(account: &AccountInfo<'_>, offset: usize) -> Result<Pubkey> {
    let data = account.try_borrow_data()?;
    let end = offset.checked_add(32).ok_or(SealedError::BadTokenAccount)?;
    let bytes = data.get(offset..end).ok_or(SealedError::BadTokenAccount)?;
    let mut raw = [0u8; 32];
    raw.copy_from_slice(bytes);
    Ok(Pubkey::new_from_array(raw))
}

fn read_u64(account: &AccountInfo<'_>, offset: usize) -> Result<u64> {
    let data = account.try_borrow_data()?;
    let end = offset.checked_add(8).ok_or(SealedError::BadTokenAccount)?;
    let bytes = data.get(offset..end).ok_or(SealedError::BadTokenAccount)?;
    let mut raw = [0u8; 8];
    raw.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(raw))
}

/// `ConfidentialTransferInstruction::InitializeMint`. No signer. The mint
/// authority is the vault, and new confidential accounts are auto-approved.
fn confidential_initialize_mint(
    mint: &Pubkey,
    authority: &Pubkey,
    auditor: &[u8; 32],
) -> Instruction {
    let mut data = Vec::with_capacity(67);
    data.push(IX_CONFIDENTIAL_EXTENSION);
    data.push(IX_CONFIDENTIAL_INITIALIZE_MINT);
    data.extend_from_slice(authority.as_ref());
    data.push(1);
    data.extend_from_slice(auditor);
    Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts: vec![AccountMeta::new(*mint, false)],
        data,
    }
}

fn confidential_update_mint(mint: &Pubkey, authority: &Pubkey, auditor: &[u8; 32]) -> Instruction {
    let mut data = Vec::with_capacity(35);
    data.push(IX_CONFIDENTIAL_EXTENSION);
    data.push(IX_CONFIDENTIAL_UPDATE_MINT);
    data.push(1);
    data.extend_from_slice(auditor);
    Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

fn initialize_mint2(mint: &Pubkey, mint_authority: &Pubkey) -> Instruction {
    let mut data = Vec::with_capacity(38);
    data.push(IX_INITIALIZE_MINT2);
    data.push(CSOL_DECIMALS);
    data.extend_from_slice(mint_authority.as_ref());
    data.extend_from_slice(&[0, 0, 0, 0]);
    Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts: vec![AccountMeta::new(*mint, false)],
        data,
    }
}

fn mint_to_checked(
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(10);
    data.push(IX_MINT_TO_CHECKED);
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(CSOL_DECIMALS);
    Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data,
    }
}

fn burn_checked(account: &Pubkey, mint: &Pubkey, owner: &Pubkey, amount: u64) -> Instruction {
    let mut data = Vec::with_capacity(10);
    data.push(IX_BURN_CHECKED);
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(CSOL_DECIMALS);
    Instruction {
        program_id: TOKEN_2022_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*account, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(*owner, true),
        ],
        data,
    }
}
