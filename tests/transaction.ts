import {
  Connection,
  Keypair,
  VersionedTransaction,
  TransactionMessage,
  TransactionConfirmationStrategy,
  TransactionInstruction,
  SendOptions,
  Commitment,
} from "@solana/web3.js";

/**
 * Sends and confirms a versioned transaction with the given instructions and signers.
 * 
 * @param connection - The Solana connection
 * @param instructions - Array of transaction instructions
 * @param payerKeypair - The keypair that will pay for the transaction
 * @param signers - Array of keypairs to sign the transaction
 * @param sendOptions - Optional send options (defaults to skipPreflight: false, preflightCommitment: "confirmed", maxRetries: 5)
 * @param commitment - Optional commitment level for confirmation (defaults to "confirmed")
 * @returns The transaction signature
 */
export async function sendAndConfirmVersionedTransaction(
  connection: Connection,
  instructions: TransactionInstruction[],
  payerKeypair: Keypair,
  signers: Keypair[],
  sendOptions?: SendOptions,
  commitment: Commitment = "confirmed"
): Promise<string> {
  // Get the latest blockhash
  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash();

  // Create the versioned transaction
  const transaction = new VersionedTransaction(
    new TransactionMessage({
      instructions,
      payerKey: payerKeypair.publicKey,
      recentBlockhash: blockhash,
    }).compileToV0Message()
  );

  // Sign the transaction with all signers
  transaction.sign([payerKeypair, ...signers]);

  // Send the transaction with default options if not provided
  const defaultSendOptions: SendOptions = {
    skipPreflight: false,
    maxRetries: 5,
    ...sendOptions,
  };

  const txSig = await connection.sendTransaction(transaction, defaultSendOptions);

  // Create confirmation strategy
  const confirmationStrategy: TransactionConfirmationStrategy = {
    signature: txSig,
    blockhash,
    lastValidBlockHeight,
  };

  // Confirm the transaction
  await connection.confirmTransaction(confirmationStrategy, commitment);

  return txSig;
}
