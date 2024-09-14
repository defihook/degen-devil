import { createAccount, createMint, mintTo, } from "@solana/spl-token"
import { Connection, Keypair, PublicKey, Signer, } from "@solana/web3.js";
import { getKeypair, getTokenBalance, writePublicKey } from "./utils";

const mintToken = (
    connection: Connection,
    { publicKey, secretKey }: Signer
) => {
    return createMint(
        connection,
        {
            publicKey,
            secretKey,
        },
        publicKey,
        null,
        0,
    );
};

const setupMint = async (
    connection: Connection,
    aliceKeypair: Keypair,
    bobKeypair: Keypair,
    mintAuthority: Signer
): Promise<[PublicKey, PublicKey, PublicKey, PublicKey, PublicKey, PublicKey, PublicKey]> => {

    const mintX = await mintToken(connection, mintAuthority);
    writePublicKey(mintX, `mint_${"X".toLowerCase()}`);
    console.log(`Creating Mint X...${mintX}`);

    const mintY = await mintToken(connection, mintAuthority);
    writePublicKey(mintY, `mint_${"Y".toLowerCase()}`);
    console.log(`Creating Mint Y...${mintY}`);

    console.log(`Creating Alice TokenAccount for X...`);
    const aliceXTokenAccount = await createAccount(connection, aliceKeypair, mintX, aliceKeypair.publicKey);
    writePublicKey(aliceXTokenAccount, `alice_${"X".toLowerCase()}`);

    console.log(`Creating Alice TokenAccount for y...`);
    const aliceYTokenAccount = await createAccount(connection, aliceKeypair, mintY, aliceKeypair.publicKey);
    writePublicKey(aliceYTokenAccount, `alice_${"Y".toLowerCase()}`);

    console.log(`Creating Bob TokenAccount for X...`);
    const bobXTokenAccount = await createAccount(connection, bobKeypair, mintX, bobKeypair.publicKey);
    writePublicKey(bobXTokenAccount, `bob_${"X".toLowerCase()}`);

    console.log(`Creating Bob TokenAccount for Y...`);
    const bobYTokenAccount = await createAccount(connection, bobKeypair, mintY, bobKeypair.publicKey);
    writePublicKey(bobYTokenAccount, `bob_${"Y".toLowerCase()}`);

    console.log(`Creating Winner Mint TokenAccount for Y...`);
    const winnerYTokenAccount = await createAccount(connection, mintAuthority, mintY, mintAuthority.publicKey);
    writePublicKey(winnerYTokenAccount, `winner_mint_holder_${"Y".toLowerCase()}`);

    console.log(`Creating Winner Mint TokenAccount for Y...`);
    const winnerXTokenAccount = await createAccount(connection, mintAuthority, mintX, mintAuthority.publicKey);
    writePublicKey(winnerXTokenAccount, `winner_mint_holder_${"X".toLowerCase()}`);

    await mintTo(connection, mintAuthority, mintX, aliceXTokenAccount, mintAuthority, 100000);
    await mintTo(connection, mintAuthority, mintX, bobXTokenAccount, mintAuthority, 100000);

    await mintTo(connection, mintAuthority, mintY, winnerYTokenAccount, mintAuthority, 100000);
    await mintTo(connection, mintAuthority, mintX, winnerXTokenAccount, mintAuthority, 100000);



    return [mintX, mintY, aliceXTokenAccount, bobXTokenAccount, aliceYTokenAccount, bobYTokenAccount, winnerYTokenAccount];
};


export const setup = async (cluster: string) => {
    const aliceKeypair = getKeypair("alice");
    const bobKeypair = getKeypair("bob");
    const mintAuthority = getKeypair("id");

    const connection = new Connection(cluster, "confirmed");


    const [mintX, mintY, aliceXTokenAccount, bobXTokenAccount, aliceYTokenAccount, bobYTokenAccount, winnerYTokenAccount] = await setupMint(
        connection,
        aliceKeypair,
        bobKeypair,
        mintAuthority,
    );

    console.log("✨Setup complete✨\n");
    console.table([
        {
            "Alice Token Account X": await getTokenBalance(
                aliceXTokenAccount,
                connection
            ),

            "Bob Token Account X": await getTokenBalance(
                bobXTokenAccount,
                connection
            ),
            "Alice Token Account Y": await getTokenBalance(
                aliceYTokenAccount,
                connection
            ),

            "Bob Token Account Y": await getTokenBalance(

                bobYTokenAccount,
                connection
            ),

            "Winning Mint Token Account Y": await getTokenBalance(
                winnerYTokenAccount,
                connection
            ),

        },
    ]);
    console.log("");
};

