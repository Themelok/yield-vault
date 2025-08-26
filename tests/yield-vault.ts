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
import * as fs from "fs";
import * as os from "os";

// let USDC_MINT: PublicKey;
let USER_USDC_ATA: PublicKey;
let VAULT_USDC_ATA: PublicKey;

const USDC_AMOUNT = 1000;


const VAULT_SEED = Buffer.from("vault");
// const USDC_VAULT_TOKEN_ACCOUNT_SEED = Buffer.from("usdc_vault");
const USDC_MINT = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
const KLEND_PROGRAM = new PublicKey("KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
const KLEND_MAIN_LENDING_MARKET = new PublicKey("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");
const KLEND_USDC_RESEVE = new PublicKey("D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59");
const KLEND_COLLATERAL_MINT = new PublicKey("B8V6WVjPxW1UGwVDfxH2d2r8SyT4cqn7dQRK6XneVa7D");
const KLEND_RESERVE_LIQUIDITY_SUPPLY = new PublicKey("Bgq7trRgVMeq33yt235zM2onQ4bRDBsY5EWiTetF4qw6");
const KLEND_LENDING_MARKET_AUTHORITY = new PublicKey("9DrvZvyWh1HuAoZxvYWMvkf2XCzryCpGgHqrMjyDWpmo");
// const mint_authority = new PublicKey("BJE5MMbqXjVwjAF7oxwPYXnTXDyspzZyt4vwenNw5ruG");

describe("yield-vault", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.yieldVault as Program<YieldVault>;
  const user = anchor.web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(`${os.homedir()}/.config/solana/user.json`, "utf8"))));
  
  const [vault_account_pda, vault_seed] = PublicKey.findProgramAddressSync([VAULT_SEED, user.publicKey.toBuffer()], program.programId);
  const connection = program.provider.connection;

  before(async () => {
    // Verify USDC mint account exists (should be cloned by test validator)
    const mintInfo = await connection.getAccountInfo(USDC_MINT);
    if (!mintInfo) {
      throw new Error("USDC mint account not found. Make sure to start test validator with --clone EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    }
    console.log("✅ USDC mint account found:", USDC_MINT.toBase58());

    const klen_program_id = await connection.getAccountInfo(KLEND_PROGRAM);
    if (!klen_program_id) {
      throw new Error("Klen program not found. Make sure to start test validator with --clone KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD");
    }
    console.log("✅ Klen program found:", KLEND_PROGRAM.toBase58());

    const klen_main_lending_market = await connection.getAccountInfo(KLEND_MAIN_LENDING_MARKET);
    if (!klen_main_lending_market) {
      throw new Error("Klen main lending market not found. Make sure to start test validator with --clone 7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");
    }
    console.log("✅ Klen main lending market found:", KLEND_MAIN_LENDING_MARKET.toBase58());

    const klen_usdc_reserve = await connection.getAccountInfo(KLEND_USDC_RESEVE);
    if (!klen_usdc_reserve) {
      throw new Error("Klen USDC reserve not found. Make sure to start test validator with --clone D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59");
    }
    console.log("✅ Klen USDC reserve found:", KLEND_USDC_RESEVE.toBase58());

    const klen_reserve_liquidity_supply = await connection.getAccountInfo(KLEND_RESERVE_LIQUIDITY_SUPPLY);
    if (!klen_reserve_liquidity_supply) {
      throw new Error("Klen reserve liquidity supply not found. Make sure to start test validator with --clone Bgq7trRgVMeq33yt235zM2onQ4bRDBsY5EWiTetF4qw6");
    }
    console.log("✅ Klen reserve liquidity supply found:", KLEND_RESERVE_LIQUIDITY_SUPPLY.toBase58());

    const klen_lending_market_authority = await connection.getAccountInfo(KLEND_LENDING_MARKET_AUTHORITY);
    if (!klen_lending_market_authority) {
      throw new Error("Klen lending market authority not found. Make sure to start test validator with --clone 9DrvZvyWh1HuAoZxvYWMvkf2XCzryCpGgHqrMjyDWpmo");
    }
    console.log("✅ Klen lending market authority found:", KLEND_LENDING_MARKET_AUTHORITY.toBase58());

    let ownerAta = await getOrCreateAssociatedTokenAccount(
      connection,
      user,
      USDC_MINT,
      user.publicKey
    );
    USER_USDC_ATA = ownerAta.address;
    if (!USER_USDC_ATA) {
      throw new Error("User USDC ATA not found");
    }
    console.log("✅ User USDC ATA:", USER_USDC_ATA.toBase58());

    const ata = getAssociatedTokenAddressSync(USDC_MINT, vault_account_pda, true);
    VAULT_USDC_ATA = ata;
    if (!VAULT_USDC_ATA) {
      throw new Error("Vault USDC ATA not found");
    }
    console.log("✅ Vault USDC ATA:", VAULT_USDC_ATA.toBase58());
  });

  it("Is initialized!", async () => {
    const tx = await program.methods.initialize().accounts({
      owner: user.publicKey,
      usdcMint: USDC_MINT,
      kaminoUsdcCollateralMint: KLEND_COLLATERAL_MINT,
    }).signers([user]).rpc();
    console.log("Your transaction signature", tx);
    // await program.provider.connection.confirmTransaction(tx, "finalized");
    await bumpSlot(connection, program.provider.wallet.payer);

    // Verify vault account is initialized
    const vault_account = await program.account.vault.fetch(vault_account_pda);
    assert.equal(vault_account.owner.toBase58(), user.publicKey.toBase58());
    assert.equal(vault_account.bump, vault_seed);

    const usdc_vault_token_account = await connection.getAccountInfo(VAULT_USDC_ATA);
    assert.equal(usdc_vault_token_account?.owner.toBase58(), TOKEN_PROGRAM_ID.toBase58());

  });

  it("Deposit USDC", async () => {
    const tx = await program.methods.depositUsdc(new anchor.BN(45_000_000)).accounts({
      owner:                        user.publicKey,
      ownerUsdcAccount:             USER_USDC_ATA,
      usdcMint:                     USDC_MINT,
      kaminoLendingMarket:          KLEND_MAIN_LENDING_MARKET,
      kaminoReserve:                KLEND_USDC_RESEVE,
      kaminoUsdcCollateralMint:     KLEND_COLLATERAL_MINT,
      kaminoLendingMarketAuthority: KLEND_LENDING_MARKET_AUTHORITY,
      kaminoReserveLiquiditySupply: KLEND_RESERVE_LIQUIDITY_SUPPLY,
    }).signers([user]).rpc();
    console.log("Deposit transaction signature", tx);
  })

  it("Withdraw USDC", async () => {
    const tx = await program.methods.withdrawUsdc(new anchor.BN(10_000_000)).accounts({
      owner: user.publicKey,
      ownerUsdcAccount: USER_USDC_ATA,
      usdcMint: USDC_MINT,
      kaminoLendingMarket: KLEND_MAIN_LENDING_MARKET,
      kaminoReserve: KLEND_USDC_RESEVE,
      kaminoUsdcCollateralMint: KLEND_COLLATERAL_MINT,
      kaminoLendingMarketAuthority: KLEND_LENDING_MARKET_AUTHORITY,
      kaminoReserveLiquiditySupply: KLEND_RESERVE_LIQUIDITY_SUPPLY,
    }).signers([user]).rpc();
    console.log("Withdraw transaction signature", tx);
  })


});

async function foundAccount(program: Program<YieldVault>, account: PublicKey) {
  const tx = await program.provider.connection.confirmTransaction(
    await program.provider.connection.requestAirdrop(account, 100 * anchor.web3.LAMPORTS_PER_SOL),
    "confirmed"
  ); 
  console.log("Airdropped transaction signature", tx);
}

async function waitNextSlot(conn: Connection) {
  const start = await conn.getSlot();
  let s = start;
  let tries = 0;
  while (s <= start && tries < 20) {
    await new Promise((r) => setTimeout(r, 250));
    s = await conn.getSlot();
    console.log("Waiting for next slot...", s);
    tries++;
  }
}

async function bumpSlot(conn: Connection, payer: anchor.web3.Keypair) {
  // Nudge the validator forward and wait for at least one new slot
  try {
    await conn.requestAirdrop(payer.publicKey, 1); // localnet only; harmless and cheap
  } catch (_) {
    // ignore if faucet not available; wait loop still helps
  }
  await waitNextSlot(conn);
}


    // const vaultAccountInfo = await getAccount(connection, VAULT_USDC_ATA);
    // const balance = vaultAccountInfo.amount;
    // assert.equal(balance.toString(), "1000");

    // const userAta = await getAccount(connection, USER_USDC_ATA);
    // const userBalance = userAta.amount; // bigint
    // assert.equal(userBalance.toString(), "123456788999000"); 


        // const vaultAccountInfo = await getAccount(connection, VAULT_USDC_ATA);
    // const balance = vaultAccountInfo.amount; // bigint
    // assert.equal(balance.toString(), "0"); 

    // const userAta = await getAccount(connection, USER_USDC_ATA);
    // const userBalance = userAta.amount; // bigint
    // assert.equal(userBalance.toString(), "123456789000000"); 