#![cfg(feature = "test-bpf")]

mod utils;

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_sdk::{instruction::AccountMeta, transaction::Transaction};
use {
    solana_program_test::*,
    solana_sdk::{instruction::Instruction, signature::Signer},
    utils::*,
};

#[tokio::test]
async fn flip() -> Result<(), Error> {
    let TestContext {
        mut ctx,
        alice,
        oracle,
        winner_mint_holder,
        ..
    } = get_program_test_context().await?;

    let token_x = create_token(2, &mut ctx).await?;
    let token_y = create_token(0, &mut ctx).await?;

    let alice_token_x_account = create_token_account(&token_x.pubkey(), &alice, &mut ctx).await?;

    let winner_token_y_account =
        create_token_account(&token_y.pubkey(), &winner_mint_holder, &mut ctx).await?;

    mint_token(
        &token_x.pubkey(),
        &alice_token_x_account.pubkey(),
        1000000,
        &mut ctx,
    )
    .await?;

    mint_token(
        &token_y.pubkey(),
        &winner_token_y_account.pubkey(),
        52500000,
        &mut ctx,
    )
    .await?;

    let amount = 5250;

    let (coin_pda, coin_bump) = degendevil::coin_pda(&alice.pubkey());
    let (winner_pda, _winner_bum) = degendevil::winner_pda(&alice.pubkey());
    let (vault_pda, vault_bump) = degendevil::vault_pda(&token_x.pubkey(), &alice.pubkey());

    let (requester, req_bump) = degenrand::requestor_pda(&alice.pubkey());
    let (oracle_vault, oracle_bump) = degenrand::vault_pda(&alice.pubkey());

    let degenrand_init_accounts = degenrand::accounts::Initialize {
        authority: alice.pubkey(),
        oracle: oracle.pubkey(),
        requester,
        vault: oracle_vault,
        rent: anchor_lang::solana_program::sysvar::rent::id(),
        system_program: anchor_lang::solana_program::system_program::id(),
    }
    .to_account_metas(None);

    let degenrand_init_data = degenrand::instruction::Initialize {
        request_bump: req_bump,
        vault_bump: oracle_bump,
    }
    .data();

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            accounts: degenrand_init_accounts,
            data: degenrand_init_data,
            program_id: degenrand::id(),
        }],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &alice],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(transaction).await?;

    let mut create_coin_accounts = degendevil::accounts::CreateCoin {
        coin: coin_pda,
        vault: vault_pda,
        initiator: alice.pubkey(),
        requester,
        // acceptor: bob.pubkey(),
        initiator_ata: alice_token_x_account.pubkey(),
        mint: token_x.pubkey(),
        oracle: oracle.pubkey(),
        oracle_vault,
        degenrand_program: degenrand::id(),
        rent: anchor_lang::solana_program::sysvar::rent::id(),
        token_program: spl_token::id(),
        system_program: anchor_lang::solana_program::system_program::id(),
    }
    .to_account_metas(None);

    create_coin_accounts.push(AccountMeta::new(coin_pda, false));

    let create_coin_data = degendevil::instruction::CreateCoin {
        // _req_bump: req_bump,
        amount,
        coin_bump,
        vault_bump,
    }
    .data();

    let ix = Instruction {
        program_id: degendevil::id(),
        accounts: create_coin_accounts,
        data: create_coin_data,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &alice],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(transaction).await?;

    let mut publish_random_accounts = degenrand::accounts::PublishRandom {
        oracle: oracle.pubkey(),

        system_program: anchor_lang::solana_program::system_program::id(),
    }
    .to_account_metas(None);

    publish_random_accounts.push(AccountMeta::new(requester, false));

    let publish_random_data = degenrand::instruction::PublishRandom {
        pkt_id: [0u8; 32],
        random: [0u8; 64],
        tls_id: [0u8; 32],
    }
    .data();

    let ix = Instruction {
        program_id: degenrand::id(),
        accounts: publish_random_accounts,
        data: publish_random_data,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &oracle],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(transaction).await?;

    let reveal_coin_data = degendevil::instruction::RevealCoin {}.data();

    let mut reveal_coin_accounts = degendevil::accounts::RevealCoin {
        authority: alice.pubkey(),
        initiator: alice.pubkey(),
        winner: winner_pda,
        initiator_ata: alice_token_x_account.pubkey(),
        admin_ata: degendevil::admin_account_pubkey()?,
        mint: token_x.pubkey(),
        vault: vault_pda,
        requester,
        degenrand_program: degenrand::id(),
        token_program: spl_token::id(),
        system_program: anchor_lang::solana_program::system_program::id(),
    }
    .to_account_metas(None);

    reveal_coin_accounts.push(AccountMeta::new(coin_pda, false));

    let ix = Instruction {
        program_id: degendevil::id(),
        accounts: reveal_coin_accounts,
        data: reveal_coin_data,
    };
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &alice],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(transaction).await?;

    Ok(())
}
