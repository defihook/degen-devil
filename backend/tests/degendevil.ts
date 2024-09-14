import * as anchor from '@project-serum/anchor';
import { Program, } from '@project-serum/anchor';
import { Degendevil } from '../target/types/degendevil';
import { randomBytes } from 'crypto';
import { MockOracleSession as OracleSession } from "./sessions.js";
import { coinPda, getKeypair, getPublicKey, oracleVaultPda, requestorPda, vaultPda, winnerPda } from './utils';
import { setup } from './setup';
import { rpc } from '@project-serum/anchor/dist/cjs/utils';
import { Token, TOKEN_PROGRAM_ID } from '@solana/spl-token';


describe('degendevil', () => {

    // const ENV = 'http://localhost:8899';
    const ENV = "https://api.devnet.solana.com";
    const degenrandId = new anchor.web3.PublicKey(anchor.workspace.Degenrand.programId);


    function createProvider(keyPair) {
        let solConnection = new anchor.web3.Connection(ENV);
        let walletWrapper = new anchor.Wallet(keyPair);
        return new anchor.Provider(solConnection, walletWrapper, {
            preflightCommitment: 'recent',
        });
    }

    async function getBalance(prov, key) {
        anchor.setProvider(prov);
        return await prov.connection.getBalance(key, "confirmed");
    }

    const userKeyPair = getKeypair("alice");
    let provider1 = createProvider(userKeyPair);

    const mintAuth = getKeypair("id");

    const oracle = getKeypair("oracle");
    const oracleSession = new OracleSession(oracle, anchor.workspace.Degenrand.idl, degenrandId, ENV);



    const program = anchor.workspace.Degendevil as Program<Degendevil>;
    const user1Program = new anchor.Program(program.idl, program.programId, provider1);



    const oraclePubkey = oracle.publicKey;
    const degenrandProgram = new anchor.Program(anchor.workspace.Degenrand.idl, degenrandId, provider1);

    const amount = new anchor.BN(5250);


    let mintX;
    let mintY;
    let initiatorAta;
    let adminAta;
    let adminYAta;

    let requestorPdaAddress, reqBump;
    let oracleVaultPdaAddress, reqVaultBump;
    let coinPdaAddress, coinBump;
    let vaultPdaAddress, vaultBump;
    let winnerPdaAddress, winnerBump;




    anchor.setProvider(provider1);

    it('Set up tests', async () => {

        //     // await setup(ENV);

        mintX = getPublicKey("mint_x");
        mintY = getPublicKey("mint_y");
        initiatorAta = getPublicKey("alice_x");
        adminAta = getPublicKey("admin_mint_x");
        adminYAta = getPublicKey("admin_mint_y");

        let rp = await requestorPda(userKeyPair.publicKey, degenrandProgram.programId);
        requestorPdaAddress = rp.requestorPdaAddress; reqBump = rp.reqBump;

        let op = await oracleVaultPda(userKeyPair.publicKey, degenrandProgram.programId);
        oracleVaultPdaAddress = op.oracleVaultPdaAddress; reqVaultBump = op.reqVaultBump;

        let cp = await coinPda(userKeyPair.publicKey, program.programId);
        coinPdaAddress = cp.coinPdaAddress, coinBump = cp.coinBump;

        let vp = await vaultPda(mintX, userKeyPair.publicKey, program.programId);

        vaultPdaAddress = vp.vaultPdaAddress; vaultBump = vp.vaultBump;

        let wp = await winnerPda(userKeyPair.publicKey, program.programId);
        winnerPdaAddress = wp.winnerPdaAddress; winnerBump = wp.winnerBump;

        console.log('Coin account: ', coinPdaAddress.toString());
        console.log('Req account: ', requestorPdaAddress.toString());
        console.log('Vault account: ', vaultPdaAddress.toString());
        console.log('Req Vault account: ', oracleVaultPdaAddress.toString());
        console.log('Winner account: ', winnerPdaAddress.toString());

        anchor.setProvider(provider1);
        await degenrandProgram.rpc.initialize(
            reqBump,
            reqVaultBump,
            {
                accounts: {
                    requester: requestorPdaAddress,
                    vault: oracleVaultPdaAddress,
                    authority: userKeyPair.publicKey,
                    oracle: oraclePubkey,
                    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                    systemProgram: anchor.web3.SystemProgram.programId,
                },
                signers: [userKeyPair],
            },
        );
    });

    it('Create a coin!', async () => {
        anchor.setProvider(provider1);
        await user1Program.rpc.createCoin(
            coinBump,
            vaultBump,
            amount,
            {
                accounts: {
                    coin: coinPdaAddress,
                    vault: vaultPdaAddress,
                    requester: requestorPdaAddress,
                    initiator: userKeyPair.publicKey,
                    initiatorAta,
                    mint: mintX,
                    oracle: oraclePubkey,
                    oracleVault: oracleVaultPdaAddress,
                    degenrandProgram: degenrandId,
                    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: anchor.web3.SystemProgram.programId,
                },
                signers: [userKeyPair],
            }
        );
    });


    it('Oracle responds to request', async () => {
        let randomNumber = randomBytes(64);
        randomNumber[0] = 0; // Force winner to be acceptor
        randomNumber[1] = 0; // Force winner to be acceptor
        randomNumber[2] = 0; // Force winner to be acceptor
        randomNumber[3] = 0; // Force winner to be acceptor
        randomNumber[4] = 0; // Force winner to be acceptor
        randomNumber[5] = 0; // Force winner to be acceptor

        let requester = { publicKey: requestorPdaAddress };

        await oracleSession.publishRandom(requester, randomNumber);
    });

    it('Reveal the result', async () => {
        anchor.setProvider(provider1);
        await user1Program.rpc.revealCoin(
            {
                accounts: {
                    initiator: userKeyPair.publicKey,
                    initiatorAta,
                    vault: vaultPdaAddress,
                    requester: requestorPdaAddress,
                    authority: userKeyPair.publicKey,
                    mint: mintX,
                    adminAta,
                    winner: winnerPdaAddress,
                    degenrandProgram: degenrandId,
                    tokenProgram: TOKEN_PROGRAM_ID,
                    systemProgram: anchor.web3.SystemProgram.programId,
                },
                remainingAccounts: [
                    {
                        pubkey: coinPdaAddress,
                        isWritable: true,
                        isSigner: false,
                    },
                    {
                        pubkey: oracleVaultPdaAddress,
                        isWritable: true,
                        isSigner: false,
                    },
                ],
                signers: [
                    userKeyPair,
                ],
            },
        );
    });


    it('remove pda from degenrand', async () => {
        anchor.setProvider(provider1);
        await degenrandProgram.rpc.removePdas(
            {
                accounts: {
                    requester: requestorPdaAddress,
                    vault: oracleVaultPdaAddress,
                    authority: userKeyPair.publicKey,
                    initiator: userKeyPair.publicKey,
                    systemProgram: anchor.web3.SystemProgram.programId,
                },
                signers: [userKeyPair],
            },
        );
    });

    it("Send Token B to winner.", async () => {

        let mint = new Token(provider1.connection, mintY, TOKEN_PROGRAM_ID, mintAuth);

        let winner = await user1Program.account.winner.fetch(winnerPdaAddress);
        console.log("Initiator Won", winner.status);

        if (winner.status) {
            let winnerYAta = await mint.getOrCreateAssociatedAccountInfo(userKeyPair.publicKey);
            await mint.transfer(adminYAta, winnerYAta.address, mintAuth, [], 1);
        }

        anchor.setProvider(provider1);
        await user1Program.rpc.removePdas(
            {
                accounts: {
                    initiator: userKeyPair.publicKey,
                    winner: winnerPdaAddress,
                    systemProgram: anchor.web3.SystemProgram.programId,
                },
                signers: [userKeyPair],
            },
        );
    })
});
