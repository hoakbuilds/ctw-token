import { Program } from "@coral-xyz/anchor";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import { CtwToken, IDL } from "./program";

export const PROGRAM_ID = new PublicKey(
  "cwTokjpVjxBeytEXomNe5B38EesYsNsXCm3JZC6tmvB"
);

const AUTHORITY_SEED = "AUTHORITY";

export const findAuthorityPda = (tokenMint: PublicKey) => {
  PublicKey.findProgramAddressSync(
    [Buffer.from(AUTHORITY_SEED, "utf-8")],
    PROGRAM_ID
  );
};

const MINT_SEED = "MINT";

export const findConfidentialMintPda = (tokenMint: PublicKey) => {
  PublicKey.findProgramAddressSync(
    [tokenMint.toBuffer(), Buffer.from(MINT_SEED, "utf-8")],
    PROGRAM_ID
  );
};

const program = new Program<CtwToken>(IDL, PROGRAM_ID);

export const initialize = async (
  tokenMint: PublicKey,
  confidentialMint: PublicKey,
  programAuthority: PublicKey,
  tokenVault: PublicKey,
  payer: PublicKey,
  tokenProgram: PublicKey,
  associatedTokenProgram: PublicKey,
  tokenExtensionsProgram: PublicKey,
  systemProgram: PublicKey,
  auditorPublicKey: Buffer | Uint8Array
): Promise<TransactionInstruction> => {
  return await program.methods
    .initialize([...auditorPublicKey])
    .accountsStrict({
      tokenMint,
      confidentialMint,
      programAuthority,
      tokenVault,
      payer,
      tokenProgram,
      associatedTokenProgram,
      tokenExtensionsProgram,
      systemProgram,
    })
    .instruction();
};

export const wrap = async (
  tokenMint: PublicKey,
  tokenAccount: PublicKey,
  tokenVault: PublicKey,
  confidentialMint: PublicKey,
  confidentialTokenAccount: PublicKey,
  programAuthority: PublicKey,
  authority: PublicKey,
  payer: PublicKey,
  tokenProgram: PublicKey,
  tokenExtensionsProgram: PublicKey,
  amount: number
): Promise<TransactionInstruction> => {
  return await program.methods
    .wrap(amount)
    .accountsStrict({
      tokenMint,
      tokenAccount,
      tokenVault,
      confidentialMint,
      confidentialTokenAccount,
      programAuthority,
      authority,
      payer,
      tokenProgram,
      tokenExtensionsProgram,
    })
    .instruction();
};

export const unwrap = async (
  tokenMint: PublicKey,
  tokenAccount: PublicKey,
  tokenVault: PublicKey,
  confidentialMint: PublicKey,
  confidentialTokenAccount: PublicKey,
  programAuthority: PublicKey,
  authority: PublicKey,
  payer: PublicKey,
  tokenProgram: PublicKey,
  tokenExtensionsProgram: PublicKey,
  amount: number
): Promise<TransactionInstruction> => {
  return await program.methods
    .unwrap(amount)
    .accountsStrict({
      tokenMint,
      tokenAccount,
      tokenVault,
      confidentialMint,
      confidentialTokenAccount,
      programAuthority,
      authority,
      payer,
      tokenProgram,
      tokenExtensionsProgram,
    })
    .instruction();
};
