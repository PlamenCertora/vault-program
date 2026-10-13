import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VoltrVault } from "../target/types/voltr_vault";
import { VoltrBasicAdaptor } from "../target/types/voltr_basic_adaptor";
import { assert } from "chai";
import { PublicKey, Keypair } from "@solana/web3.js";
import {
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
  getMint,
  getAccount,
  createAssociatedTokenAccountIdempotent,
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
  createMint,
  mintTo,
} from "@solana/spl-token";
import { BN } from "bn.js";
import {
  convertDecimalBitsToDecimal,
  convertNumberToDecimal,
  fixedPointDiv,
} from "./conversion";
import { sendAndConfirmVersionedTransaction } from "./transaction";
import Decimal from "decimal.js";
import { SEEDS, VoltrClient, VaultConfigField } from "../sdk/src";

describe("voltr_vault", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const vaultProgram = anchor.workspace.VoltrVault as Program<VoltrVault>;
  const adaptorProgram = anchor.workspace
    .VoltrBasicAdaptor as Program<VoltrBasicAdaptor>;

  const client = new VoltrClient(provider.connection);

  const MAX_FEE_BPS = 10000;
  const REDEMPTION_FEE_PERCENTAGE_BPS = 100;
  const ISSUANCE_FEE_PERCENTAGE_BPS = 100;
  const DEAD_WEIGHT = new BN(1000);

  // Test accounts and variables
  let protocol: PublicKey;
  let protocolBump: number;
  let vaultLpMint: PublicKey;
  let vaultAssetIdleAuth: PublicKey;
  let vaultAssetIdleAta: PublicKey;
  let userAssetAta: PublicKey;
  let userLpAta: PublicKey;
  let newManagerLpAta: PublicKey;
  let adminLpAta: PublicKey;
  let requestWithdrawVaultReceipt: PublicKey;
  let requestWithdrawVaultReceiptBump: number;
  let requestWithdrawLpAta: PublicKey;
  let vaultAssetMint: PublicKey;
  let vaultStrategyAssetAta: PublicKey;
  let strategyAta: PublicKey;
  let vaultStrategyAuth: PublicKey;
  let strategyInitReceipt: PublicKey;

  const allOperationalState = 8191;
  const DisabledOperations = { WITHDRAW_VAULT: 1 << 1 };

  const metadataProgram = new PublicKey(
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
  );

  const payerKp = Keypair.generate();
  const payer = payerKp.publicKey;
  const vaultKp = Keypair.generate();
  const vault = vaultKp.publicKey;
  const manager = provider.wallet.publicKey;
  const admin = provider.wallet.publicKey;
  const newManagerKp = Keypair.generate();
  const newManager = newManagerKp.publicKey;

  const initialDepositAmount = new anchor.BN(8_000_000_000_000);
  const subsequentDepositAmount = new anchor.BN(2_000_000_000_000);
  const initialMintAmount = initialDepositAmount.add(subsequentDepositAmount);
  const withdrawLpAmount = new anchor.BN(2_000_000_000_000);
  const withdrawAssetAmount = new anchor.BN(2_000_000_000_000);
  const depositStrategyAmount = new anchor.BN(5_000_000_000_000);

  const vaultConfig = {
    maxCap: new BN(12_000_000_000_000), // to be changed in update vault
    startAtTs: new BN(0),
    lockedProfitDegradationDuration: new BN(0),
    managerPerformanceFee: 2000, // 20% in basis points
    adminPerformanceFee: 0, // 0% in basis points
    managerManagementFee: 2000, // 20% in basis points
    adminManagementFee: 0, // 0% in basis points
    redemptionFee: REDEMPTION_FEE_PERCENTAGE_BPS,
    issuanceFee: ISSUANCE_FEE_PERCENTAGE_BPS,
    withdrawalWaitingPeriod: new BN(10_000),
  };

  const vaultName = "Test Vault";
  const vaultDescription = "A test vault for automated testing";

  const [strategy] = PublicKey.findProgramAddressSync(
    [Buffer.from("strategy")],
    adaptorProgram.programId
  );

  it("Airdrops SOL to user", async () => {
    const airdropUserSig = await provider.connection.requestAirdrop(
      payer,
      90e9
    );
    const latestBlockhash = await provider.connection.getLatestBlockhash();

    await provider.connection.confirmTransaction({
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      signature: airdropUserSig,
    });
  });

  it("Initialize tokens and accounts using SDK", async () => {
    [protocol, protocolBump] = PublicKey.findProgramAddressSync([SEEDS.PROTOCOL], vaultProgram.programId);
    vaultAssetMint = await createMint(provider.connection, payerKp, provider.wallet.publicKey, null, 6);

    // Utilize SDK for PDAs
    vaultLpMint = client.findVaultLpMint(vault);
    vaultAssetIdleAuth = client.findVaultAssetIdleAuth(vault);
    requestWithdrawVaultReceipt = client.findRequestWithdrawVaultReceipt(vault, provider.wallet.publicKey);
    [, requestWithdrawVaultReceiptBump] = PublicKey.findProgramAddressSync(
      [SEEDS.REQUEST_WITHDRAW_VAULT_RECEIPT, vault.toBuffer(), provider.wallet.publicKey.toBuffer()],
      vaultProgram.programId
    );

    const vaultStratAddrs = client.findVaultStrategyAddresses(vault, strategy);
    vaultStrategyAuth = vaultStratAddrs.vaultStrategyAuth;
    strategyInitReceipt = vaultStratAddrs.strategyInitReceipt;

    requestWithdrawLpAta = getAssociatedTokenAddressSync(vaultLpMint, requestWithdrawVaultReceipt, true);
    vaultStrategyAssetAta = getAssociatedTokenAddressSync(vaultAssetMint, vaultStrategyAuth, true);
    strategyAta = getAssociatedTokenAddressSync(vaultAssetMint, strategy, true);
    vaultAssetIdleAta = getAssociatedTokenAddressSync(vaultAssetMint, vaultAssetIdleAuth, true);
    userAssetAta = getAssociatedTokenAddressSync(vaultAssetMint, provider.wallet.publicKey);
    userLpAta = getAssociatedTokenAddressSync(vaultLpMint, provider.wallet.publicKey);
    newManagerLpAta = getAssociatedTokenAddressSync(vaultLpMint, newManager);
    adminLpAta = getAssociatedTokenAddressSync(vaultLpMint, admin);

    await createAssociatedTokenAccountIdempotent(provider.connection, payerKp, vaultAssetMint, provider.wallet.publicKey);
    await mintTo(provider.connection, payerKp, vaultAssetMint, userAssetAta, provider.wallet.payer!, initialMintAmount.toNumber());
  });

  it("initialize protocol", async () => {
    const tx = await vaultProgram.methods
      .initOrUpdateProtocol(allOperationalState)
      .accounts({
        payer: payer,
        currentAdmin: admin,
        newAdmin: admin,
      })
      .signers([payerKp])
      .rpc();

    const protocolAccount = await vaultProgram.account.protocol.fetch(protocol);

    assert.equal(protocolAccount.operationalState, allOperationalState);
    assert.equal(protocolAccount.bump, protocolBump);
    assert.ok(protocolAccount.admin.equals(provider.wallet.publicKey));
  });

  it("Initialize Vault", async () => {
    const vaultParams = {
      config: vaultConfig,
      name: vaultName,
      description: vaultDescription,
    };

    const initializeVaultIx = await client.createInitializeVaultIx(vaultParams, {
      vault,
      vaultAssetMint,
      payer,
      admin: provider.wallet.publicKey,
      manager: provider.wallet.publicKey,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [initializeVaultIx],
      payerKp,
      [vaultKp]
    );

    // Fetch and verify the vault account
    const vaultAccount = await vaultProgram.account.vault.fetch(vault);

    // Check name and description
    assert.equal(
      Buffer.from(vaultAccount.name).toString().replace(/\0+$/, ""),
      vaultName,
      "Vault name should match"
    );
    assert.equal(
      Buffer.from(vaultAccount.description).toString().replace(/\0+$/, ""),
      vaultDescription,
      "Vault description should match"
    );

    // Check asset fields
    assert.ok(
      vaultAccount.asset.mint.equals(vaultAssetMint),
      "Asset mint should match"
    );
    assert.ok(
      vaultAccount.asset.idleAta.equals(vaultAssetIdleAta),
      "Idle ATA should match"
    );
    assert.equal(
      vaultAccount.asset.totalValue.toNumber(),
      0,
      "Initial total assets should be 0"
    );

    // Check LP mint
    assert.ok(vaultAccount.lp.mint.equals(vaultLpMint), "LP mint should match");

    // Check owner
    assert.ok(
      vaultAccount.admin.equals(provider.wallet.publicKey),
      "Admin should match"
    );
    assert.ok(
      vaultAccount.manager.equals(provider.wallet.publicKey),
      "Manager should match"
    );

    // Check configuration
    assert.equal(
      vaultAccount.feeConfiguration.managerPerformanceFee,
      vaultConfig.managerPerformanceFee,
      "Manager performance fee should match"
    );
    assert.equal(
      vaultAccount.feeConfiguration.adminPerformanceFee,
      vaultConfig.adminPerformanceFee,
      "Admin performance fee should match"
    );
    assert.equal(
      vaultAccount.feeConfiguration.managerManagementFee,
      vaultConfig.managerManagementFee,
      "Manager management fee should match"
    );
    assert.equal(
      vaultAccount.feeConfiguration.adminManagementFee,
      vaultConfig.adminManagementFee,
      "Admin management fee should match"
    );

    assert.equal(
      vaultAccount.vaultConfiguration.startAtTs.toString(),
      vaultConfig.startAtTs.toString(),
      "Start at timestamp should match"
    );

    assert.equal(
      vaultAccount.vaultConfiguration.maxCap.toString(),
      vaultConfig.maxCap.toString(),
      "Max cap should match"
    );

    assert.equal(
      vaultAccount.feeState.accumulatedLpAdminFees.toString(),
      "0",
      "Accumulated LP admin fees should be 0"
    );

    assert.equal(
      vaultAccount.feeState.accumulatedLpManagerFees.toString(),
      "0",
      "Accumulated LP manager fees should be 0"
    );

    assert.equal(
      vaultAccount.feeState.accumulatedLpProtocolFees.toString(),
      "0",
      "Accumulated LP protocol fees should be 0"
    );

    assert.notEqual(
      vaultAccount.feeUpdate.lastManagementFeeUpdateTs.toString(),
      "0",
      "Last management fee update timestamp should not be 0"
    );

    assert.equal(
      vaultAccount.feeUpdate.lastPerformanceFeeUpdateTs.toString(),
      "0",
      "Last performance fee update timestamp should be 0"
    );

    // Verify ATAs were created
    const idleAtaInfo = await provider.connection.getAccountInfo(
      vaultAssetIdleAta
    );
    assert.isNotNull(idleAtaInfo, "Idle ATA should exist");
  });

  it("Updates vault protocol fees", async () => {
    const performanceFeeBps = 500; // 5%
    const managementFeeBps = 200;  // 2%

    await vaultProgram.methods
      .updateVaultProtocolFee({ protocolPerformanceFee: {} }, performanceFeeBps)
      .accountsPartial({
        admin: admin,
        protocol: protocol,
        vault: vault,
      })
      .rpc();

    await vaultProgram.methods
      .updateVaultProtocolFee({ protocolManagementFee: {} }, managementFeeBps)
      .accountsPartial({
        admin: admin,
        protocol: protocol,
        vault: vault,
      })
      .rpc();

    const vaultAccount = await vaultProgram.account.vault.fetch(vault);
    assert.equal(vaultAccount.feeConfiguration.protocolPerformanceFee, performanceFeeBps);
    assert.equal(vaultAccount.feeConfiguration.protocolManagementFee, managementFeeBps);
  });


  it("create lp metadata", async () => {
    const payerLamportsBefore = await provider.connection.getBalance(payer);
    const [metadataAccount] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        metadataProgram.toBuffer(),
        vaultLpMint.toBuffer(),
      ],
      metadataProgram
    );

    const createLpMetadataIx = await client.createCreateLpMetadataIx({
      name: "Test LP",
      symbol: "TEST",
      uri: "https://test.com",
    }, {
      payer,
      admin,
      vault,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [createLpMetadataIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const payerLamportsAfter = await provider.connection.getBalance(payer);

    const metadataAccountInfo = await provider.connection.getAccountInfo(
      metadataAccount
    );
    assert.isNotNull(metadataAccountInfo, "Metadata account should be created");
    assert.ok(
      payerLamportsAfter < payerLamportsBefore,
      "Payer should have paid for the metadata account"
    );
  });

  it("update lp metadata", async () => {
    const [metadataAccount] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("metadata"),
        metadataProgram.toBuffer(),
        vaultLpMint.toBuffer(),
      ],
      metadataProgram
    );

    const metadataAccountInfoBefore = await provider.connection.getAccountInfo(
      metadataAccount
    );

    const updateLpMetadataIx = await client.createCreateLpMetadataIx({
      name: "Test LP Updated",
      symbol: "TEST",
      uri: "https://test.com",
    }, {
      payer: payer,
      admin: admin,
      vault: vault,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateLpMetadataIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const metadataAccountInfoAfter = await provider.connection.getAccountInfo(
      metadataAccount
    );

    assert.notEqual(
      metadataAccountInfoBefore?.data?.toString(),
      metadataAccountInfoAfter?.data?.toString(),
      "Metadata account should be updated"
    );
  });

  it("adds adaptor", async () => {
    const addAdaptorIx = await client.createAddAdaptorIx({
      vault: vault,
      payer,
      admin: provider.wallet.publicKey,
      adaptorProgram: adaptorProgram.programId,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [addAdaptorIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const [adaptorAddReceipt, adaptorAddReceiptBump] =
      PublicKey.findProgramAddressSync(
        [
          SEEDS.ADAPTOR_ADD_RECEIPT,
          vault.toBuffer(),
          adaptorProgram.programId.toBuffer(),
        ],
        vaultProgram.programId
      );

    const adaptorAddReceiptAccount =
      await vaultProgram.account.adaptorAddReceipt.fetch(adaptorAddReceipt);

    assert.equal(adaptorAddReceiptAccount.bump, adaptorAddReceiptBump);
    assert.equal(adaptorAddReceiptAccount.vault.toString(), vault.toString());
    assert.equal(
      adaptorAddReceiptAccount.adaptorProgram.toString(),
      adaptorProgram.programId.toString()
    );
  });

  it("Initial deposits assets successfully", async () => {
    // Create user LP ATA if it doesn't exist
    const createAssociatedTokenAccountIdempotentIx =
      createAssociatedTokenAccountIdempotentInstruction(
        payerKp.publicKey,
        userLpAta,
        provider.wallet.publicKey,
        vaultLpMint
      );

    // Get state before deposit
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeLpSupplyInclFees = new BN(beforeLpSupply.toString()).add(
      new BN(beforeVault.feeState.accumulatedLpProtocolFees.toString())
        .add(new BN(beforeVault.feeState.accumulatedLpAdminFees.toString()))
        .add(new BN(beforeVault.feeState.accumulatedLpManagerFees.toString()))
    );
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);
    const beforeUserAssetBalance = beforeUserAsset.amount;

    // Execute deposit
    const depositIx = await client.createDepositVaultIx(initialDepositAmount, {
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [createAssociatedTokenAccountIdempotentIx, depositIx],
      payerKp,
      [provider.wallet.payer!]
    );

    // Get state after deposit
    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterTotalAssets = afterVault.asset.totalValue;
    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterLpSupply = afterLpMint.supply;
    const afterLpSupplyInclFees = new BN(afterLpSupply.toString())
      .add(
        new BN(afterVault.feeState.accumulatedLpProtocolFees.toString())
          .add(new BN(afterVault.feeState.accumulatedLpAdminFees.toString()))
          .add(new BN(afterVault.feeState.accumulatedLpManagerFees.toString()))
      )
      .add(afterVault.deadWeight);
    const afterUserAsset = await getAccount(provider.connection, userAssetAta);
    const afterUserAssetBalance = afterUserAsset.amount;
    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;

    // Verify changes
    assert.equal(
      afterTotalAssets.toString(),
      beforeTotalAssets.add(initialDepositAmount).toString(),
      "Vault total assets should increase by deposit amount"
    );

    assert.equal(
      afterUserAssetBalance.toString(),
      (
        beforeUserAssetBalance - BigInt(initialDepositAmount.toString())
      ).toString(),
      "User asset balance should decrease by deposit amount"
    );

    // If first deposit, verify 1:1 mint
    if (beforeTotalAssets.eqn(0) && beforeLpSupplyInclFees.eqn(0)) {
      const expectedLpMinted = initialDepositAmount
        .mul(new BN(10 ** 9))
        .div(new BN(10 ** 6))
        .sub(DEAD_WEIGHT);

      assert.equal(
        afterLpSupplyInclFees.sub(DEAD_WEIGHT).toString(),
        expectedLpMinted.toString(),
        "First deposit should mint 1:1 LP tokens"
      );
      assert.equal(
        afterUserLpBalance.toString(),
        expectedLpMinted.toString(),
        "User should receive 1:1 LP tokens on first deposit"
      );
    } else {
      const totalAccumulatedFeesLp = afterVault.feeState.accumulatedLpAdminFees
        .add(afterVault.feeState.accumulatedLpManagerFees)
        .add(afterVault.feeState.accumulatedLpProtocolFees);
      const afterLpSupplyInclFees = new BN(afterLpSupply.toString()).add(
        totalAccumulatedFeesLp
      );
      const lpTokenMinted = afterLpSupply - beforeLpSupply;

      const expectedLpAmount = initialDepositAmount
        .mul(afterLpSupplyInclFees)
        .mul(new anchor.BN(10_000 - ISSUANCE_FEE_PERCENTAGE_BPS))
        .div(new anchor.BN(10_000).mul(new anchor.BN(afterTotalAssets)));

      assert.equal(
        lpTokenMinted.toString(),
        expectedLpAmount.toString(),
        "LP tokens minted should maintain ratio"
      );
    }
  });

  it("Subsequent deposits assets successfully", async () => {
    // Get state before deposit
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);
    const beforeUserAssetBalance = beforeUserAsset.amount;

    // Execute deposit
    const depositIx = await client.createDepositVaultIx(subsequentDepositAmount, {
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [depositIx],
      payerKp,
      [provider.wallet.payer!]
    );

    // Get state after deposit
    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterTotalAssets = afterVault.asset.totalValue;
    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterLpSupply = afterLpMint.supply;
    const afterUserAsset = await getAccount(provider.connection, userAssetAta);
    const afterUserAssetBalance = afterUserAsset.amount;
    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;
    const totalAccumulatedFeesLp = afterVault.feeState.accumulatedLpAdminFees
      .add(afterVault.feeState.accumulatedLpManagerFees)
      .add(afterVault.feeState.accumulatedLpProtocolFees);
    const afterLpSupplyInclFees = new BN(afterLpSupply.toString())
      .add(totalAccumulatedFeesLp)
      .add(afterVault.deadWeight);

    // Verify changes
    assert.equal(
      afterTotalAssets.toString(),
      beforeTotalAssets.add(subsequentDepositAmount).toString(),
      "Vault total assets should increase by deposit amount"
    );

    assert.equal(
      afterUserAssetBalance.toString(),
      (
        beforeUserAssetBalance - BigInt(subsequentDepositAmount.toString())
      ).toString(),
      "User asset balance should decrease by deposit amount"
    );

    // If first deposit, verify 1:1 mint
    if (beforeTotalAssets.eqn(0) && beforeLpSupply === BigInt(0)) {
      const expectedLpMinted = subsequentDepositAmount
        .mul(new BN(10 ** 9))
        .div(new BN(10 ** 6))
        .sub(DEAD_WEIGHT);

      assert.equal(
        afterLpSupplyInclFees.sub(DEAD_WEIGHT).toString(),
        expectedLpMinted.toString(),
        "First deposit should mint 1:1 LP tokens"
      );
      assert.equal(
        afterUserLpBalance.toString(),
        subsequentDepositAmount
          .mul(new BN(10 ** 9))
          .div(new BN(10 ** 6))
          .toString(),
        "User should receive 1:1 LP tokens on first deposit"
      );
    } else {
      const lpTokenMinted = afterLpSupply - beforeLpSupply;

      const expectedLpAmount = subsequentDepositAmount
        .mul(afterLpSupplyInclFees)
        .mul(new anchor.BN(10_000 - ISSUANCE_FEE_PERCENTAGE_BPS))
        .div(new anchor.BN(10_000).mul(new anchor.BN(afterTotalAssets)));

      assert.equal(
        lpTokenMinted.toString(),
        expectedLpAmount.toString(),
        "LP tokens minted should maintain ratio"
      );
    }
  });

  it("Requests withdraw vault successfully", async () => {
    const beforeUserLp = await getAccount(provider.connection, userLpAta);
    const beforeUserLpBalance = beforeUserLp.amount;
    // Create user LP ATA if it doesn't exist
    const createRequestWithdrawLpAtaIx =
      createAssociatedTokenAccountIdempotentInstruction(
        provider.wallet.publicKey,
        requestWithdrawLpAta,
        requestWithdrawVaultReceipt,
        vaultLpMint
      );

    const requestWithdrawIx = await client.createRequestWithdrawVaultIx(
      { amount: withdrawLpAmount, isAmountInLp: true, isWithdrawAll: false },
      {
        payer,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [createRequestWithdrawLpAtaIx, requestWithdrawIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;
    const afterRequestWithdrawLp = await getAccount(
      provider.connection,
      requestWithdrawLpAta
    );
    const afterRequestWithdrawLpAtaBalance = afterRequestWithdrawLp.amount;

    const actualLpDelta = beforeUserLpBalance - afterUserLpBalance;

    assert.equal(
      actualLpDelta.toString(),
      afterRequestWithdrawLpAtaBalance.toString(),
      "LP should be transferred to the request withdraw LP ATA"
    );

    assert.equal(
      afterUserLpBalance.toString(),
      new BN(beforeUserLpBalance.toString()).sub(withdrawLpAmount).toString(),
      "User LP balance should be reduced by the withdraw amount"
    );

    const afterRequestWithdrawVaultReceipt =
      await vaultProgram.account.requestWithdrawVaultReceipt.fetch(
        requestWithdrawVaultReceipt
      );

    assert.equal(
      afterRequestWithdrawVaultReceipt.amountLpEscrowed.toString(),
      actualLpDelta.toString(),
      "LP to burn should be equal to the actual LP delta"
    );

    assert.equal(
      afterRequestWithdrawVaultReceipt.vault.toString(),
      vault.toString(),
      "Vault should be the same"
    );

    assert.equal(
      afterRequestWithdrawVaultReceipt.user.toString(),
      provider.wallet.publicKey.toString(),
      "User should be the same"
    );

    assert.ok(
      afterRequestWithdrawVaultReceipt.bump === requestWithdrawVaultReceiptBump,
      "Bump should be the same"
    );
  });

  it("Should not be able to withdraw vault before waiting period", async () => {
    try {
      const withdrawIx = await client.createWithdrawVaultIx({
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
        vaultAssetMint: vaultAssetMint,
        assetTokenProgram: TOKEN_PROGRAM_ID,
      });

      await sendAndConfirmVersionedTransaction(
        provider.connection,
        [withdrawIx],
        payerKp,
        [provider.wallet.payer!]
      );

      assert.fail("Should not be able to withdraw before waiting period");
    } catch (error: any) {
      assert.include(error.message, "WithdrawalNotYetAvailable");
    }
  });

  it("Cancels withdraw request successfully", async () => {
    const beforeUserLp = await getAccount(provider.connection, userLpAta);
    const beforeUserLpBalance = beforeUserLp.amount;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = convertNumberToDecimal(beforeLpMint.supply);
    const beforeRequestWithdrawVaultReceipt =
      await vaultProgram.account.requestWithdrawVaultReceipt.fetch(
        requestWithdrawVaultReceipt
      );
    const beforeRequestWithdrawVaultReceiptAmountLpEscrowed =
      beforeRequestWithdrawVaultReceipt.amountLpEscrowed;

    // Fetch vault data to calculate expected refund
    const vaultData = await vaultProgram.account.vault.fetch(vault);
    const accumulatedLpFees = convertNumberToDecimal(
      vaultData.feeState.accumulatedLpAdminFees
    )
      .add(vaultData.feeState.accumulatedLpManagerFees.toNumber())
      .add(vaultData.feeState.accumulatedLpProtocolFees.toNumber());
    const beforeLpSupplyInclFees =
      convertNumberToDecimal(beforeLpSupply).add(accumulatedLpFees);
    const totalAsset = convertNumberToDecimal(vaultData.asset.totalValue);
    // Get initial asset worth from the receipt
    const initialAmountAssetWorth = convertDecimalBitsToDecimal(
      beforeRequestWithdrawVaultReceipt.amountAssetToWithdrawDecimalBits
    );
    const initialAmountLpToRefund = convertNumberToDecimal(
      beforeRequestWithdrawVaultReceipt.amountLpEscrowed
    );
    // (tlp - ilp)
    const tlpSubIlp = beforeLpSupplyInclFees.sub(initialAmountLpToRefund);

    // ta * 10000
    const taMulConstBps = totalAsset.mul(MAX_FEE_BPS);

    // (10000 - r)
    const constBpsSubFeeBps = convertNumberToDecimal(
      MAX_FEE_BPS - REDEMPTION_FEE_PERCENTAGE_BPS
    );

    // ia * (10000 - r)
    const iaMulConstBpsSubFeeBps =
      initialAmountAssetWorth.mul(constBpsSubFeeBps);

    // (ta * 10000 - ia * (10000 - r))
    const denominator = taMulConstBps.sub(iaMulConstBpsSubFeeBps);
    // clp = ia * (tlp - ilp) * (10000 - r) / (ta * 10000 - ia * (10000 - r))
    const currentAmountLpToRefund = initialAmountAssetWorth
      .mul(tlpSubIlp)
      .mul(constBpsSubFeeBps)
      .div(denominator)
      .floor();

    // Only reduce LP to refund, never increase
    const expectedLpToRefund = currentAmountLpToRefund.lt(
      initialAmountLpToRefund
    )
      ? currentAmountLpToRefund
      : initialAmountLpToRefund;

    const expectedLpToBurn = initialAmountLpToRefund.sub(expectedLpToRefund);

    const cancelIx = await client.createCancelRequestWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [cancelIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;
    const refundedLpToUser = afterUserLpBalance - beforeUserLpBalance;
    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterLpSupply = convertNumberToDecimal(afterLpMint.supply);
    const lpBurnt = beforeLpSupply.sub(afterLpSupply);

    assert.equal(
      refundedLpToUser.toString(),
      expectedLpToRefund.toString(),
      "LP refunded to user should match calculated amount"
    );

    assert.equal(
      lpBurnt.toString(),
      expectedLpToBurn.toString(),
      "LP burnt should match calculated amount"
    );

    const afterRequestWithdrawVaultReceipt =
      await provider.connection.getAccountInfo(requestWithdrawVaultReceipt);
    assert.isNull(
      afterRequestWithdrawVaultReceipt,
      "Request withdraw vault receipt should be closed"
    );
  });

  it("Update vault config - MaxCap", async () => {
    const newMaxCap = new BN(20_000_000_000_000);
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const oldMaxCap = beforeVault.vaultConfiguration.maxCap;

    // Serialize the new value
    const data = newMaxCap.toArrayLike(Buffer, "le", 8);

    const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.MaxCap, data, {
      vault,
      admin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.vaultConfiguration.maxCap.toString(),
      newMaxCap.toString(),
      "Max cap should be updated"
    );

    assert.notEqual(
      oldMaxCap.toString(),
      newMaxCap.toString(),
      "Max cap should have changed"
    );
  });

  it("Update vault config - WithdrawalWaitingPeriod", async () => {
    const newWaitingPeriod = new BN(0);
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const oldWaitingPeriod =
      beforeVault.vaultConfiguration.withdrawalWaitingPeriod;

    const data = newWaitingPeriod.toArrayLike(Buffer, "le", 8);

    const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.WithdrawalWaitingPeriod, data, {
      vault,
      admin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.vaultConfiguration.withdrawalWaitingPeriod.toString(),
      newWaitingPeriod.toString(),
      "Withdrawal waiting period should be updated"
    );

    assert.notEqual(
      oldWaitingPeriod.toString(),
      newWaitingPeriod.toString(),
      "Withdrawal waiting period should have changed"
    );
  });

  it("Update vault config - ManagerManagementFee (should charge management fee)", async () => {
    const newManagerManagementFee = 1000; // 10%
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const oldFee = beforeVault.feeConfiguration.managerManagementFee;
    const beforeAccumulatedManagerFees =
      beforeVault.feeState.accumulatedLpManagerFees;

    const data = Buffer.alloc(2);
    data.writeUInt16LE(newManagerManagementFee, 0);

    const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.ManagerManagementFee, data, {
      vault,
      admin,
      vaultLpMint,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.feeConfiguration.managerManagementFee,
      newManagerManagementFee,
      "Manager management fee should be updated"
    );

    assert.notEqual(
      oldFee,
      newManagerManagementFee,
      "Manager management fee should have changed"
    );

    // Verify that management fees were charged (if time elapsed)
    if (
      afterVault.feeUpdate.lastManagementFeeUpdateTs.gt(
        beforeVault.feeUpdate.lastManagementFeeUpdateTs
      )
    ) {
      assert.isTrue(
        afterVault.feeState.accumulatedLpManagerFees.gte(
          beforeAccumulatedManagerFees
        ),
        "Management fees should have been charged"
      );
    }

    // Verify timestamp was updated
    assert.isTrue(
      afterVault.feeUpdate.lastManagementFeeUpdateTs.gt(new BN(0)),
      "Last management fee update timestamp should be set"
    );
  });

  it("Update vault config - IssuanceFee", async () => {
    const newIssuanceFee = 75; // 0.75%
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const oldFee = beforeVault.feeConfiguration.issuanceFee;

    const data = Buffer.alloc(2);
    data.writeUInt16LE(newIssuanceFee, 0);

    const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.IssuanceFee, data, {
      vault,
      admin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.feeConfiguration.issuanceFee,
      newIssuanceFee,
      "Issuance fee should be updated"
    );

    assert.notEqual(oldFee, newIssuanceFee, "Issuance fee should have changed");
  });

  it("Update vault config - Manager", async () => {
    const data = newManager.toBuffer();

    const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.Manager, data, {
      vault,
      admin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.manager.toString(),
      newManager.toString(),
      "Manager should be updated"
    );
  });

  it("Update vault config - DisabledOperations", async () => {
    const newDisabledOperations = DisabledOperations.WITHDRAW_VAULT;

    const data = new BN(newDisabledOperations).toArrayLike(Buffer, "le", 2);

    const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.DisabledOperations, data, {
      vault,
      admin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.vaultConfiguration.disabledOperations.toString(),
      newDisabledOperations.toString(),
      "Disabled operations should be updated"
    );
  });

  it("Should fail to update vault config with invalid performance fee (exceeds max)", async () => {
    const invalidFee = 15000; // 150% - exceeds MAX_FEE_BPS
    const data = Buffer.alloc(2);
    data.writeUInt16LE(invalidFee, 0);

    try {
      const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.ManagerPerformanceFee, data, {
        vault,
        admin,
      });

      await sendAndConfirmVersionedTransaction(
        provider.connection,
        [updateIx],
        provider.wallet.payer as any,
        []
      );
      assert.fail("Should not allow performance fee exceeding MAX_FEE_BPS");
    } catch (error: any) {
      assert.include(error.message, "InvalidFeeConfiguration");
    }
  });

  it("Should fail when non-admin tries to update vault config", async () => {
    const unauthorizedUser = Keypair.generate();
    const newMaxCap = new BN(15_000_000_000_000);
    const data = newMaxCap.toArrayLike(Buffer, "le", 8);

    try {
      const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.MaxCap, data, {
        vault,
        admin: unauthorizedUser.publicKey,
      });

      await sendAndConfirmVersionedTransaction(
        provider.connection,
        [updateIx],
        unauthorizedUser,
        []
      );
      assert.fail("Should not allow non-admin to update vault config");
    } catch (error) {
      // Should fail with constraint violation
      assert.isTrue(true);
    }
  });

  it("Should fail when disabled operation is not allowed", async () => {
    try {
      const requestWithdrawIx = await client.createRequestWithdrawVaultIx(
        { amount: withdrawLpAmount, isAmountInLp: true, isWithdrawAll: false },
        {
          payer: provider.wallet.publicKey,
          userTransferAuthority: provider.wallet.publicKey,
          vault: vault,
        }
      );

      await sendAndConfirmVersionedTransaction(
        provider.connection,
        [requestWithdrawIx],
        provider.wallet.payer as any,
        []
      );
      assert.fail("Should not allow disabled operation");
    } catch (error: any) {
      assert.include(error.message, "OperationNotAllowed.");
    }
  });

  it("Withdraws assets in LP successfully", async () => {
    const enableWithdrawIx = await client.createUpdateVaultConfigIx(
      VaultConfigField.DisabledOperations,
      new BN(0).toArrayLike(Buffer, "le", 2),
      { vault, admin }
    );

    // Get state before withdrawal
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeUserLp = await getAccount(provider.connection, userLpAta);
    const beforeUserLpBalance = beforeUserLp.amount;
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);
    const beforeUserAssetBalance = beforeUserAsset.amount;

    const requestWithdrawIx = await client.createRequestWithdrawVaultIx(
      { amount: withdrawLpAmount, isAmountInLp: true, isWithdrawAll: false },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [enableWithdrawIx, requestWithdrawIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const requestWithdrawReceipt =
      await vaultProgram.account.requestWithdrawVaultReceipt.fetch(
        requestWithdrawVaultReceipt
      );

    const amountLpEscrowed = requestWithdrawReceipt.amountLpEscrowed;
    const originalAmountAssetToWithdraw = new BN(
      convertDecimalBitsToDecimal(
        requestWithdrawReceipt.amountAssetToWithdrawDecimalBits
      )
        .floor()
        .toString()
    );

    // Execute withdrawal
    const withdrawIx = await client.createWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawIx],
      payerKp,
      [provider.wallet.payer!]
    );

    // Get state after withdrawal
    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterTotalAssets = afterVault.asset.totalValue;
    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterLpSupply = afterLpMint.supply;
    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;
    const afterUserAsset = await getAccount(provider.connection, userAssetAta);
    const afterUserAssetBalance = afterUserAsset.amount;

    const afterAccumulatedLpFees = afterVault.feeState.accumulatedLpAdminFees
      .add(afterVault.feeState.accumulatedLpManagerFees)
      .add(afterVault.feeState.accumulatedLpProtocolFees);

    // Calculate expected asset return
    const currentAmountAssetToWithdraw = new anchor.BN(
      amountLpEscrowed.toString()
    )
      .mul(new anchor.BN(beforeTotalAssets.toString()))
      .mul(new anchor.BN(MAX_FEE_BPS - REDEMPTION_FEE_PERCENTAGE_BPS))
      .div(new anchor.BN(beforeLpSupply.toString()).add(afterAccumulatedLpFees))
      .div(new anchor.BN(MAX_FEE_BPS));

    const expectedAmountAssetToWithdraw = currentAmountAssetToWithdraw.gt(
      originalAmountAssetToWithdraw
    )
      ? originalAmountAssetToWithdraw
      : currentAmountAssetToWithdraw;

    const actualWithdrawnAssets = beforeTotalAssets.sub(afterTotalAssets);
    // Verify total assets decreased by withdrawn amount
    assert.equal(
      actualWithdrawnAssets.toString(),
      expectedAmountAssetToWithdraw.toString(),
      "Vault total assets should decrease by withdrawn amount"
    );

    // Verify LP tokens were burned
    assert.equal(
      (beforeLpSupply - afterLpSupply).toString(),
      withdrawLpAmount.toString(),
      "LP tokens should be burned proportionally"
    );

    // Verify user's LP balance decreased
    assert.equal(
      (beforeUserLpBalance - afterUserLpBalance).toString(),
      withdrawLpAmount.toString(),
      "User's LP token balance should decrease by withdrawal amount"
    );

    // Verify user received correct amount of assets
    assert.equal(
      (afterUserAssetBalance - beforeUserAssetBalance).toString(),
      actualWithdrawnAssets.toString(),
      "User should receive correct amount of assets"
    );

    // Additional sanity checks
    assert.isTrue(
      afterTotalAssets.lt(beforeTotalAssets),
      "Total assets should decrease"
    );
    assert.isTrue(afterLpSupply < beforeLpSupply, "LP supply should decrease");
  });

  it("Withdraws assets in asset successfully", async () => {
    // Get state before withdrawal
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeUserLp = await getAccount(provider.connection, userLpAta);
    const beforeUserLpBalance = beforeUserLp.amount;
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);
    const beforeUserAssetBalance = beforeUserAsset.amount;
    const totalAccumulatedFeesLp = beforeVault.feeState.accumulatedLpAdminFees
      .add(beforeVault.feeState.accumulatedLpManagerFees)
      .add(beforeVault.feeState.accumulatedLpProtocolFees);
    const beforeLpSupplyInclFees = new BN(beforeLpSupply.toString())
      .add(totalAccumulatedFeesLp)
      .add(beforeVault.deadWeight);

    const requestWithdrawIx = await client.createRequestWithdrawVaultIx(
      { amount: withdrawAssetAmount, isAmountInLp: false, isWithdrawAll: false },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [requestWithdrawIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const requestWithdrawReceipt =
      await vaultProgram.account.requestWithdrawVaultReceipt.fetch(
        requestWithdrawVaultReceipt
      );

    const originalAmountAssetToWithdraw = new BN(
      convertDecimalBitsToDecimal(
        requestWithdrawReceipt.amountAssetToWithdrawDecimalBits
      )
        .floor()
        .toString()
    );
    const amountLpEscrowed = requestWithdrawReceipt.amountLpEscrowed;

    // Execute withdrawal
    const withdrawIx = await client.createWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawIx],
      payerKp,
      [provider.wallet.payer!]
    );

    // Get state after withdrawal
    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterTotalAssets = afterVault.asset.totalValue;
    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterLpSupply = afterLpMint.supply;
    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;
    const afterUserAsset = await getAccount(provider.connection, userAssetAta);
    const afterUserAssetBalance = afterUserAsset.amount;
    const afterAccumulatedLpFees = afterVault.feeState.accumulatedLpAdminFees
      .add(afterVault.feeState.accumulatedLpManagerFees)
      .add(afterVault.feeState.accumulatedLpProtocolFees)
      .add(afterVault.deadWeight);

    const currentAmountAssetToWithdraw = new anchor.BN(
      amountLpEscrowed.toString()
    )
      .mul(new anchor.BN(beforeTotalAssets.toString()))
      .mul(new anchor.BN(MAX_FEE_BPS - REDEMPTION_FEE_PERCENTAGE_BPS))
      .div(
        new anchor.BN(beforeLpSupply.toString())
          .add(afterAccumulatedLpFees)
          .mul(new anchor.BN(MAX_FEE_BPS))
      );

    const expectedAmountAssetToWithdraw = currentAmountAssetToWithdraw.gt(
      originalAmountAssetToWithdraw
    )
      ? originalAmountAssetToWithdraw
      : currentAmountAssetToWithdraw;

    // Calculate expected asset return
    const expectedBurnedLpAmountNumerator = new anchor.BN(
      withdrawAssetAmount.toString()
    )
      .mul(new anchor.BN(beforeLpSupplyInclFees.toString()))
      .mul(new anchor.BN(MAX_FEE_BPS));

    const expectedBurnedLpAmountDenominator = new anchor.BN(
      beforeTotalAssets.toString()
    ).mul(new anchor.BN(MAX_FEE_BPS - REDEMPTION_FEE_PERCENTAGE_BPS));

    let expectedBurnedLpAmount = expectedBurnedLpAmountNumerator.div(
      expectedBurnedLpAmountDenominator
    );

    if (
      expectedBurnedLpAmountNumerator
        .mod(expectedBurnedLpAmountDenominator)
        .gt(new BN(0))
    ) {
      expectedBurnedLpAmount = expectedBurnedLpAmount.add(new BN(1));
    }

    const actualWithdrawnAsset = afterUserAssetBalance - beforeUserAssetBalance;
    const vaultAssetDelta = beforeTotalAssets.sub(afterTotalAssets);

    // Verify total assets decreased by withdrawn amount
    assert.equal(
      vaultAssetDelta.toString(),
      actualWithdrawnAsset.toString(),
      "Vault total assets should decrease by withdrawn amount"
    );

    // Verify user received correct amount of assets
    assert.equal(
      actualWithdrawnAsset.toString(),
      expectedAmountAssetToWithdraw.toString(),
      "User should receive correct amount of assets"
    );

    // Verify LP tokens were burned
    assert.equal(
      (beforeLpSupply - afterLpSupply).toString(),
      expectedBurnedLpAmount.toString(),
      "LP tokens should be burned proportionally"
    );

    // Verify user's LP balance decreased
    assert.equal(
      (beforeUserLpBalance - afterUserLpBalance).toString(),
      expectedBurnedLpAmount.toString(),
      "User's LP token balance should decrease by withdrawal amount"
    );

    // Additional sanity checks
    assert.isTrue(
      afterTotalAssets.lt(beforeTotalAssets),
      "Total assets should decrease"
    );
    assert.isTrue(afterLpSupply < beforeLpSupply, "LP supply should decrease");
  });

  it("Initializes strategy", async () => {
    const initStrategyIx = await client.createInitializeStrategyIx(
      { instructionDiscriminator: null, additionalArgs: null },
      {
        payer: provider.wallet.publicKey,
        manager: provider.wallet.publicKey,
        vault,
        strategy,
        adaptorProgram: adaptorProgram.programId,
        remainingAccounts: []
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [initStrategyIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const strategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    assert.equal(strategyInitReceiptAccount.positionValue.toString(), "0");
    assert.equal(strategyInitReceiptAccount.vault.toString(), vault.toString());
    assert.equal(
      strategyInitReceiptAccount.strategy.toString(),
      strategy.toString()
    );
    assert.equal(
      strategyInitReceiptAccount.adaptorProgram.toString(),
      adaptorProgram.programId.toString()
    );
  });

  it("Deposits assets to the strategy successfully", async () => {
    const createAtaIx = createAssociatedTokenAccountIdempotentInstruction(
      provider.wallet.publicKey,
      vaultStrategyAssetAta,
      vaultStrategyAuth,
      vaultAssetMint
    );

    const createStrategyAtaIx =
      createAssociatedTokenAccountIdempotentInstruction(
        provider.wallet.publicKey,
        strategyAta,
        strategy,
        vaultAssetMint
      );

    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;

    const beforeStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    const beforeIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );

    const depositStrategyIx = await client.createDepositStrategyIx(
      { depositAmount: depositStrategyAmount, instructionDiscriminator: null, additionalArgs: null },
      {
        manager: provider.wallet.publicKey,
        vault,
        vaultAssetMint: vaultAssetMint,
        adaptorProgram: adaptorProgram.programId,
        strategy,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: strategyAta, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [createAtaIx, createStrategyAtaIx, depositStrategyIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );

    const actualDepositAmount = beforeIdleAta.amount - afterIdleAta.amount;

    const afterTotalAssets = afterVault.asset.totalValue;

    const afterStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    assert.ok(
      actualDepositAmount.toString() === depositStrategyAmount.toString()
    );

    assert.ok(
      afterTotalAssets.toNumber() - beforeTotalAssets.toNumber() ===
      afterStrategyInitReceiptAccount.positionValue.toNumber() -
      (beforeStrategyInitReceiptAccount.positionValue.toNumber() +
        Number(actualDepositAmount))
    );
  });

  it("Refreshes the strategy value and account fees", async () => {
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;

    const beforeStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    const beforeIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );

    const beforeLpSupply = await getMint(provider.connection, vaultLpMint).then(
      (mint) => mint.supply
    );

    const updatedStrategyValue = new BN(depositStrategyAmount.mul(new BN(2)));

    const refreshStrategyIx = await client.createDepositStrategyIx(
      {
        depositAmount: new BN(0),
        instructionDiscriminator: null,
        additionalArgs: updatedStrategyValue.toArrayLike(Buffer, "le", 8)
      },
      {
        manager: provider.wallet.publicKey,
        vault,
        vaultAssetMint: vaultAssetMint,
        adaptorProgram: adaptorProgram.programId,
        strategy,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: strategyAta, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [refreshStrategyIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterTotalAssets = afterVault.asset.totalValue;

    const afterIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );

    const actualDepositAmount = beforeIdleAta.amount - afterIdleAta.amount;

    const afterStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    const strategyPnl = afterStrategyInitReceiptAccount.positionValue.sub(
      beforeStrategyInitReceiptAccount.positionValue
    );
    const vaultPnl = afterTotalAssets.sub(beforeTotalAssets);
    const actualPnl = updatedStrategyValue.sub(depositStrategyAmount);

    assert.ok(actualDepositAmount.toString() === "0");
    assert.ok(
      afterStrategyInitReceiptAccount.positionValue.toString() ===
      updatedStrategyValue.toString()
    );
    assert.ok(strategyPnl.toString() === actualPnl.toString());
    assert.ok(strategyPnl.toString() === vaultPnl.toString());

    const totalAccumulatedFeesBefore =
      beforeVault.feeState.accumulatedLpAdminFees
        .add(beforeVault.feeState.accumulatedLpManagerFees)
        .add(beforeVault.feeState.accumulatedLpProtocolFees);
    const totalAccumulatedFeesAfter = afterVault.feeState.accumulatedLpAdminFees
      .add(afterVault.feeState.accumulatedLpManagerFees)
      .add(afterVault.feeState.accumulatedLpProtocolFees);
    const totalAccumulatedFeesDelta = totalAccumulatedFeesAfter.sub(
      totalAccumulatedFeesBefore
    );

    const totalPerformanceFeesBps =
      beforeVault.feeConfiguration.managerPerformanceFee +
      beforeVault.feeConfiguration.adminPerformanceFee +
      beforeVault.feeConfiguration.protocolPerformanceFee;

    const highWaterMark = convertDecimalBitsToDecimal(
      beforeVault.highWaterMark.highestAssetPerLpDecimalBits
    );

    const beforeLpSupplyInclFees = new BN(beforeLpSupply.toString()).add(
      totalAccumulatedFeesBefore
    );

    const currAssetPerOriginalLpDecimal = fixedPointDiv(
      afterTotalAssets.toString(),
      beforeLpSupplyInclFees.toString()
    );

    const eligibleProfitPerLpDecimal =
      currAssetPerOriginalLpDecimal.sub(highWaterMark);

    const eligibleProfit = eligibleProfitPerLpDecimal.mul(
      beforeLpSupplyInclFees.toString()
    );

    const expectedFeeAsset = eligibleProfit
      .mul(new Decimal(totalPerformanceFeesBps))
      .div(new Decimal(MAX_FEE_BPS))
      .ceil();

    const afterLpSupply = await getMint(provider.connection, vaultLpMint).then(
      (mint) => mint.supply
    );

    const actualFeeAsset = totalAccumulatedFeesDelta
      .mul(new BN(afterTotalAssets.toString()))
      .div(new BN(afterLpSupply.toString()).add(totalAccumulatedFeesAfter));

    assert.equal(
      actualFeeAsset.toString(),
      expectedFeeAsset.toString(),
      "Fee asset should be correct"
    );
  });

  it("Calibrates high water mark successfully after a loss", async () => {
    const beforeLossVault = await vaultProgram.account.vault.fetch(vault);
    const highWaterMarkBeforeLoss = convertDecimalBitsToDecimal(
      beforeLossVault.highWaterMark.highestAssetPerLpDecimalBits
    );

    const beforeLossStrategyReceipt =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);
    const positionValueBeforeLoss = beforeLossStrategyReceipt.positionValue;

    const lossAmount = new BN(depositStrategyAmount.div(new BN(2)));
    const updatedStrategyValue = positionValueBeforeLoss.sub(lossAmount);

    const updateLossIx = await client.createDepositStrategyIx(
      {
        depositAmount: new BN(0),
        instructionDiscriminator: null,
        additionalArgs: updatedStrategyValue.toArrayLike(Buffer, "le", 8)
      },
      {
        manager: provider.wallet.publicKey,
        vault,
        vaultAssetMint: vaultAssetMint,
        adaptorProgram: adaptorProgram.programId,
        strategy,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: strategyAta, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateLossIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterLossVault = await vaultProgram.account.vault.fetch(vault);
    const highWaterMarkAfterLoss = convertDecimalBitsToDecimal(
      afterLossVault.highWaterMark.highestAssetPerLpDecimalBits
    );

    assert.equal(
      highWaterMarkAfterLoss.toString(),
      highWaterMarkBeforeLoss.toString(),
      "High water mark should not decrease after a loss"
    );

    const afterLossTotalAssets = new Decimal(
      afterLossVault.asset.totalValue.toString()
    );

    const totalAccumulatedFees = afterLossVault.feeState.accumulatedLpAdminFees
      .add(afterLossVault.feeState.accumulatedLpManagerFees)
      .add(afterLossVault.feeState.accumulatedLpProtocolFees);

    const performanceFeesBps =
      afterLossVault.feeConfiguration.managerPerformanceFee +
      afterLossVault.feeConfiguration.adminPerformanceFee +
      afterLossVault.feeConfiguration.protocolPerformanceFee;

    const lpMintBeforeCalibrate = await getMint(
      provider.connection,
      vaultLpMint
    );

    const beforeAdminLpAtaAmt = await getAccount(
      provider.connection,
      adminLpAta
    ).then((account) => account.amount);

    const totalLpSupplyInclFeesBeforeCalibrate = new BN(
      lpMintBeforeCalibrate.supply.toString()
    )
      .add(totalAccumulatedFees)
      .add(afterLossVault.deadWeight);

    const currentAssetPerLpDecimal = fixedPointDiv(
      afterLossTotalAssets.toString(),
      totalLpSupplyInclFeesBeforeCalibrate.toString()
    );

    const eligibleProfit = highWaterMarkBeforeLoss
      .sub(currentAssetPerLpDecimal)
      .mul(totalLpSupplyInclFeesBeforeCalibrate.toString())
      .ceil();

    const assetToClawBack = eligibleProfit
      .mul(performanceFeesBps)
      .div(MAX_FEE_BPS)
      .ceil();

    const calibrateIx = await client.createCalibrateHighWaterMarkIx({
      vault,
      admin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [calibrateIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterAdminLpAtaAmt = await getAccount(
      provider.connection,
      adminLpAta
    ).then((account) => account.amount);

    const lpBurned = beforeAdminLpAtaAmt - afterAdminLpAtaAmt;
    const expectedLpBurned = assetToClawBack
      .mul(totalLpSupplyInclFeesBeforeCalibrate.toString())
      .div(afterLossTotalAssets.toString())
      .ceil();

    assert.equal(
      lpBurned.toString(),
      expectedLpBurned.toString(),
      "LP burned should be correct"
    );

    const afterCalibrateVault = await vaultProgram.account.vault.fetch(vault);
    const highWaterMarkAfterCalibrate = convertDecimalBitsToDecimal(
      afterCalibrateVault.highWaterMark.highestAssetPerLpDecimalBits
    );

    const lpMintAfterCalibrate = await getMint(
      provider.connection,
      vaultLpMint
    );

    const totalLpSupplyInclFeesAfterCalibrate = new BN(
      lpMintAfterCalibrate.supply.toString()
    )
      .add(totalAccumulatedFees)
      .add(afterCalibrateVault.deadWeight);

    const expectedNewHwm = fixedPointDiv(
      afterCalibrateVault.asset.totalValue.toString(),
      totalLpSupplyInclFeesAfterCalibrate.toString()
    );

    const tolerance = new Decimal("1e-9");
    assert.isTrue(
      highWaterMarkAfterCalibrate.sub(expectedNewHwm).abs().lte(tolerance),
      `High water mark should be calibrated to the current asset/lp ratio. Expected: ${expectedNewHwm.toString()}, Actual: ${highWaterMarkAfterCalibrate.toString()}`
    );

    assert.isTrue(
      highWaterMarkAfterCalibrate.lt(highWaterMarkBeforeLoss),
      "Calibrated high water mark should be lower than the previous high"
    );
  });

  it("Withdaws from the strategy", async () => {
    const beforeIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );

    const withdrawStrategyIx = await client.createWithdrawStrategyIx(
      {
        withdrawAmount: depositStrategyAmount,
        instructionDiscriminator: null,
        additionalArgs: null
      },
      {
        manager: provider.wallet.publicKey,
        vault,
        vaultAssetMint: vaultAssetMint,
        adaptorProgram: adaptorProgram.programId,
        strategy,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: strategyAta, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawStrategyIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );
    const afterStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    assert.ok(
      (afterIdleAta.amount - beforeIdleAta.amount).toString() ===
      depositStrategyAmount.toString()
    );

    assert.equal(
      afterStrategyInitReceiptAccount.positionValue.toString(),
      "0",
      "Position value should be 0"
    );
  });

  it("Withdraws all assets from the vault successfully", async () => {
    // Get state before withdrawal
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeUserLp = await getAccount(provider.connection, userLpAta);
    const beforeUserLpBalance = beforeUserLp.amount;
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);
    const beforeUserAssetBalance = beforeUserAsset.amount;

    const requestWithdrawIx = await client.createRequestWithdrawVaultIx(
      { amount: new BN(Number.MAX_SAFE_INTEGER), isAmountInLp: false, isWithdrawAll: true },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [requestWithdrawIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const requestWithdrawReceipt =
      await vaultProgram.account.requestWithdrawVaultReceipt.fetch(
        requestWithdrawVaultReceipt
      );

    const originalAmountAssetToWithdraw = new BN(
      convertDecimalBitsToDecimal(
        requestWithdrawReceipt.amountAssetToWithdrawDecimalBits
      )
        .floor()
        .toString()
    );

    // Execute withdrawal
    const withdrawIx = await client.createWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawIx],
      payerKp,
      [provider.wallet.payer!]
    );

    // Get state after withdrawal
    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterTotalAssets = afterVault.asset.totalValue;
    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterLpSupply = afterLpMint.supply;
    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;
    const afterUserAsset = await getAccount(provider.connection, userAssetAta);
    const afterUserAssetBalance = afterUserAsset.amount;

    const timeElapsed = afterVault.feeUpdate.lastManagementFeeUpdateTs.sub(
      beforeVault.feeUpdate.lastManagementFeeUpdateTs
    );

    const beforeTotalManagementFeesBps = beforeVault.feeConfiguration.managerManagementFee
      + beforeVault.feeConfiguration.protocolManagementFee
      + beforeVault.feeConfiguration.adminManagementFee;

    const feeNumerator = timeElapsed
      .mul(beforeVault.asset.totalValue)
      .mul(new BN(beforeTotalManagementFeesBps));
    const feeDenominator = new BN(10_000).mul(new BN(365 * 24 * 60 * 60));

    const expectedFeesInAsset = feeNumerator
      .add(feeDenominator)
      .subn(1)
      .div(feeDenominator);

    const beforeTotalLpSupply = new BN(beforeLpSupply.toString())
      .add(
        beforeVault.feeState.accumulatedLpAdminFees
          .add(beforeVault.feeState.accumulatedLpManagerFees)
          .add(beforeVault.feeState.accumulatedLpProtocolFees)
      )
      .add(beforeVault.deadWeight);

    const expectedFeesInLp = new Decimal(expectedFeesInAsset.toString())
      .mul(beforeTotalLpSupply.toString())
      .div(
        new Decimal(beforeVault.asset.totalValue.toString()).sub(
          expectedFeesInAsset.toString()
        )
      )
      .ceil();

    const actualFeesInLp = afterVault.feeState.accumulatedLpManagerFees
      .add(afterVault.feeState.accumulatedLpProtocolFees)
      .sub(beforeVault.feeState.accumulatedLpManagerFees)
      .sub(beforeVault.feeState.accumulatedLpProtocolFees);

    assert.equal(
      actualFeesInLp.toString(),
      expectedFeesInLp.toString(),
      "Fees should be correct"
    );

    // Calculate expected asset return
    const withdrawnAssets = new anchor.BN(afterUserAssetBalance.toString()).sub(
      new anchor.BN(beforeUserAssetBalance.toString())
    );

    const afterAccumulatedLpFees = afterVault.feeState.accumulatedLpAdminFees
      .add(afterVault.feeState.accumulatedLpManagerFees)
      .add(afterVault.feeState.accumulatedLpProtocolFees)
      .add(afterVault.deadWeight);

    const currentAmountAssetToWithdraw = new anchor.BN(
      beforeUserLpBalance.toString()
    )
      .mul(new anchor.BN(beforeTotalAssets.toString()))
      .mul(new anchor.BN(MAX_FEE_BPS - REDEMPTION_FEE_PERCENTAGE_BPS))
      .div(
        new anchor.BN(beforeLpSupply.toString())
          .add(afterAccumulatedLpFees)
          .mul(new anchor.BN(MAX_FEE_BPS))
      );

    const expectedAmountAssetToWithdraw = currentAmountAssetToWithdraw.gt(
      originalAmountAssetToWithdraw
    )
      ? originalAmountAssetToWithdraw
      : currentAmountAssetToWithdraw;

    // Verify total assets decreased by withdrawn amount
    assert.equal(
      afterTotalAssets.toString(),
      beforeTotalAssets.sub(withdrawnAssets).toString(),
      "Vault total assets should decrease by withdrawn amount"
    );

    // Verify LP tokens were burned
    assert.equal(
      (beforeLpSupply - afterLpSupply).toString(),
      beforeUserLpBalance.toString(),
      "LP tokens should be burned proportionally"
    );

    // Verify user's LP balance is 0
    assert.equal(
      afterUserLpBalance.toString(),
      new BN(0).toString(),
      "User's LP token balance should be 0"
    );

    const actualWithdrawnAsset = afterUserAssetBalance - beforeUserAssetBalance;

    // Verify user received correct amount of assets
    assert.equal(
      actualWithdrawnAsset.toString(),
      expectedAmountAssetToWithdraw.toString(),
      "User should receive correct amount of assets"
    );

    // Additional sanity checks
    assert.isTrue(
      afterTotalAssets.lt(beforeTotalAssets),
      "Total assets should decrease"
    );
    assert.isTrue(afterLpSupply < beforeLpSupply, "LP supply should decrease");
  });

  it("Harvests fee from the vault successfully", async () => {
    const vaultAdmin = admin;
    const protocolAdmin = admin;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeVault = await vaultProgram.account.vault.fetch(vault);

    // Accumulated fees already sitting in the state (usually from Performance Fees in previous steps)
    const beforeAccLpAdmin = beforeVault.feeState.accumulatedLpAdminFees;
    const beforeAccLpManager = beforeVault.feeState.accumulatedLpManagerFees;
    const beforeAccLpProtocol = beforeVault.feeState.accumulatedLpProtocolFees;
    const totalBeforeAccLpFees = beforeAccLpAdmin.add(beforeAccLpManager).add(beforeAccLpProtocol);

    // Determine relationships between entities
    const isManagerSameAsVaultAdmin = newManager.equals(vaultAdmin);
    const isManagerSameAsProtocolAdmin = newManager.equals(protocolAdmin);
    const isVaultAdminSameAsProtocolAdmin = vaultAdmin.equals(protocolAdmin);

    // Get protocol admin's LP token account
    const protocolAdminLpAta = await getAssociatedTokenAddress(vaultLpMint, protocolAdmin, true);

    // Helper to get balance safely
    const getBalance = async (ata: PublicKey) => {
      try {
        return new BN((await getAccount(provider.connection, ata)).amount.toString());
      } catch {
        return new BN(0);
      }
    };

    const beforeNewManagerLpAmt = await getBalance(newManagerLpAta);
    const beforeVaultAdminLpAmt = await getBalance(adminLpAta);
    const beforeProtocolAdminLpAmt = await getBalance(protocolAdminLpAta);

    // Create token accounts if needed
    const preInstructions = [
      createAssociatedTokenAccountIdempotentInstruction(provider.wallet.publicKey, newManagerLpAta, newManager, vaultLpMint),
      createAssociatedTokenAccountIdempotentInstruction(provider.wallet.publicKey, adminLpAta, vaultAdmin, vaultLpMint),
      createAssociatedTokenAccountIdempotentInstruction(provider.wallet.publicKey, protocolAdminLpAta, protocolAdmin, vaultLpMint),
    ];

    // Execute harvest fee
    const harvestIx = await client.createHarvestFeeIx({
      harvester: provider.wallet.publicKey,
      vault,
      vaultAdmin,
      vaultManager: newManager,
      protocolAdmin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [...preInstructions, harvestIx],
      payerKp,
      [provider.wallet.payer!]
    );

    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterVault = await vaultProgram.account.vault.fetch(vault);

    const afterNewManagerLpAmt = await getBalance(newManagerLpAta);
    const afterVaultAdminLpAmt = await getBalance(adminLpAta);
    const afterProtocolAdminLpAmt = await getBalance(protocolAdminLpAta);

    const totalMgmtBps = beforeVault.feeConfiguration.managerManagementFee +
      beforeVault.feeConfiguration.adminManagementFee +
      beforeVault.feeConfiguration.protocolManagementFee;

    const timeElapsed = afterVault.feeUpdate.lastManagementFeeUpdateTs.sub(
      beforeVault.feeUpdate.lastManagementFeeUpdateTs
    );
    const feeNumerator = timeElapsed
      .mul(beforeVault.asset.totalValue)
      .mul(new BN(totalMgmtBps));
    const feeDenominator = new BN(MAX_FEE_BPS).mul(new BN(365 * 24 * 60 * 60));

    const expectedMgmtFeeAsset = feeNumerator.add(feeDenominator).subn(1).div(feeDenominator);

    const beforeTotalLpSupply = new BN(beforeLpSupply.toString())
      .add(totalBeforeAccLpFees)
      .add(beforeVault.deadWeight);

    const mgmtLpToMint = new BN(
      new Decimal(expectedMgmtFeeAsset.toString())
        .mul(beforeTotalLpSupply.toString())
        .div(new Decimal(beforeVault.asset.totalValue.toString()).sub(expectedMgmtFeeAsset.toString()))
        .ceil()
        .toString()
    );

    const calcSplit = (lpAmount: anchor.BN, pBps: number, aBps: number, mBps: number) => {
      const totalBps = pBps + aBps + mBps;
      if (totalBps === 0) return [new BN(0), new BN(0), new BN(0)];

      // 1. Protocol share (Ceil)
      const pAmt = lpAmount.muln(pBps).addn(totalBps - 1).divn(totalBps);
      const remaining = lpAmount.sub(pAmt);

      // 2. Admin share of remainder (Ceil)
      const den = aBps + mBps;
      const aAmt = den === 0 ? new BN(0) : remaining.muln(aBps).addn(den - 1).divn(den);

      // 3. Manager gets leftover
      const mAmt = remaining.sub(aAmt);
      return [pAmt, aAmt, mAmt];
    };

    const [mgmtPLp, mgmtALp, mgmtMLp] = calcSplit(
      mgmtLpToMint,
      beforeVault.feeConfiguration.protocolManagementFee,
      beforeVault.feeConfiguration.adminManagementFee,
      beforeVault.feeConfiguration.managerManagementFee
    );

    // Total expected increases for each ATA
    const expectedProtocolIncrease = beforeAccLpProtocol.add(mgmtPLp);
    const expectedAdminIncrease = beforeAccLpAdmin.add(mgmtALp);
    const expectedManagerIncrease = beforeAccLpManager.add(mgmtMLp);

    // --- VERIFICATION SECTION ---

    // Verify State Reset
    assert.equal(afterVault.feeState.accumulatedLpManagerFees.toNumber(), 0);
    assert.equal(afterVault.feeState.accumulatedLpAdminFees.toNumber(), 0);
    assert.equal(afterVault.feeState.accumulatedLpProtocolFees.toNumber(), 0);

    // Verify Balances (considering identity overlaps)
    let actualProtocolDelta = afterProtocolAdminLpAmt.sub(beforeProtocolAdminLpAmt);
    let actualAdminDelta = afterVaultAdminLpAmt.sub(beforeVaultAdminLpAmt);
    let actualManagerDelta = afterNewManagerLpAmt.sub(beforeNewManagerLpAmt);

    if (isVaultAdminSameAsProtocolAdmin && !isManagerSameAsProtocolAdmin) {
      // Admin ATA should have Protocol + Admin fees
      assert.equal(actualAdminDelta.toString(), expectedProtocolIncrease.add(expectedAdminIncrease).toString());
      assert.equal(actualManagerDelta.toString(), expectedManagerIncrease.toString());
    } else if (!isVaultAdminSameAsProtocolAdmin && !isManagerSameAsProtocolAdmin && !isManagerSameAsVaultAdmin) {
      // Everyone is unique
      assert.equal(actualProtocolDelta.toString(), expectedProtocolIncrease.toString());
      assert.equal(actualAdminDelta.toString(), expectedAdminIncrease.toString());
      assert.equal(actualManagerDelta.toString(), expectedManagerIncrease.toString());
    } else {
      // Handle other overlap cases (like the default setup where admin == protocol_admin)
      const totalMinted = afterLpMint.supply - beforeLpSupply;
      const totalExpected = expectedProtocolIncrease.add(expectedAdminIncrease).add(expectedManagerIncrease);
      assert.equal(totalMinted.toString(), totalExpected.toString(), "Total LP minted should match sum of splits");
    }
  });
});