import * as anchor from "@anchor-lang/core";
import { Program, BN } from "@anchor-lang/core";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";
import { SolanaAmm } from "../target/types/solana_amm";

describe("Constant Product AMM Tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const connection = provider.connection;

  const program = anchor.workspace.solanaAmm as Program<SolanaAmm>;

  // Keypairs
  const admin = provider.wallet as anchor.Wallet;
  const alice = Keypair.generate(); // Liquidity Provider 1
  const bob = Keypair.generate(); // Liquidity Provider 2
  const charlie = Keypair.generate(); // Trader
  const treasury = Keypair.generate(); // Treasury Account

  let mintX: PublicKey;
  let mintY: PublicKey;
  let mintLp: PublicKey;

  let configPda: PublicKey;
  let configBump: number;
  let lpBump: number;

  let vaultX: PublicKey;
  let vaultY: PublicKey;

  let treasuryX: PublicKey;
  let treasuryY: PublicKey;

  let aliceX: PublicKey;
  let aliceY: PublicKey;
  let aliceLp: PublicKey;

  let bobX: PublicKey;
  let bobY: PublicKey;
  let bobLp: PublicKey;

  let charlieX: PublicKey;
  let charlieY: PublicKey;

  const poolSeed = new BN(12345);
  const initialFeeBps = 30; // 0.30% total fee
  const initialProtocolFeeBps = 5; // 0.05% treasury fee (cut of swap)

  before(async () => {
    // 1. Airdrop SOL to test users
    const airdropAlice = await connection.requestAirdrop(
      alice.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    const airdropBob = await connection.requestAirdrop(
      bob.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    const airdropCharlie = await connection.requestAirdrop(
      charlie.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    const airdropTreasury = await connection.requestAirdrop(
      treasury.publicKey,
      2 * LAMPORTS_PER_SOL
    );

    const latestBlockhash = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature: airdropAlice,
      ...latestBlockhash,
    });
    await connection.confirmTransaction({
      signature: airdropBob,
      ...latestBlockhash,
    });
    await connection.confirmTransaction({
      signature: airdropCharlie,
      ...latestBlockhash,
    });
    await connection.confirmTransaction({
      signature: airdropTreasury,
      ...latestBlockhash,
    });

    // 2. Create Token X and Token Y mints
    // To ensure deterministic order and avoid IdenticalMints error
    const rawMintA = await createMint(
      connection,
      admin.payer,
      admin.publicKey,
      null,
      6
    );
    const rawMintB = await createMint(
      connection,
      admin.payer,
      admin.publicKey,
      null,
      6
    );

    // Sort mints for consistency
    if (rawMintA.toBuffer().compare(rawMintB.toBuffer()) < 0) {
      mintX = rawMintA;
      mintY = rawMintB;
    } else {
      mintX = rawMintB;
      mintY = rawMintA;
    }

    // 3. Derive PDAs
    const seedBytes = poolSeed.toArrayLike(Buffer, "le", 8);
    [configPda, configBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), seedBytes],
      program.programId
    );

    [mintLp, lpBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), configPda.toBuffer()],
      program.programId
    );

    // Derive associated token accounts
    vaultX = anchor.utils.token.associatedAddress({
      mint: mintX,
      owner: configPda,
    });
    vaultY = anchor.utils.token.associatedAddress({
      mint: mintY,
      owner: configPda,
    });

    // 4. Create user and treasury token accounts
    const aliceXAta = await getOrCreateAssociatedTokenAccount(
      connection,
      alice,
      mintX,
      alice.publicKey
    );
    aliceX = aliceXAta.address;

    const aliceYAta = await getOrCreateAssociatedTokenAccount(
      connection,
      alice,
      mintY,
      alice.publicKey
    );
    aliceY = aliceYAta.address;

    aliceLp = anchor.utils.token.associatedAddress({
      mint: mintLp,
      owner: alice.publicKey,
    });

    const bobXAta = await getOrCreateAssociatedTokenAccount(
      connection,
      bob,
      mintX,
      bob.publicKey
    );
    bobX = bobXAta.address;

    const bobYAta = await getOrCreateAssociatedTokenAccount(
      connection,
      bob,
      mintY,
      bob.publicKey
    );
    bobY = bobYAta.address;

    bobLp = anchor.utils.token.associatedAddress({
      mint: mintLp,
      owner: bob.publicKey,
    });

    const charlieXAta = await getOrCreateAssociatedTokenAccount(
      connection,
      charlie,
      mintX,
      charlie.publicKey
    );
    charlieX = charlieXAta.address;

    const charlieYAta = await getOrCreateAssociatedTokenAccount(
      connection,
      charlie,
      mintY,
      charlie.publicKey
    );
    charlieY = charlieYAta.address;

    const treasuryXAta = await getOrCreateAssociatedTokenAccount(
      connection,
      treasury,
      mintX,
      treasury.publicKey
    );
    treasuryX = treasuryXAta.address;

    const treasuryYAta = await getOrCreateAssociatedTokenAccount(
      connection,
      treasury,
      mintY,
      treasury.publicKey
    );
    treasuryY = treasuryYAta.address;

    // 5. Mint tokens to users
    // Alice gets 10,000,000 X and 20,000,000 Y
    await mintTo(
      connection,
      admin.payer,
      mintX,
      aliceX,
      admin.publicKey,
      10_000_000
    );
    await mintTo(
      connection,
      admin.payer,
      mintY,
      aliceY,
      admin.publicKey,
      20_000_000
    );

    // Bob gets 5,000,000 X and 10,000,000 Y
    await mintTo(
      connection,
      admin.payer,
      mintX,
      bobX,
      admin.publicKey,
      5_000_000
    );
    await mintTo(
      connection,
      admin.payer,
      mintY,
      bobY,
      admin.publicKey,
      10_000_000
    );

    // Charlie gets 1,000,000 X and 1,000,000 Y
    await mintTo(
      connection,
      admin.payer,
      mintX,
      charlieX,
      admin.publicKey,
      1_000_000
    );
    await mintTo(
      connection,
      admin.payer,
      mintY,
      charlieY,
      admin.publicKey,
      1_000_000
    );
  });

  it("1. Initialize AMM pool with fees and treasury", async () => {
    const tx = await program.methods
      .initialize(
        poolSeed,
        initialFeeBps,
        initialProtocolFeeBps,
        admin.publicKey
      )
      .accountsPartial({
        initializer: admin.publicKey,
        mintX,
        mintY,
        mintLp,
        vaultX,
        vaultY,
        treasury: treasury.publicKey,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    expect(tx).to.be.a("string");

    // Fetch config and verify
    const configAccount = await program.account.config.fetch(configPda);
    expect(configAccount.seed.toNumber()).to.equal(poolSeed.toNumber());
    expect(configAccount.authority.toBase58()).to.equal(
      admin.publicKey.toBase58()
    );
    expect(configAccount.mintX.toBase58()).to.equal(mintX.toBase58());
    expect(configAccount.mintY.toBase58()).to.equal(mintY.toBase58());
    expect(configAccount.fee).to.equal(initialFeeBps);
    expect(configAccount.protocolFee).to.equal(initialProtocolFeeBps);
    expect(configAccount.treasury.toBase58()).to.equal(
      treasury.publicKey.toBase58()
    );
    expect(configAccount.locked).to.equal(false);
    expect(configAccount.configBump).to.equal(configBump);
    expect(configAccount.lpBump).to.equal(lpBump);

    // Vaults should be initialized with 0 balance
    const vaultXAccount = await getAccount(connection, vaultX);
    const vaultYAccount = await getAccount(connection, vaultY);
    expect(Number(vaultXAccount.amount)).to.equal(0);
    expect(Number(vaultYAccount.amount)).to.equal(0);
  });

  it("2. Initial liquidity deposit (bootstrap CPMM pool)", async () => {
    // Create Alice's LP token account now that mintLp is initialized
    await getOrCreateAssociatedTokenAccount(
      connection,
      alice,
      mintLp,
      alice.publicKey
    );

    const depositX = 1_000_000; // 1.0 Token X
    const depositY = 2_000_000; // 2.0 Token Y
    // sqrt(1_000_000 * 2_000_000) = sqrt(2e12) = 1_414_213
    const expectedLp = 1_414_213;

    await program.methods
      .deposit(new BN(expectedLp), new BN(depositX), new BN(depositY))
      .accountsPartial({
        depositor: alice.publicKey,
        mintX,
        mintY,
        mintLp,
        vaultX,
        vaultY,
        depositorX: aliceX,
        depositorY: aliceY,
        depositorLp: aliceLp,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([alice])
      .rpc();

    // Verify balances
    const aliceLpAccount = await getAccount(connection, aliceLp);
    const vaultXAccount = await getAccount(connection, vaultX);
    const vaultYAccount = await getAccount(connection, vaultY);

    expect(Number(aliceLpAccount.amount)).to.equal(expectedLp);
    expect(Number(vaultXAccount.amount)).to.equal(depositX);
    expect(Number(vaultYAccount.amount)).to.equal(depositY);
  });

  it("3. Subsequent liquidity deposit maintaining constant product ratio", async () => {
    // Create Bob's LP token account
    await getOrCreateAssociatedTokenAccount(
      connection,
      bob,
      mintLp,
      bob.publicKey
    );

    // Bob deposits 50% more liquidity
    // To mint 707,106 LP tokens:
    // dx = ceil(707_106 * 1_000_000 / 1_414_213) = 500_000
    // dy = ceil(707_106 * 2_000_000 / 1_414_213) = 1_000_000
    const desiredLp = 707_106;
    const maxDepositX = 500_005;
    const maxDepositY = 1_000_010;

    await program.methods
      .deposit(new BN(desiredLp), new BN(maxDepositX), new BN(maxDepositY))
      .accountsPartial({
        depositor: bob.publicKey,
        mintX,
        mintY,
        mintLp,
        vaultX,
        vaultY,
        depositorX: bobX,
        depositorY: bobY,
        depositorLp: bobLp,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([bob])
      .rpc();

    const bobLpAccount = await getAccount(connection, bobLp);
    const vaultXAccount = await getAccount(connection, vaultX);
    const vaultYAccount = await getAccount(connection, vaultY);

    expect(Number(bobLpAccount.amount)).to.equal(desiredLp);
    expect(Number(vaultXAccount.amount)).to.equal(1_500_000);
    expect(Number(vaultYAccount.amount)).to.equal(3_000_000);
  });

  it("4. Deposit slippage protection: reject when required input exceeds max", async () => {
    const desiredLp = 100_000;
    // Deliberately set max_x too low
    const insufficientMaxX = 10;
    const maxDepositY = 500_000;

    let failed = false;
    try {
      await program.methods
        .deposit(new BN(desiredLp), new BN(insufficientMaxX), new BN(maxDepositY))
        .accountsPartial({
          depositor: bob.publicKey,
          mintX,
          mintY,
          mintLp,
          vaultX,
          vaultY,
          depositorX: bobX,
          depositorY: bobY,
          depositorLp: bobLp,
          config: configPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([bob])
        .rpc();
    } catch (err: any) {
      failed = true;
      expect(err.toString()).to.include("MaxInputExceeded");
    }
    expect(failed).to.be.true;
  });

  it("5. Swap Token X for Token Y with fee deduction and treasury protocol fee", async () => {
    // Current reserves: X = 1,500,000, Y = 3,000,000
    // Charlie swaps 100,000 X
    // Total fee (30 bps) = 100_000 * 30 / 10_000 = 300 X
    // Protocol fee (5 bps) = 100_000 * 5 / 10_000 = 50 X (transferred to treasury)
    // LP fee = 250 X (stays in pool)
    // Effective in = 100_000 - 300 = 99_700 X
    // dy = floor((3_000_000 * 99_700) / (1_500_000 + 99_700)) = 186_972 Y
    const amountIn = 100_000;
    const minAmountOut = 186_000; // Slippage limit

    const charlieYBefore = await getAccount(connection, charlieY);
    const treasuryXBefore = await getAccount(connection, treasuryX);
    const vaultXBefore = await getAccount(connection, vaultX);
    const vaultYBefore = await getAccount(connection, vaultY);

    await program.methods
      .swap(new BN(amountIn), new BN(minAmountOut), true)
      .accountsPartial({
        user: charlie.publicKey,
        mintX,
        mintY,
        vaultX,
        vaultY,
        userX: charlieX,
        userY: charlieY,
        treasuryX,
        treasuryY,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([charlie])
      .rpc();

    const charlieYAfter = await getAccount(connection, charlieY);
    const treasuryXAfter = await getAccount(connection, treasuryX);
    const vaultXAfter = await getAccount(connection, vaultX);
    const vaultYAfter = await getAccount(connection, vaultY);

    const receivedY = Number(charlieYAfter.amount) - Number(charlieYBefore.amount);
    const treasuryXGained =
      Number(treasuryXAfter.amount) - Number(treasuryXBefore.amount);
    const vaultXGained =
      Number(vaultXAfter.amount) - Number(vaultXBefore.amount);
    const vaultYLost =
      Number(vaultYBefore.amount) - Number(vaultYAfter.amount);

    expect(receivedY).to.equal(186_972);
    expect(treasuryXGained).to.equal(50); // Protocol fee verified in treasury!
    expect(vaultXGained).to.equal(99_950); // 100_000 in - 50 protocol fee
    expect(vaultYLost).to.equal(186_972);

    // Verify invariant k increased due to LP fee retention
    const kBefore =
      BigInt(vaultXBefore.amount.toString()) *
      BigInt(vaultYBefore.amount.toString());
    const kAfter =
      BigInt(vaultXAfter.amount.toString()) *
      BigInt(vaultYAfter.amount.toString());
    expect(kAfter >= kBefore).to.be.true;
  });

  it("6. Swap Token Y for Token X with fee deduction and treasury protocol fee", async () => {
    const amountInY = 200_000;
    const minAmountOutX = 90_000;

    const charlieXBefore = await getAccount(connection, charlieX);
    const treasuryYBefore = await getAccount(connection, treasuryY);

    await program.methods
      .swap(new BN(amountInY), new BN(minAmountOutX), false)
      .accountsPartial({
        user: charlie.publicKey,
        mintX,
        mintY,
        vaultX,
        vaultY,
        userX: charlieX,
        userY: charlieY,
        treasuryX,
        treasuryY,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([charlie])
      .rpc();

    const charlieXAfter = await getAccount(connection, charlieX);
    const treasuryYAfter = await getAccount(connection, treasuryY);

    const receivedX = Number(charlieXAfter.amount) - Number(charlieXBefore.amount);
    const treasuryYGained =
      Number(treasuryYAfter.amount) - Number(treasuryYBefore.amount);

    // Protocol fee (5 bps of 200_000) = 100 Y
    expect(treasuryYGained).to.equal(100);
    expect(receivedX).to.be.greaterThan(minAmountOutX);
  });

  it("7. Swap slippage protection: reject when output is below min_amount_out", async () => {
    const amountIn = 50_000;
    const unattainableMinOut = 1_000_000; // Demanding far more than possible

    let failed = false;
    try {
      await program.methods
        .swap(new BN(amountIn), new BN(unattainableMinOut), true)
        .accountsPartial({
          user: charlie.publicKey,
          mintX,
          mintY,
          vaultX,
          vaultY,
          userX: charlieX,
          userY: charlieY,
          treasuryX,
          treasuryY,
          config: configPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([charlie])
        .rpc();
    } catch (err: any) {
      failed = true;
      expect(err.toString()).to.include("SlippageExceeded");
    }
    expect(failed).to.be.true;
  });

  it("8. Partial liquidity withdrawal by LP", async () => {
    // Bob withdraws half of his LP tokens: 353_553 LP
    const withdrawLp = 353_553;
    const bobXBefore = await getAccount(connection, bobX);
    const bobYBefore = await getAccount(connection, bobY);
    const bobLpBefore = await getAccount(connection, bobLp);

    await program.methods
      .withdraw(new BN(withdrawLp), new BN(1), new BN(1))
      .accountsPartial({
        withdrawer: bob.publicKey,
        mintX,
        mintY,
        mintLp,
        vaultX,
        vaultY,
        withdrawerX: bobX,
        withdrawerY: bobY,
        withdrawerLp: bobLp,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([bob])
      .rpc();

    const bobXAfter = await getAccount(connection, bobX);
    const bobYAfter = await getAccount(connection, bobY);
    const bobLpAfter = await getAccount(connection, bobLp);

    expect(Number(bobLpBefore.amount) - Number(bobLpAfter.amount)).to.equal(
      withdrawLp
    );
    expect(Number(bobXAfter.amount)).to.be.greaterThan(
      Number(bobXBefore.amount)
    );
    expect(Number(bobYAfter.amount)).to.be.greaterThan(
      Number(bobYBefore.amount)
    );
  });

  it("9. Withdrawal slippage protection: reject when min tokens requested exceeds share", async () => {
    const withdrawLp = 100_000;
    const impossibleMinX = 5_000_000;

    let failed = false;
    try {
      await program.methods
        .withdraw(new BN(withdrawLp), new BN(impossibleMinX), new BN(1))
        .accountsPartial({
          withdrawer: bob.publicKey,
          mintX,
          mintY,
          mintLp,
          vaultX,
          vaultY,
          withdrawerX: bobX,
          withdrawerY: bobY,
          withdrawerLp: bobLp,
          config: configPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([bob])
        .rpc();
    } catch (err: any) {
      failed = true;
      expect(err.toString()).to.include("SlippageExceeded");
    }
    expect(failed).to.be.true;
  });

  it("10. Admin: Update fee and protocol fee configuration", async () => {
    const newFee = 50; // 0.50%
    const newProtocolFee = 10; // 0.10%

    // Successful update by authority
    await program.methods
      .updateFee(newFee, newProtocolFee)
      .accountsPartial({
        authority: admin.publicKey,
        config: configPda,
      })
      .rpc();

    const configAccount = await program.account.config.fetch(configPda);
    expect(configAccount.fee).to.equal(newFee);
    expect(configAccount.protocolFee).to.equal(newProtocolFee);

    // Non-authority attempt should be rejected
    let unauthorizedFailed = false;
    try {
      await program.methods
        .updateFee(100, 20)
        .accountsPartial({
          authority: alice.publicKey,
          config: configPda,
        })
        .signers([alice])
        .rpc();
    } catch (err: any) {
      unauthorizedFailed = true;
      expect(err.toString()).to.include("Unauthorized");
    }
    expect(unauthorizedFailed).to.be.true;
  });

  it("11. Admin: Update treasury address", async () => {
    const newTreasury = Keypair.generate().publicKey;

    await program.methods
      .updateTreasury(newTreasury)
      .accountsPartial({
        authority: admin.publicKey,
        config: configPda,
      })
      .rpc();

    let configAccount = await program.account.config.fetch(configPda);
    expect(configAccount.treasury.toBase58()).to.equal(newTreasury.toBase58());

    // Revert back to original treasury for subsequent tests
    await program.methods
      .updateTreasury(treasury.publicKey)
      .accountsPartial({
        authority: admin.publicKey,
        config: configPda,
      })
      .rpc();

    configAccount = await program.account.config.fetch(configPda);
    expect(configAccount.treasury.toBase58()).to.equal(
      treasury.publicKey.toBase58()
    );
  });

  it("12. Emergency Circuit Breaker: lock and unlock pool to mitigate downtime", async () => {
    // 1. Authority locks pool
    await program.methods
      .setLocked(true)
      .accountsPartial({
        authority: admin.publicKey,
        config: configPda,
      })
      .rpc();

    const configLocked = await program.account.config.fetch(configPda);
    expect(configLocked.locked).to.be.true;

    // 2. Swaps should fail when locked
    let swapBlocked = false;
    try {
      await program.methods
        .swap(new BN(10_000), new BN(1), true)
        .accountsPartial({
          user: charlie.publicKey,
          mintX,
          mintY,
          vaultX,
          vaultY,
          userX: charlieX,
          userY: charlieY,
          treasuryX,
          treasuryY,
          config: configPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([charlie])
        .rpc();
    } catch (err: any) {
      swapBlocked = true;
      expect(err.toString()).to.include("PoolLocked");
    }
    expect(swapBlocked).to.be.true;

    // 3. Deposits should fail when locked
    let depositBlocked = false;
    try {
      await program.methods
        .deposit(new BN(10_000), new BN(100_000), new BN(100_000))
        .accountsPartial({
          depositor: bob.publicKey,
          mintX,
          mintY,
          mintLp,
          vaultX,
          vaultY,
          depositorX: bobX,
          depositorY: bobY,
          depositorLp: bobLp,
          config: configPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([bob])
        .rpc();
    } catch (err: any) {
      depositBlocked = true;
      expect(err.toString()).to.include("PoolLocked");
    }
    expect(depositBlocked).to.be.true;

    // 4. Authority unlocks pool
    await program.methods
      .setLocked(false)
      .accountsPartial({
        authority: admin.publicKey,
        config: configPda,
      })
      .rpc();

    const configUnlocked = await program.account.config.fetch(configPda);
    expect(configUnlocked.locked).to.be.false;

    // 5. Swap succeeds again after unlocking
    await program.methods
      .swap(new BN(10_000), new BN(1), true)
      .accountsPartial({
        user: charlie.publicKey,
        mintX,
        mintY,
        vaultX,
        vaultY,
        userX: charlieX,
        userY: charlieY,
        treasuryX,
        treasuryY,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([charlie])
      .rpc();
  });

  it("13. Full liquidity withdrawal", async () => {
    // Alice withdraws all her remaining LP tokens
    const aliceLpAccount = await getAccount(connection, aliceLp);
    const aliceLpBalance = Number(aliceLpAccount.amount);
    expect(aliceLpBalance).to.be.greaterThan(0);

    const aliceXBefore = await getAccount(connection, aliceX);
    const aliceYBefore = await getAccount(connection, aliceY);

    await program.methods
      .withdraw(new BN(aliceLpBalance), new BN(1), new BN(1))
      .accountsPartial({
        withdrawer: alice.publicKey,
        mintX,
        mintY,
        mintLp,
        vaultX,
        vaultY,
        withdrawerX: aliceX,
        withdrawerY: aliceY,
        withdrawerLp: aliceLp,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([alice])
      .rpc();

    const aliceLpAfter = await getAccount(connection, aliceLp);
    const aliceXAfter = await getAccount(connection, aliceX);
    const aliceYAfter = await getAccount(connection, aliceY);

    expect(Number(aliceLpAfter.amount)).to.equal(0);
    expect(Number(aliceXAfter.amount)).to.be.greaterThan(
      Number(aliceXBefore.amount)
    );
    expect(Number(aliceYAfter.amount)).to.be.greaterThan(
      Number(aliceYBefore.amount)
    );
  });
});
