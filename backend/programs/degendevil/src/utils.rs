use std::str::FromStr;

use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::{DegenErrorCode, COIN_PREFIX, VAULT_PREFIX, WINNER_PREFIX};

/// Signer Seeds for Vault
///  let signer_seeds = &[
///     VAULT_PREFIX.as_bytes(),
///     mint.key.as_ref(),
///     program_id.as_ref(),
///     &[vault_bump],
/// ];
pub fn vault_pda(mint: &Pubkey, initiator: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            VAULT_PREFIX.as_bytes(),
            mint.as_ref(),
            initiator.as_ref(),
            &crate::id().as_ref(),
        ],
        &crate::id(),
    )
}

/// Handles the decimal value.
/// Converts to appropriate u64 representation.
pub fn calculate_amount(mint: &Mint, amount: u64) -> u64 {
    amount.saturating_mul(10_u64.pow((mint.decimals) as u32))
}

/// Signer Seeds for Coin
///  let signer_seeds =  &[
///    COIN_PREFIX.as_bytes(),
///    initiator.key.as_ref(),
///    &[coin_bump],
/// ];
pub fn coin_pda(initiator: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[COIN_PREFIX.as_bytes(), initiator.as_ref()], &crate::id())
}

/// Signer Seeds for Winner
///  let signer_seeds =  &[
///    WINNER_PREFIX.as_bytes(),
///    initiator.key.as_ref(),
///    &[winner_bump],
/// ];
pub fn winner_pda(initiator: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[WINNER_PREFIX.as_bytes(), initiator.as_ref()],
        &crate::id(),
    )
}
// Bet token 52.5 A 75% chance win Token B
// Bet token 35 A 50% chance win Token B
// Bet 17.5 token A 25% chance win Token B
// Bet 7 token A 10% chance win Token B
pub fn calculate_probability(amount: u64, random: &[u8]) -> u8 {
    let r50 = rand50(random[0]);
    let r75 = rand50(random[1]) | rand50(random[2]);
    let r90 = rand50(random[3]) |
     r75;

    match amount {
        amount if amount >= 5250 => r75,
        amount if amount >= 3500 && amount < 5250 => r50,
        amount if amount >= 1750 && amount < 3500 => 1 - r75,
        amount if amount >= 700 && amount < 1750 => 1 - r90,
        _ => random[4],
    }
}
pub fn rand50(rand: u8) -> u8 {
    &rand & 1
}

const ADMIN_TOKEN_A_PUBKEY: &str = "9kjgGV2PjKgpu4r7wFqWRhV6jq1SjJGgQjZYh6bG5Asa";

pub fn admin_account_pubkey() -> Result<Pubkey> {
    Pubkey::from_str(ADMIN_TOKEN_A_PUBKEY).map_err(|_| DegenErrorCode::InvalidAdminPubkey.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    fn random_bytes() -> Vec<u8> {
        let mut rng = rand::thread_rng();

        (0..64).map(|_| rng.gen_range(0..=1)).collect()
    }

    #[test]
    fn test_probabilities() {
        let rand = random_bytes();

        let v = calculate_probability(5250, rand.as_slice());
        println!("{}", v);

        let v = calculate_probability(3500, rand.as_slice());
        println!("{}", v);

        let v = calculate_probability(1750, rand.as_slice());
        println!("{}", v);

        let v = calculate_probability(1000, rand.as_slice());
        println!("{}", v);

        let v = calculate_probability(700, rand.as_slice());
        println!("{}", v);
    }

    #[test]
    fn test_probability_5250_amount() {
        let v = (0..=100).fold(0, |mut acc, _| {
            let rand = random_bytes();
            acc += calculate_probability(5250, rand.as_slice());
            acc
        });

        println!("Amount : 5250, `{}` Wins per 100", v);
    }

    #[test]
    fn test_probability_3500_amount() {
        let v = (0..=100).fold(0, |mut acc, _| {
            let rand = random_bytes();
            acc += calculate_probability(3500, rand.as_slice());
            acc
        });

        println!("Amount : 3500, `{}` Wins per 100", v);
    }

    #[test]
    fn test_probability_1750_amount() {
        let v = (0..=100).fold(0, |mut acc: u8, _| {
            let rand = random_bytes();
            acc += calculate_probability(1750, rand.as_slice());
            acc
        });

        println!("Amount : 1750, `{}` Wins per 100", v);
    }

    #[test]
    fn test_probability_700_amount() {
        let v = (0..=100).fold(0, |mut acc: u8, _| {
            let rand = random_bytes();
            acc += calculate_probability(700, rand.as_slice());
            acc
        });

        println!("Amount : 700, `{}` Wins per 100", v);
    }
}
