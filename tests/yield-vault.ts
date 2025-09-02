import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { YieldVault } from "../target/types/yield_vault";
import { PublicKey, Connection, Keypair } from "@solana/web3.js";
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
import { getBankVaultAuthority, BankVaultType } from "@mrgnlabs/marginfi-client-v2"; 

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

// Marginfi:
const MARGINFI_PROGRAM = new PublicKey("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");
const MARGINFI_GROUP = new PublicKey("4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");
const MARGINFI_BANK = new PublicKey("2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB");
const MARGINFI_BANK_USDC_LIQUIDITY_VAULT = new PublicKey("7jaiZR5Sk8hdYN9MxTpczTcwbWpb5WEoxSANuUwveuat");
const MARGINFI_BANK_USDC_LIQUIDITY_VAULT_AUTH = new PublicKey("3uxNepDbmkDNq6JhRja5Z8QwbTrfmkKP8AKZV5chYDGG");
// const mint_authority = new PublicKey("BJE5MMbqXjVwjAF7oxwPYXnTXDyspzZyt4vwenNw5ruG");

describe("yield-vault", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.yieldVault as Program<YieldVault>;
  const user = anchor.web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(`${os.homedir()}/.config/solana/user.json`, "utf8"))));
  const keeper = anchor.web3.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(`/Users/semi/wrksp/solana/exp/yield-vault/keeper/bot7F9sfkm5ztmMGL11St2PD9necoEY6fC84L1WKMDg.json`, "utf8"))));

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

    let ownerAta = await getOrCreateAssociatedTokenAccount(connection, user, USDC_MINT, user.publicKey);
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

    const marginfi_bank_usdc_liquidity_vault_auth = await connection.getAccountInfo(MARGINFI_BANK_USDC_LIQUIDITY_VAULT_AUTH);
    if (!marginfi_bank_usdc_liquidity_vault_auth) {
      throw new Error("Marginfi bank USDC liquidity vault authority not found. Make sure to start test validator with --clone 3uxNepDbmkDNq6JhRja5Z8QwbTrfmkKP8AKZV5chYDGG");
    }
    console.log("✅ Marginfi bank USDC liquidity vault authority found:", MARGINFI_BANK_USDC_LIQUIDITY_VAULT_AUTH.toBase58());

    // Validate Marginfi bank account
    const marginfi_bank = await connection.getAccountInfo(MARGINFI_BANK);
    if (!marginfi_bank) {
      throw new Error("Marginfi bank not found. Make sure to start test validator with --clone 2s37akK2eyBbp8DZgCm7RtsaEz8eJP3Nxd4urLHQv7yB");
    }
    console.log("✅ Marginfi bank found:", MARGINFI_BANK.toBase58());

    // Validate Marginfi group account
    const marginfi_group = await connection.getAccountInfo(MARGINFI_GROUP);
    if (!marginfi_group) {
      throw new Error("Marginfi group not found. Make sure to start test validator with --clone 4qp6Fx6tnZkY5Wropq9wUYgtFxXKwE6viZxFHg3rdAG8");
    }
    console.log("✅ Marginfi group found:", MARGINFI_GROUP.toBase58());

    // Validate Marginfi bank liquidity vault
    const marginfi_bank_liquidity_vault = await connection.getAccountInfo(MARGINFI_BANK_USDC_LIQUIDITY_VAULT);
    if (!marginfi_bank_liquidity_vault) {
      throw new Error("Marginfi bank USDC liquidity vault not found. Make sure to start test validator with --clone 7jaiZR5Sk8hdYN9MxTpczTcwbWpb5WEoxSANuUwveuat");
    }
    console.log("✅ Marginfi bank USDC liquidity vault found:", MARGINFI_BANK_USDC_LIQUIDITY_VAULT.toBase58());
  });

  // it("Is initialized!", async () => {
  //   const MARGINFI_ACCOUNT = Keypair.generate();
  //   console.log("📦 MARGINFI_ACCOUNT", MARGINFI_ACCOUNT.publicKey.toBase58());
  //   const tx = await program.methods.initialize().accounts({
  //     user: user.publicKey,
  //     usdcMint: USDC_MINT,
  //     kaminoUsdcCollateralMint: KLEND_COLLATERAL_MINT,
  //     marginfiAccount: MARGINFI_ACCOUNT.publicKey,
  //     marginfiGroup: MARGINFI_GROUP,
  //   }).signers([user, MARGINFI_ACCOUNT]).rpc();
  //   console.log("Your transaction signature", tx);
    
  //   await bumpSlot(connection, program.provider.wallet.payer);
  //   await bumpSlot(connection, program.provider.wallet.payer);
  //   // Verify vault account is initialized
  //   const vault_account = await program.account.userVault.fetch(vault_account_pda);
  //   assert.equal(vault_account.owner.toBase58(), user.publicKey.toBase58());
  //   assert.equal(vault_account.bump, vault_seed);

  //   const usdc_vault_token_account = await connection.getAccountInfo(VAULT_USDC_ATA);
  //   assert.equal(usdc_vault_token_account?.owner.toBase58(), TOKEN_PROGRAM_ID.toBase58());

  // });

  it("Deposit USDC to Marginfi", async () => {
    const marginfi_account = await program.account.userVault.fetch(vault_account_pda);
    console.log("👀 Fetched Marginfi account:", marginfi_account.marginfiAccount.toBase58());

    const tx = await program.methods.depositUsdcMarginfi(new anchor.BN(45_000_000)).accounts({
      owner: user.publicKey,
      ownerUsdcAccount: USER_USDC_ATA,
      usdcMint: USDC_MINT,
      marginfiGroup: MARGINFI_GROUP,
      marginfiAccount: marginfi_account.marginfiAccount,
      marginfiBank: MARGINFI_BANK,
      marginfiBankLiquidityVault: MARGINFI_BANK_USDC_LIQUIDITY_VAULT,
    }).signers([user]).rpc();
    console.log("Marginfi Deposit transaction signature", tx);
    await bumpSlot(connection, program.provider.wallet.payer);
  })

  it("Withdraw USDC from Marginfi", async () => {
    // fetch marginfi account from vault
    const marginfi_account = await program.account.userVault.fetch(vault_account_pda);
    console.log("👀 Fetched Marginfi account:", marginfi_account.marginfiAccount.toBase58());

    const tx = await program.methods.withdrawUsdcMarginfi().accounts({
      owner: user.publicKey,
      ownerUsdcAccount: USER_USDC_ATA,
      usdcMint: USDC_MINT,
      marginfiGroup: MARGINFI_GROUP,
      marginfiAccount: marginfi_account.marginfiAccount,
      marginfiBank: MARGINFI_BANK,
      marginfiBankLiquidityVault: MARGINFI_BANK_USDC_LIQUIDITY_VAULT,
      marginfiBankLiquidityVaultAuthority: MARGINFI_BANK_USDC_LIQUIDITY_VAULT_AUTH,
    }).signers([user]).rpc();
    console.log("Marginfi Withdraw transaction signature", tx);
    await bumpSlot(connection, program.provider.wallet.payer);
  })

  // it("Deposit USDC Kamino", async () => {
  //   const tx = await program.methods.deployUsdcKamino(new anchor.BN(4_000_000)).accounts({
  //     keeper:                       keeper.publicKey,
  //     user:                         user.publicKey,
  //     usdcMint:                     USDC_MINT,
  //     kaminoLendingMarket:          KLEND_MAIN_LENDING_MARKET,
  //     kaminoReserve:                KLEND_USDC_RESEVE,
  //     kaminoUsdcCollateralMint:     KLEND_COLLATERAL_MINT,
  //     kaminoLendingMarketAuthority: KLEND_LENDING_MARKET_AUTHORITY,
  //     kaminoReserveLiquiditySupply: KLEND_RESERVE_LIQUIDITY_SUPPLY,
  //   }).signers([keeper]).rpc();
  //   console.log("Kamino Deposit transaction signature", tx);
  //   await bumpSlot(connection, program.provider.wallet.payer);
  // })

  // it("Withdraw USDC Kamino", async () => {
  //   const tx = await program.methods.redeemUsdcKaminio().accounts({
  //     keeper: keeper.publicKey,
  //     user: user.publicKey,
  //     usdcMint: USDC_MINT,
  //     kaminoLendingMarket: KLEND_MAIN_LENDING_MARKET,
  //     kaminoReserve: KLEND_USDC_RESEVE,
  //     kaminoUsdcCollateralMint: KLEND_COLLATERAL_MINT,
  //     kaminoLendingMarketAuthority: KLEND_LENDING_MARKET_AUTHORITY,
  //     kaminoReserveLiquiditySupply: KLEND_RESERVE_LIQUIDITY_SUPPLY,
  //   }).signers([keeper]).rpc();
  //   console.log("Kamino Withdraw transaction signature", tx);
  // })


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
    await new Promise((r) => setTimeout(r, 2500));
    s = await conn.getSlot();
    console.log("Waiting for next slot...", s);
    tries++;
  }
}

async function bumpSlot(conn: Connection, payer: anchor.web3.Keypair) {
  // Nudge the validator forward
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