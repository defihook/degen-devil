import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import * as fs from "fs";
import * as anchor from "@project-serum/anchor";

export const writePublicKey = (publicKey: PublicKey, name: string) => {
    fs.writeFileSync(
        `./keys/${name}_pub.json`,
        JSON.stringify(publicKey.toString())
    );
};

export const getPublicKey = (name: string) =>
    new PublicKey(
        JSON.parse(fs.readFileSync(`./keys/${name}_pub.json`) as unknown as string)
    );

export const getPrivateKey = (name: string) =>
    Uint8Array.from(
        JSON.parse(fs.readFileSync(`./keys/${name}.json`) as unknown as string)
    );

export const getKeypair = (name: string) =>
    new Keypair({
        publicKey: getPublicKey(name).toBytes(),
        secretKey: getPrivateKey(name),
    });

export const getTokenBalance = async (
    pubkey: PublicKey,
    connection: Connection
) => {
    return parseInt(
        (await connection.getTokenAccountBalance(pubkey)).value.amount
    );
};


export const coinPrefix = "DEGENDEVIL_COIN_SEED_V1.0";
export const winnerPrefix = "DEGENDEVIL_WINNER_SEED_V1.0";
export const vaultPrefix = "DEGENDEVIL_VAULT_SEED_V1.0";
export const randVaultPrefix = "DEGENRAND_VAULT_SEED_V1.0";
export const requesterPrefix = "DEGENRAND_REQUESTOR_SEED_V1.0";


export const requestorPda = async (publicKey, programId) => {
    publicKey = new anchor.web3.PublicKey(publicKey);
    programId = new anchor.web3.PublicKey(programId);
    let [requestorPdaAddress, reqBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(requesterPrefix), publicKey.toBuffer()],
        programId
    );
    return { requestorPdaAddress, reqBump };
};

export const oracleVaultPda = async (publicKey, programId) => {
    publicKey = new anchor.web3.PublicKey(publicKey);
    programId = new anchor.web3.PublicKey(programId);
    let [oracleVaultPdaAddress, reqVaultBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(randVaultPrefix), publicKey.toBuffer(), programId.toBuffer()],
        programId,
    );
    return { oracleVaultPdaAddress, reqVaultBump };

};

export const coinPda = async (publicKey, programId) => {
    publicKey = new anchor.web3.PublicKey(publicKey);
    programId = new anchor.web3.PublicKey(programId);
    let [coinPdaAddress, coinBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(coinPrefix), publicKey.toBuffer()],
        programId
    );
    return { coinPdaAddress, coinBump };
};

export const winnerPda = async (publicKey, programId) => {
    publicKey = new anchor.web3.PublicKey(publicKey);
    programId = new anchor.web3.PublicKey(programId);
    let [winnerPdaAddress, winnerBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(winnerPrefix), publicKey.toBuffer()],
        programId
    );
    return { winnerPdaAddress, winnerBump };
};
export const vaultPda = async (mintX, publicKey, programId) => {
    mintX = new anchor.web3.PublicKey(mintX);
    publicKey = new anchor.web3.PublicKey(publicKey);
    programId = new anchor.web3.PublicKey(programId);
    let [vaultPdaAddress, vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(vaultPrefix), mintX.toBuffer(), publicKey.toBuffer(), programId.toBuffer()],
        programId
    );
    return { vaultPdaAddress, vaultBump };
};