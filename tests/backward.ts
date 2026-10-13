import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VoltrVault } from "../target/types/voltr_vault";
import { assert } from "chai";
import { PublicKey } from "@solana/web3.js";
import {
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
  getMint,
  getAccount,
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { BN } from "bn.js";
import { assertApprox, convertNumberToDecimal } from "./conversion";
import Decimal from "decimal.js";
import { VoltrClient, SEEDS, VaultConfigField, DRIFT_ADAPTOR_PROGRAM_ID } from "../sdk/src";
import { sendAndConfirmVersionedTransaction } from "./transaction";

describe("backward compatibility", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const vaultProgram = anchor.workspace.VoltrVault as Program<VoltrVault>;
  const lendingAdaptorProgramId = new PublicKey(
    "aVoLTRCRt3NnnchvLYH6rMYehJHwM5m45RmLBZq7PGz"
  );

  const client = new VoltrClient(provider.connection);

  const REDEMPTION_FEE_PERCENTAGE_BPS = 0;
  const ISSUANCE_FEE_PERCENTAGE_BPS = 0;
  const DEAD_WEIGHT = new BN(1000);
  const TOLERANCE = 1;

  let protocol: PublicKey;
  let protocolBump: number;
  let vaultLpMint: PublicKey;
  let vaultAssetIdleAuth: PublicKey;
  let adaptorAddReceipt: PublicKey;
  let strategyInitReceipt: PublicKey;
  let vaultStrategyAuth: PublicKey;
  let vaultStrategyAssetAta: PublicKey;
  let strategy: PublicKey;
  let requestWithdrawVaultReceipt: PublicKey;
  let requestWithdrawVaultReceiptBump: number;
  let requestWithdrawLpAta: PublicKey;

  let vaultAssetIdleAta: PublicKey;
  let userAssetAta: PublicKey;
  let userLpAta: PublicKey;
  let managerLpAta: PublicKey;
  let adminLpAta: PublicKey;

  const allOperationalState = 8191;
  const DisabledOperations = { WITHDRAW_VAULT: 1 << 1 };

  const vaultAssetMint = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
  const vault = new PublicKey("GqoypwVGG35JSR1AwCm2jeqJPUPvA4cWE45rSbfxHgdK");

  const initialDepositAmount = new anchor.BN(10_000_000);
  const subsequentDepositAmount = new anchor.BN(2_000_000);
  const withdrawLpAmount = new anchor.BN(2_000_000);
  const withdrawAssetAmount = new anchor.BN(2_000_000);
  const depositStrategyAmount = new anchor.BN(5_000_000);
  const directWithdrawStrategyLpAmount = new anchor.BN(1_000_000);
  const directWithdrawStrategyAssetAmount = new anchor.BN(1_000_000);

  const driftProgram = new PublicKey("dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH");
  const manager = new PublicKey("mngr6Afk47ANWYREWPppKg8Xf1UQQY857oXDyCEqKLz");
  const admin = provider.wallet.publicKey;

  let userStats: PublicKey;
  let state = new PublicKey("5zpq7DvB6UdFFvpmBPspGPNfUGoBRRCE2HHg5u3gxcsN");
  let user: PublicKey;
  const marketIndex = 0;
  const subAccountId = 0;
  const oracle = new PublicKey("9VCioxmni2gDLv11qufWzT3RDERhQE4iY5Gf7NTfYyAV");

  const [spotMarket] = PublicKey.findProgramAddressSync([Buffer.from("spot_market"), new BN(marketIndex).toArrayLike(Buffer, "le", 2)], driftProgram);
  const [bankLiquidityVault] = PublicKey.findProgramAddressSync([Buffer.from("spot_market_vault"), new BN(marketIndex).toArrayLike(Buffer, "le", 2)], driftProgram);
  const [bankLiquidityVaultAuth] = PublicKey.findProgramAddressSync([Buffer.from("drift_signer")], driftProgram);

  it("Airdrops SOL to user", async () => {
    const airdropUserSig = await provider.connection.requestAirdrop(provider.wallet.publicKey, 90e9);
    const latestBlockhash = await provider.connection.getLatestBlockhash();

    await provider.connection.confirmTransaction({
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      signature: airdropUserSig,
    });
  });

  it("Initialize tokens and accounts using SDK", async () => {
    [protocol, protocolBump] = await PublicKey.findProgramAddress([SEEDS.PROTOCOL], vaultProgram.programId);

    vaultLpMint = client.findVaultLpMint(vault);
    vaultAssetIdleAuth = client.findVaultAssetIdleAuth(vault);

    requestWithdrawVaultReceipt = client.findRequestWithdrawVaultReceipt(vault, provider.wallet.publicKey);
    [, requestWithdrawVaultReceiptBump] = PublicKey.findProgramAddressSync(
      [SEEDS.REQUEST_WITHDRAW_VAULT_RECEIPT, vault.toBuffer(), provider.wallet.publicKey.toBuffer()],
      vaultProgram.programId
    );

    [strategy] = await PublicKey.findProgramAddress([SEEDS.STRATEGY, bankLiquidityVault.toBuffer()], lendingAdaptorProgramId);

    [adaptorAddReceipt] = await PublicKey.findProgramAddress(
      [SEEDS.ADAPTOR_ADD_RECEIPT, vault.toBuffer(), lendingAdaptorProgramId.toBuffer()],
      vaultProgram.programId
    );

    const stratAddrs = client.findVaultStrategyAddresses(vault, strategy);
    strategyInitReceipt = stratAddrs.strategyInitReceipt;
    vaultStrategyAuth = stratAddrs.vaultStrategyAuth;

    [user] = PublicKey.findProgramAddressSync([Buffer.from("user"), vaultStrategyAuth.toBuffer(), new BN(subAccountId).toArrayLike(Buffer, "le", 2)], driftProgram);
    [userStats] = PublicKey.findProgramAddressSync([Buffer.from("user_stats"), vaultStrategyAuth.toBuffer()], driftProgram);

    requestWithdrawLpAta = getAssociatedTokenAddressSync(vaultLpMint, requestWithdrawVaultReceipt, true);
    vaultAssetIdleAta = getAssociatedTokenAddressSync(vaultAssetMint, vaultAssetIdleAuth, true);
    vaultStrategyAssetAta = getAssociatedTokenAddressSync(vaultAssetMint, vaultStrategyAuth, true);
    userAssetAta = getAssociatedTokenAddressSync(vaultAssetMint, provider.wallet.publicKey);
    userLpAta = getAssociatedTokenAddressSync(vaultLpMint, provider.wallet.publicKey);
    managerLpAta = getAssociatedTokenAddressSync(vaultLpMint, manager);
    adminLpAta = getAssociatedTokenAddressSync(vaultLpMint, admin);
  });

  it("initialize protocol", async () => {
    const tx = await vaultProgram.methods
      .initOrUpdateProtocol(allOperationalState)
      .accounts({
        payer: provider.wallet.publicKey,
        currentAdmin: admin,
        newAdmin: admin,
      })
      .signers([])
      .rpc();

    const protocolAccount = await vaultProgram.account.protocol.fetch(protocol);

    assert.equal(protocolAccount.operationalState, allOperationalState);
    assert.equal(protocolAccount.bump, protocolBump);
    assert.ok(protocolAccount.admin.equals(provider.wallet.publicKey));
  });

  it("Initial deposits assets successfully", async () => {
    // Create user LP ATA if it doesn't exist
    const createAssociatedTokenAccountIdempotentIx =
      createAssociatedTokenAccountIdempotentInstruction(
        provider.wallet.publicKey,
        userLpAta,
        provider.wallet.publicKey,
        vaultLpMint,
        TOKEN_PROGRAM_ID
      );

    // Get state before deposit
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);
    const beforeUserAssetBalance = beforeUserAsset.amount;

    const depositIx = await client.createDepositVaultIx(initialDepositAmount, {
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [createAssociatedTokenAccountIdempotentIx, depositIx],
      provider.wallet.payer!,
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
    if (beforeTotalAssets.eqn(0) && beforeLpSupply === BigInt(0)) {
      assert.equal(
        afterLpSupply.toString(),
        initialDepositAmount
          .mul(new BN(10 ** 9))
          .div(new BN(10 ** 6))
          .sub(DEAD_WEIGHT)
          .toString(),
        "First deposit should mint 1:1 LP tokens"
      );
      assert.equal(
        afterUserLpBalance.toString(),
        initialDepositAmount
          .mul(new BN(10 ** 9))
          .div(new BN(10 ** 6))
          .sub(DEAD_WEIGHT)
          .toString(),
        "User should receive 1:1 LP tokens on first deposit"
      );
    } else {
      const totalAccumulatedFeesLp = afterVault.feeState.accumulatedLpAdminFees
        .add(afterVault.feeState.accumulatedLpManagerFees)
        .add(afterVault.feeState.accumulatedLpProtocolFees);
      const afterLpSupplyInclFees = new BN(afterLpSupply.toString())
        .add(totalAccumulatedFeesLp)
        .add(afterVault.deadWeight);
      const lpTokenMinted = afterLpSupply - beforeLpSupply;

      const expectedLpAmount = initialDepositAmount
        .mul(afterLpSupplyInclFees)
        .mul(new anchor.BN(10_000 - ISSUANCE_FEE_PERCENTAGE_BPS))
        .div(new anchor.BN(10_000).mul(new anchor.BN(afterTotalAssets)));

      assert.equal(
        lpTokenMinted.toString(),
        expectedLpAmount.sub(DEAD_WEIGHT).toString(),
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
      provider.wallet.payer!,
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
      assert.equal(
        afterLpSupply.toString(),
        subsequentDepositAmount
          .mul(new BN(10 ** 9))
          .div(new BN(10 ** 6))
          .sub(DEAD_WEIGHT)
          .toString(),
        "First deposit should mint 1:1 LP tokens"
      );
      assert.equal(
        afterUserLpBalance.toString(),
        subsequentDepositAmount
          .mul(new BN(10 ** 9))
          .div(new BN(10 ** 6))
          .sub(DEAD_WEIGHT)
          .toString(),
        "User should receive 1:1 LP tokens on first deposit"
      );
    } else {
      const totalAccumulatedFeesLp = afterVault.feeState.accumulatedLpAdminFees
        .add(afterVault.feeState.accumulatedLpManagerFees)
        .add(afterVault.feeState.accumulatedLpProtocolFees);
      const afterLpSupplyInclFees = new BN(afterLpSupply.toString())
        .add(totalAccumulatedFeesLp)
        .add(afterVault.deadWeight);
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
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [createRequestWithdrawLpAtaIx, requestWithdrawIx],
      provider.wallet.payer!,
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

  it("Cancels withdraw request successfully", async () => {
    const beforeUserLp = await getAccount(provider.connection, userLpAta);
    const beforeUserLpBalance = beforeUserLp.amount;
    const beforeRequestWithdrawVaultReceipt =
      await vaultProgram.account.requestWithdrawVaultReceipt.fetch(
        requestWithdrawVaultReceipt
      );
    const beforeRequestWithdrawVaultReceiptAmountLpEscrowed =
      beforeRequestWithdrawVaultReceipt.amountLpEscrowed;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;

    const cancelIx = await client.createCancelRequestWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [cancelIx],
      provider.wallet.payer!,
      [provider.wallet.payer!]
    );

    const afterUserLp = await getAccount(provider.connection, userLpAta);
    const afterUserLpBalance = afterUserLp.amount;
    const refundedLpToUser = afterUserLpBalance - beforeUserLpBalance;
    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterLpSupply = afterLpMint.supply;

    const burntLp = beforeLpSupply - afterLpSupply;

    assertApprox(
      refundedLpToUser.toString(),
      beforeRequestWithdrawVaultReceiptAmountLpEscrowed.toString(),
      TOLERANCE,
      "LP should be refunded to the user"
    );

    assert.equal(
      (burntLp + refundedLpToUser).toString(),
      beforeRequestWithdrawVaultReceiptAmountLpEscrowed.toString(),
      "Sum of LP burnt and refunded should be equal to the LP escrowed"
    );

    const afterRequestWithdrawVaultReceipt =
      await provider.connection.getAccountInfo(requestWithdrawVaultReceipt);
    assert.isNull(
      afterRequestWithdrawVaultReceipt,
      "Request withdraw vault receipt should be closed"
    );
  });

  it("Update vault config - MaxCap", async () => {
    const newMaxCap = new BN(Number.MAX_SAFE_INTEGER);
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
      provider.wallet.payer!,
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

  it("Update vault config - ManagerManagementFee (should charge management fee)", async () => {
    const newManagerManagementFee = 4900; // 49%
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeAccumulatedManagerFees =
      beforeVault.feeState.accumulatedLpManagerFees;

    const data = Buffer.alloc(2);
    data.writeUInt16LE(newManagerManagementFee, 0);

    // Management fee updates require the LP mint as a remaining account
    const updateIx = await client.createUpdateVaultConfigIx(VaultConfigField.ManagerManagementFee, data, {
      vault,
      admin,
      vaultLpMint,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [updateIx],
      provider.wallet.payer!,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.feeConfiguration.managerManagementFee,
      newManagerManagementFee,
      "Manager management fee should be updated"
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

  it("Updates vault protocol fees", async () => {
    const performanceFeeBps = 0; // 0%
    const managementFeeBps = 5000;  // 50%

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
      provider.wallet.payer!,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    assert.equal(
      afterVault.vaultConfiguration.disabledOperations.toString(),
      newDisabledOperations.toString(),
      "Disabled operations should be updated"
    );
  });

  it("Should fail when disabled operation is not allowed", async () => {
    try {
      const withdrawRequestIx = await client.createRequestWithdrawVaultIx(
        { amount: withdrawLpAmount, isAmountInLp: true, isWithdrawAll: false },
        {
          payer: provider.wallet.publicKey,
          userTransferAuthority: provider.wallet.publicKey,
          vault: vault,
        }
      );
      await sendAndConfirmVersionedTransaction(
        provider.connection,
        [withdrawRequestIx],
        provider.wallet.payer!,
        [provider.wallet.payer!]
      );
      assert.fail("Should not allow disabled operation");
    } catch (error: any) {
      assert.include(error.message, "OperationNotAllowed.");
    }
  });

  it("Withdraws assets in LP successfully", async () => {
    const enableWithdrawIx = await client.createUpdateVaultConfigIx(VaultConfigField.DisabledOperations, new BN(0).toArrayLike(Buffer, "le", 2), {
      vault,
      admin,
    });

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

    const withdrawRequestIx = await client.createRequestWithdrawVaultIx(
      { amount: withdrawLpAmount, isAmountInLp: true, isWithdrawAll: false },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    const withdrawIx = await client.createWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [enableWithdrawIx, withdrawRequestIx, withdrawIx],
      provider.wallet.payer!,
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

    // Calculate expected asset return
    const withdrawnAssets = new anchor.BN(withdrawLpAmount.toString())
      .mul(new anchor.BN(beforeTotalAssets.toString()))
      .mul(new anchor.BN(10000 - REDEMPTION_FEE_PERCENTAGE_BPS))
      .div(new anchor.BN(beforeLpSupplyInclFees.toString()))
      .div(new anchor.BN(10000));

    // Verify total assets decreased by withdrawn amount
    assert.equal(
      afterTotalAssets.toString(),
      beforeTotalAssets.sub(withdrawnAssets).toString(),
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
      withdrawnAssets.toString(),
      "User should receive correct amount of assets"
    );

    // Additional sanity checks
    assert.isTrue(
      afterTotalAssets.ltn(beforeTotalAssets.toNumber()),
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

    const withdrawRequestIx = await client.createRequestWithdrawVaultIx(
      { amount: withdrawAssetAmount, isAmountInLp: false, isWithdrawAll: false },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );
    const withdrawIx = await client.createWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawRequestIx, withdrawIx],
      provider.wallet.payer!,
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

    // Calculate expected asset return
    const expectedBurnedLpAmountNumerator = new anchor.BN(
      withdrawAssetAmount.toString()
    )
      .mul(new anchor.BN(beforeLpSupplyInclFees.toString()))
      .mul(new anchor.BN(10000));

    const expectedBurnedLpAmountDenominator = new anchor.BN(
      beforeTotalAssets.toString()
    ).mul(new anchor.BN(10000 - REDEMPTION_FEE_PERCENTAGE_BPS));

    const expectedBurnedLpAmountNumeratorDecimal = convertNumberToDecimal(
      expectedBurnedLpAmountNumerator
    );

    const expectedBurnedLpAmountDenominatorDecimal = convertNumberToDecimal(
      expectedBurnedLpAmountDenominator
    );

    const expectedBurnedLpAmountDecimal =
      expectedBurnedLpAmountNumeratorDecimal.div(
        expectedBurnedLpAmountDenominatorDecimal
      );

    assert.equal(
      beforeTotalAssets.sub(afterTotalAssets).toString(),
      (afterUserAssetBalance - beforeUserAssetBalance).toString(),
      "Change in vault assets should be opposingly equal to change in user asset balance"
    );

    assertApprox(
      beforeTotalAssets.sub(afterTotalAssets).toString(),
      withdrawAssetAmount.toString(),
      TOLERANCE,
      "Vault total assets should decrease by withdrawn amount"
    );

    // Verify user received correct amount of assets
    assertApprox(
      (afterUserAssetBalance - beforeUserAssetBalance).toString(),
      withdrawAssetAmount.toString(),
      TOLERANCE,
      "User should receive correct amount of assets"
    );

    // Verify LP tokens were burned
    assert.equal(
      (beforeLpSupply - afterLpSupply).toString(),
      expectedBurnedLpAmountDecimal.ceil().toString(),
      "LP tokens should be burned proportionally"
    );

    // Verify user's LP balance decreased
    assert.equal(
      (beforeUserLpBalance - afterUserLpBalance).toString(),
      expectedBurnedLpAmountDecimal.ceil().toString(),
      "User's LP token balance should decrease by withdrawal amount"
    );

    // Additional sanity checks
    assert.isTrue(
      afterTotalAssets.ltn(beforeTotalAssets.toNumber()),
      "Total assets should decrease"
    );
    assert.isTrue(afterLpSupply < beforeLpSupply, "LP supply should decrease");
  });

  it("Deposits assets to the strategy successfully", async () => {
    const createAtaIx = createAssociatedTokenAccountIdempotentInstruction(
      provider.wallet.publicKey,
      vaultStrategyAssetAta,
      vaultStrategyAuth,
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
      {
        depositAmount: depositStrategyAmount, instructionDiscriminator: null, additionalArgs:
          Buffer.from([...new BN(marketIndex).toArrayLike(Buffer, "le", 2)])
      },
      {
        manager: provider.wallet.publicKey,
        vault,
        vaultAssetMint: vaultAssetMint,
        adaptorProgram: lendingAdaptorProgramId,
        strategy,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: bankLiquidityVault, isSigner: false, isWritable: true },
          { pubkey: driftProgram, isSigner: false, isWritable: false },
          { pubkey: state, isSigner: false, isWritable: false },
          { pubkey: user, isSigner: false, isWritable: true },
          { pubkey: userStats, isSigner: false, isWritable: true },
          { pubkey: oracle, isSigner: false, isWritable: false },
          { pubkey: spotMarket, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [createAtaIx, depositStrategyIx],
      provider.wallet.payer!,
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

  it("User withdraws assets in asset directly from strategy successfully", async () => {
    const beforeVault = await vaultProgram.account.vault.fetch(vault);

    const beforeStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);

    const withdrawRequestIx = await client.createRequestWithdrawVaultIx(
      { amount: directWithdrawStrategyAssetAmount, isAmountInLp: false, isWithdrawAll: false },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    const directWithdrawStrategyIx = await client.createDirectWithdrawStrategyIx(
      { userArgs: null },
      {
        user: provider.wallet.publicKey,
        vault,
        vaultAssetMint: vaultAssetMint,
        adaptorProgram: lendingAdaptorProgramId,
        strategy,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: bankLiquidityVaultAuth, isSigner: false, isWritable: true },
          { pubkey: bankLiquidityVault, isSigner: false, isWritable: true },
          { pubkey: driftProgram, isSigner: false, isWritable: false },
          { pubkey: state, isSigner: false, isWritable: false },
          { pubkey: user, isSigner: false, isWritable: true },
          { pubkey: userStats, isSigner: false, isWritable: true },
          { pubkey: oracle, isSigner: false, isWritable: false },
          { pubkey: spotMarket, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawRequestIx, directWithdrawStrategyIx],
      provider.wallet.payer!,
      [provider.wallet.payer!]
    );

    const afterUserAsset = await getAccount(provider.connection, userAssetAta);

    const afterVault = await vaultProgram.account.vault.fetch(vault);

    const deltaVaultAssets =
      afterVault.asset.totalValue.toNumber() -
      beforeVault.asset.totalValue.toNumber();

    const deltaUserAsset = afterUserAsset.amount - beforeUserAsset.amount;

    const afterStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    assert.ok(
      Number(deltaUserAsset) <=
      directWithdrawStrategyAssetAmount.add(new BN(1)).toNumber() &&
      Number(deltaUserAsset) >=
      directWithdrawStrategyAssetAmount.sub(new BN(1)).toNumber()
    );
    assert.ok(
      deltaVaultAssets ===
      afterStrategyInitReceiptAccount.positionValue.toNumber() -
      beforeStrategyInitReceiptAccount.positionValue.toNumber()
    );
  });

  it("User withdraws assets in LP directly from strategy successfully", async () => {
    const beforeVault = await vaultProgram.account.vault.fetch(vault);

    const beforeStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);
    const beforeUserLp = await getAccount(provider.connection, userLpAta);

    const withdrawRequestIx = await client.createRequestWithdrawVaultIx(
      { amount: directWithdrawStrategyLpAmount, isAmountInLp: true, isWithdrawAll: false },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    const directWithdrawStrategyIx = await client.createDirectWithdrawStrategyIx(
      { userArgs: null },
      {
        user: provider.wallet.publicKey,
        vault,
        strategy,
        vaultAssetMint,
        adaptorProgram: lendingAdaptorProgramId,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: bankLiquidityVaultAuth, isSigner: false, isWritable: true },
          { pubkey: bankLiquidityVault, isSigner: false, isWritable: true },
          { pubkey: driftProgram, isSigner: false, isWritable: false },
          { pubkey: state, isSigner: false, isWritable: false },
          { pubkey: user, isSigner: false, isWritable: true },
          { pubkey: userStats, isSigner: false, isWritable: true },
          { pubkey: oracle, isSigner: false, isWritable: false },
          { pubkey: spotMarket, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawRequestIx, directWithdrawStrategyIx],
      provider.wallet.payer!,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterUserLp = await getAccount(provider.connection, userLpAta);

    const deltaVaultAssets =
      afterVault.asset.totalValue.toNumber() -
      beforeVault.asset.totalValue.toNumber();

    const deltaUserLp = beforeUserLp.amount - afterUserLp.amount;

    const afterStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    assert.ok(
      deltaUserLp.toString() === directWithdrawStrategyLpAmount.toString()
    );
    assert.ok(
      deltaVaultAssets ===
      afterStrategyInitReceiptAccount.positionValue.toNumber() -
      beforeStrategyInitReceiptAccount.positionValue.toNumber()
    );
  });

  it("Withdraws assets from the strategy successfully", async () => {
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;

    const beforeStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);
    const beforeIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );

    const withdrawStrategyIx = await client.createWithdrawStrategyIx(
      {
        withdrawAmount: depositStrategyAmount.add(new BN(1_000_000)),
        instructionDiscriminator: null,
        additionalArgs:
          Buffer.from([...new BN(marketIndex).toArrayLike(Buffer, "le", 2)])
      },
      {
        manager: provider.wallet.publicKey,
        vault,
        vaultAssetMint: vaultAssetMint,
        adaptorProgram: lendingAdaptorProgramId,
        strategy,
        assetTokenProgram: TOKEN_PROGRAM_ID,
        remainingAccounts: [
          { pubkey: bankLiquidityVaultAuth, isSigner: false, isWritable: true },
          { pubkey: bankLiquidityVault, isSigner: false, isWritable: true },
          { pubkey: driftProgram, isSigner: false, isWritable: false },
          { pubkey: state, isSigner: false, isWritable: false },
          { pubkey: user, isSigner: false, isWritable: true },
          { pubkey: userStats, isSigner: false, isWritable: true },
          { pubkey: oracle, isSigner: false, isWritable: false },
          { pubkey: spotMarket, isSigner: false, isWritable: true },
        ]
      }
    );

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [withdrawStrategyIx],
      provider.wallet.payer!,
      [provider.wallet.payer!]
    );

    const afterVault = await vaultProgram.account.vault.fetch(vault);
    const afterTotalAssets = afterVault.asset.totalValue;

    const afterStrategyInitReceiptAccount =
      await vaultProgram.account.strategyInitReceipt.fetch(strategyInitReceipt);

    const afterIdleAta = await getAccount(
      provider.connection,
      vaultAssetIdleAta
    );

    const actualWithdrawAmount = afterIdleAta.amount - beforeIdleAta.amount;

    assert.ok(
      afterTotalAssets.toNumber() - beforeTotalAssets.toNumber() ===
      afterStrategyInitReceiptAccount.positionValue.toNumber() -
      (beforeStrategyInitReceiptAccount.positionValue.toNumber() -
        Number(actualWithdrawAmount))
    );
  });

  it("Withdraws all assets from the vault successfully", async () => {
    // sleep for 1 min
    await new Promise((resolve) => setTimeout(resolve, 6000));
    // Get state before withdrawal
    const beforeVault = await vaultProgram.account.vault.fetch(vault);
    const beforeTotalAssets = beforeVault.asset.totalValue;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeUserLp = await getAccount(provider.connection, userLpAta);
    const beforeUserLpBalance = beforeUserLp.amount;
    const beforeUserAsset = await getAccount(provider.connection, userAssetAta);
    const beforeUserAssetBalance = beforeUserAsset.amount;
    const beforeAccumulatedFeesLp = beforeVault.feeState.accumulatedLpAdminFees
      .add(beforeVault.feeState.accumulatedLpManagerFees)
      .add(beforeVault.feeState.accumulatedLpProtocolFees);
    const beforeLpSupplyInclFees = new BN(beforeLpSupply.toString())
      .add(beforeAccumulatedFeesLp)
      .add(beforeVault.deadWeight);

    const requestWithdrawIx = await client.createRequestWithdrawVaultIx(
      { amount: new BN(Number.MAX_SAFE_INTEGER), isAmountInLp: false, isWithdrawAll: true },
      {
        payer: provider.wallet.publicKey,
        userTransferAuthority: provider.wallet.publicKey,
        vault: vault,
      }
    );

    const withdrawIx = await client.createWithdrawVaultIx({
      userTransferAuthority: provider.wallet.publicKey,
      vault: vault,
      vaultAssetMint: vaultAssetMint,
      assetTokenProgram: TOKEN_PROGRAM_ID,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [requestWithdrawIx, withdrawIx],
      provider.wallet.payer!,
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

    const timeElapsed = new BN(
      afterVault.feeUpdate.lastManagementFeeUpdateTs.toString()
    ).sub(new BN(beforeVault.feeUpdate.lastManagementFeeUpdateTs.toString()));

    const feeNumerator = timeElapsed
      .mul(beforeVault.asset.totalValue)
      .mul(new BN(beforeVault.feeConfiguration.managerManagementFee
        + beforeVault.feeConfiguration.protocolManagementFee
        + beforeVault.feeConfiguration.adminManagementFee));
    const feeDenominator = new BN(10_000).mul(new BN(365 * 24 * 60 * 60));

    const expectedFeesInAsset = feeNumerator
      .add(feeDenominator)
      .subn(1)
      .div(feeDenominator);

    const expectedFeesInLp = new Decimal(expectedFeesInAsset.toString())
      .mul(beforeLpSupplyInclFees.toString())
      .div(
        new Decimal(beforeVault.asset.totalValue.toString()).sub(
          expectedFeesInAsset.toString()
        )
      )
      .ceil();

    const totalAccumulatedFeesLpBefore = beforeVault.feeState.accumulatedLpAdminFees
      .add(beforeVault.feeState.accumulatedLpManagerFees)
      .add(beforeVault.feeState.accumulatedLpProtocolFees);

    const totalAccumulatedFeesLpAfter = afterVault.feeState.accumulatedLpAdminFees
      .add(afterVault.feeState.accumulatedLpManagerFees)
      .add(afterVault.feeState.accumulatedLpProtocolFees);

    const actualFeesInLp = totalAccumulatedFeesLpAfter.sub(totalAccumulatedFeesLpBefore);

    assertApprox(
      actualFeesInLp.toString(),
      expectedFeesInLp.toString(),
      1,
      "Fees should be correct"
    );

    const actualFeesInLpManager = afterVault.feeState.accumulatedLpManagerFees.sub(beforeVault.feeState.accumulatedLpManagerFees);
    const actualFeesInLpAdmin = afterVault.feeState.accumulatedLpAdminFees.sub(beforeVault.feeState.accumulatedLpAdminFees);
    const actualFeesInLpProtocol = afterVault.feeState.accumulatedLpProtocolFees.sub(beforeVault.feeState.accumulatedLpProtocolFees);

    const expectedFeesInLpProtocol = expectedFeesInLp.mul(beforeVault.feeConfiguration.protocolManagementFee).div(
      beforeVault.feeConfiguration.managerManagementFee
      + beforeVault.feeConfiguration.adminManagementFee
      + beforeVault.feeConfiguration.protocolManagementFee
    ).ceil();

    const expectedFeesInLpAdmin = new Decimal(actualFeesInLp.toString()).sub(
      new Decimal(actualFeesInLpProtocol.toString())
    ).mul(new Decimal(beforeVault.feeConfiguration.adminManagementFee)).div(
      beforeVault.feeConfiguration.managerManagementFee
      + beforeVault.feeConfiguration.adminManagementFee
    ).ceil();

    const expectedFeesInLpManager = actualFeesInLp.sub(actualFeesInLpProtocol).sub(actualFeesInLpAdmin);

    assert.equal(actualFeesInLpProtocol.toString(), expectedFeesInLpProtocol.toString(), "Protocol fees should be correct");
    assert.equal(actualFeesInLpAdmin.toString(), expectedFeesInLpAdmin.toString(), "Admin fees should be correct");
    assert.equal(actualFeesInLpManager.toString(), expectedFeesInLpManager.toString(), "Manager fees should be correct");

    // Calculate expected asset return
    const withdrawnAssets = new anchor.BN(afterUserAssetBalance.toString()).sub(
      new anchor.BN(beforeUserAssetBalance.toString())
    );

    const totalAccumulatedFeesLp = afterVault.feeState.accumulatedLpAdminFees
      .add(afterVault.feeState.accumulatedLpManagerFees)
      .add(afterVault.feeState.accumulatedLpProtocolFees);

    const beforeTotalLpSupply = new BN(beforeLpSupply.toString())
      .add(totalAccumulatedFeesLp)
      .add(beforeVault.deadWeight);

    const expectedWithdrawnAssets = new anchor.BN(
      beforeUserLpBalance.toString()
    )
      .mul(new anchor.BN(beforeTotalAssets.toString()))
      .mul(new anchor.BN(10000 - REDEMPTION_FEE_PERCENTAGE_BPS))
      .div(
        new anchor.BN(beforeTotalLpSupply.toString()).mul(new anchor.BN(10000))
      );

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

    // Verify user received correct amount of assets
    assert.equal(
      (afterUserAssetBalance - beforeUserAssetBalance).toString(),
      expectedWithdrawnAssets.toString(),
      "User should receive correct amount of assets"
    );

    // Additional sanity checks
    assert.isTrue(
      afterTotalAssets.ltn(beforeTotalAssets.toNumber()),
      "Total assets should decrease"
    );
    assert.isTrue(afterLpSupply < beforeLpSupply, "LP supply should decrease");
  });

  it("Removes a strategy from the vault successfully", async () => {
    // Find PDA for vault strategy account

    // Execute remove strategy
    await vaultProgram.methods
      .removeAdaptor()
      .accounts({
        vault,
        adaptorProgram: lendingAdaptorProgramId,
      })
      .signers([])
      .rpc();

    // Verify the account was closed
    const adaptorAddReceiptAccountInfo =
      await provider.connection.getAccountInfo(adaptorAddReceipt);
    assert.isNull(
      adaptorAddReceiptAccountInfo,
      "Adaptor account should be closed"
    );
  });

  it("Harvests fee from the vault successfully", async () => {
    const vaultAdmin = admin;
    const protocolAdmin = admin;
    const beforeLpMint = await getMint(provider.connection, vaultLpMint);
    const beforeLpSupply = beforeLpMint.supply;
    const beforeVault = await vaultProgram.account.vault.fetch(vault);

    // 1. Existing fees sitting in state (from performance fees or earlier mgmt fees)
    const beforeAccLpAdmin = beforeVault.feeState.accumulatedLpAdminFees;
    const beforeAccLpManager = beforeVault.feeState.accumulatedLpManagerFees;
    const beforeAccLpProtocol = beforeVault.feeState.accumulatedLpProtocolFees;
    const totalBeforeAccLpFees = beforeAccLpAdmin.add(beforeAccLpManager).add(beforeAccLpProtocol);

    const isManagerSameAsVaultAdmin = manager.equals(vaultAdmin);
    const isManagerSameAsProtocolAdmin = manager.equals(protocolAdmin);
    const isVaultAdminSameAsProtocolAdmin = vaultAdmin.equals(protocolAdmin);

    const protocolAdminLpAta = await getAssociatedTokenAddress(vaultLpMint, protocolAdmin, true);

    const getBalance = async (ata: PublicKey) => {
      try {
        return new BN((await getAccount(provider.connection, ata)).amount.toString());
      } catch { return new BN(0); }
    };

    const b4ManagerAmt = await getBalance(managerLpAta);
    const b4AdminAmt = await getBalance(adminLpAta);
    const b4ProtocolAmt = await getBalance(protocolAdminLpAta);

    const preInstructions = [
      createAssociatedTokenAccountIdempotentInstruction(provider.wallet.publicKey, managerLpAta, manager, vaultLpMint),
      createAssociatedTokenAccountIdempotentInstruction(provider.wallet.publicKey, adminLpAta, vaultAdmin, vaultLpMint),
      createAssociatedTokenAccountIdempotentInstruction(provider.wallet.publicKey, protocolAdminLpAta, protocolAdmin, vaultLpMint),
    ];

    const harvestFeeIx = await client.createHarvestFeeIx({
      harvester: provider.wallet.publicKey,
      vault,
      vaultAdmin,
      vaultManager: manager,
      protocolAdmin,
    });

    await sendAndConfirmVersionedTransaction(
      provider.connection,
      [...preInstructions, harvestFeeIx],
      provider.wallet.payer!,
      [provider.wallet.payer!]
    );

    const afterLpMint = await getMint(provider.connection, vaultLpMint);
    const afterVault = await vaultProgram.account.vault.fetch(vault);

    const aftManagerAmt = await getBalance(managerLpAta);
    const aftAdminAmt = await getBalance(adminLpAta);
    const aftProtocolAmt = await getBalance(protocolAdminLpAta);

    // --- MATH SECTION: Calculate Management Fee Accrued DURING the harvest call ---
    const totalMgmtBps = beforeVault.feeConfiguration.managerManagementFee +
      beforeVault.feeConfiguration.adminManagementFee +
      beforeVault.feeConfiguration.protocolManagementFee;

    const timeElapsed = afterVault.feeUpdate.lastManagementFeeUpdateTs.sub(
      beforeVault.feeUpdate.lastManagementFeeUpdateTs
    );

    // fee_in_asset = ceil(total_assets * time * bps / (MAX_BPS * YEAR))
    const feeNumerator = timeElapsed.mul(beforeVault.asset.totalValue).mul(new BN(totalMgmtBps));
    const feeDenominator = new BN(10_000).mul(new BN(365 * 24 * 60 * 60));
    const expectedMgmtFeeAsset = feeNumerator.add(feeDenominator).subn(1).div(feeDenominator);

    const beforeTotalLpSupplyInclAll = new BN(beforeLpSupply.toString())
      .add(totalBeforeAccLpFees)
      .add(beforeVault.deadWeight);

    // lp_to_mint = ceil(asset_fee * supply / (total_assets - asset_fee))
    const expectedMgmtLpMinted = new BN(
      new Decimal(expectedMgmtFeeAsset.toString())
        .mul(beforeTotalLpSupplyInclAll.toString())
        .div(new Decimal(beforeVault.asset.totalValue.toString()).sub(expectedMgmtFeeAsset.toString()))
        .ceil()
        .toString()
    );

    // Calculate Split of the NEWLY accrued management fee (matching Rust calc_fee_split)
    const calcSplit = (lpAmount: anchor.BN, pBps: number, aBps: number, mBps: number) => {
      const total = pBps + aBps + mBps;
      if (total === 0) return [new BN(0), new BN(0), new BN(0)];
      const pAmt = lpAmount.muln(pBps).addn(total - 1).divn(total);
      const rem = lpAmount.sub(pAmt);
      const aAmt = (aBps + mBps === 0) ? new BN(0) : rem.muln(aBps).addn(aBps + mBps - 1).divn(aBps + mBps);
      const mAmt = rem.sub(aAmt);
      return [pAmt, aAmt, mAmt];
    };

    const [newPLp, newALp, newMLp] = calcSplit(
      expectedMgmtLpMinted,
      beforeVault.feeConfiguration.protocolManagementFee,
      beforeVault.feeConfiguration.adminManagementFee,
      beforeVault.feeConfiguration.managerManagementFee
    );

    // Totals = Existing sitting fees + newly accrued split
    const totalExpProtocol = beforeAccLpProtocol.add(newPLp);
    const totalExpAdmin = beforeAccLpAdmin.add(newALp);
    const totalExpManager = beforeAccLpManager.add(newMLp);
    const totalExpMinted = totalBeforeAccLpFees.add(expectedMgmtLpMinted);

    // --- VERIFICATION ---
    const lpTokenMinted = afterLpMint.supply - beforeLpSupply;

    assertApprox(
      lpTokenMinted.toString(),
      totalExpMinted.toString(),
      1, // Tolerance for rounding
      "Total LP tokens minted should match sum of old fees + new mgmt accrual"
    );

    assert.equal(afterVault.feeState.accumulatedLpManagerFees.toNumber(), 0);
    assert.equal(afterVault.feeState.accumulatedLpAdminFees.toNumber(), 0);
    assert.equal(afterVault.feeState.accumulatedLpProtocolFees.toNumber(), 0);

    // Verify balance changes (accounting for entity overlaps)
    const protocolDelta = aftProtocolAmt.sub(b4ProtocolAmt);
    const adminDelta = aftAdminAmt.sub(b4AdminAmt);
    const managerDelta = aftManagerAmt.sub(b4ManagerAmt);

    if (isVaultAdminSameAsProtocolAdmin) {
      assertApprox(adminDelta.toString(), totalExpAdmin.add(totalExpProtocol).toString(), 1);
    } else {
      assertApprox(protocolDelta.toString(), totalExpProtocol.toString(), 1);
      assertApprox(adminDelta.toString(), totalExpAdmin.toString(), 1);
    }
    assertApprox(managerDelta.toString(), totalExpManager.toString(), 1);
  });

  it("Closes strategy successfully", async () => {
    await vaultProgram.methods
      .closeStrategy()
      .accounts({
        payer: provider.wallet.publicKey,
        manager: provider.wallet.publicKey,
        vault,
        strategy,
      })
      .rpc();

    const strategyInitReceiptAccountInfo =
      await provider.connection.getAccountInfo(strategyInitReceipt);
    assert.isNull(
      strategyInitReceiptAccountInfo,
      "Strategy init receipt account should be closed"
    );
  });
});
