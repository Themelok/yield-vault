import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { YieldVault } from "../target/types/yield_vault";
import { PublicKey, Connection } from "@solana/web3.js";
import { 
  TOKEN_PROGRAM_ID, 
  getOrCreateAssociatedTokenAccount, 
  createMint, 
  mintTo, 
  setAuthority, 
  AuthorityType,
  getAssociatedTokenAddressSync,
  getAccount,
  createAssociatedTokenAccountIdempotent
} from "@solana/spl-token";
import { assert } from "chai";

let USDC_MINT: PublicKey;
let USER_USDC_ATA: PublicKey;
let VAULT_USDC_ATA: PublicKey;

const USDC_AMOUNT = 1000;


const VAULT_SEED = Buffer.from("vault");
const USDC_VAULT_TOKEN_ACCOUNT_SEED = Buffer.from("usdc_vault");
// const USDC_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const mint_authority = new PublicKey("BJE5MMbqXjVwjAF7oxwPYXnTXDyspzZyt4vwenNw5ruG");

describe("yield-vault", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.yieldVault as Program<YieldVault>;
  const user = anchor.web3.Keypair.generate();
  
  const [vault_account_pda, vault_seed] = PublicKey.findProgramAddressSync([VAULT_SEED, user.publicKey.toBuffer()], program.programId);
  const connection = program.provider.connection;

  before(async () => {
    // Verify USDC mint account exists (should be cloned by test validator)
    // const mintInfo = await connection.getAccountInfo(USDC_MINT);
    // if (!mintInfo) {
    //   throw new Error("USDC mint account not found. Make sure to start test validator with --clone EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    // }
    // console.log("✅ USDC mint account found:", USDC_MINT.toBase58());
    await foundAccount(program, user.publicKey);

    // create mock USDC mint
    USDC_MINT = await createMint(
      connection,
      program.provider.wallet.payer,
      program.provider.wallet.publicKey,
      program.provider.wallet.publicKey,
      6
    )
    console.log("✅ Mock USDC mint created:", USDC_MINT.toBase58());

    // create user's ATA for mock USDC mint
    let ownerAta = await getOrCreateAssociatedTokenAccount(
      connection,
      program.provider.wallet.payer,
      USDC_MINT,
      user.publicKey
    );
    USER_USDC_ATA = ownerAta.address;
    console.log("✅ User USDC ATA:", USER_USDC_ATA.toBase58());

    // mint some mock USDC to user's ATA
    await mintTo(
      connection,
      program.provider.wallet.payer,
      USDC_MINT,
      USER_USDC_ATA,
      program.provider.wallet.publicKey,
      1_000_000_000
    )
    console.log("✅ Minted 1,000 mock USDC to user ATA");

    const ata = getAssociatedTokenAddressSync(USDC_MINT, vault_account_pda, true);
    VAULT_USDC_ATA = ata;
    console.log("✅ Vault USDC ATA:", VAULT_USDC_ATA.toBase58());
  });

  it("Is initialized!", async () => {
    const tx = await program.methods.initialize().accounts({
      owner: user.publicKey,
      usdcMint: USDC_MINT,
    }).signers([user]).rpc();
    console.log("Your transaction signature", tx);

    // Verify vault account is initialized
    const vault_account = await program.account.vault.fetch(vault_account_pda);
    assert.equal(vault_account.owner.toBase58(), user.publicKey.toBase58());
    assert.equal(vault_account.bump, vault_seed);

    const usdc_vault_token_account = await connection.getAccountInfo(VAULT_USDC_ATA);
    assert.equal(usdc_vault_token_account?.owner.toBase58(), TOKEN_PROGRAM_ID.toBase58());
  });

  it("Deposit USDC", async () => {
    const tx = await program.methods.depositUsdc(new anchor.BN(1_000)).accounts({
      owner: user.publicKey,
      ownerUsdcAccount: USER_USDC_ATA,
      usdcMint: USDC_MINT,
    }).signers([user]).rpc();
    console.log("Deposit transaction signature", tx);

    const vaultAccountInfo = await getAccount(connection, VAULT_USDC_ATA);
    const balance = vaultAccountInfo.amount;
    assert.equal(balance.toString(), "1000");

    const userAta = await getAccount(connection, USER_USDC_ATA);
    const userBalance = userAta.amount; // bigint
    assert.equal(userBalance.toString(), "999999000"); 
  })

  it("Withdraw USDC", async () => {
    const tx = await program.methods.withdrawUsdc(new anchor.BN(1000)).accounts({
      owner: user.publicKey,
      ownerUsdcAccount: USER_USDC_ATA,
      usdcMint: USDC_MINT,
    }).signers([user]).rpc();
    console.log("Withdraw transaction signature", tx);

    const vaultAccountInfo = await getAccount(connection, VAULT_USDC_ATA);
    const balance = vaultAccountInfo.amount; // bigint
    assert.equal(balance.toString(), "0"); 

    const userAta = await getAccount(connection, USER_USDC_ATA);
    const userBalance = userAta.amount; // bigint
    assert.equal(userBalance.toString(), "1000000000"); 
  })


});

async function foundAccount(program: Program<YieldVault>, account: PublicKey) {
  const tx = await program.provider.connection.confirmTransaction(
    await program.provider.connection.requestAirdrop(account, 100 * anchor.web3.LAMPORTS_PER_SOL),
    "confirmed"
  ); 
  console.log("Airdropped transaction signature", tx);
}
