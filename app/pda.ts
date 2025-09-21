import * as anchor from "@coral-xyz/anchor";
import { findAssociatedTokenPda } from "@solana-program/token";
import { Address } from "@solana/kit";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";

export const programId = new anchor.web3.PublicKey("");

export const getLaunchPadConfigPda = () => {
  const [launchPadConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("launch_pad_config:")],
    programId
  );
  return launchPadConfigPda;
};

export const getVaultConfigPda = () => {
  const [vaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault:")],
    programId
  );
  return vaultPda;
};

export const getLaunchPadTokenPda = (tokenAddress: string) => {
  const mintPublicKey = new anchor.web3.PublicKey(tokenAddress);
  const [launchPadTokenPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("launch_pad_token:"), mintPublicKey.toBuffer()],
    programId
  );
  return launchPadTokenPda;
};

export const getGraduationVaultPda = (tokenAddress: string) => {
  const mintPublicKey = new anchor.web3.PublicKey(tokenAddress);
  const [graduationVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault_graduation:"), mintPublicKey.toBuffer()],
    programId
  );
  return graduationVaultPda;
};

export const getLaunchPadTokenATAPda = async (tokenAddress: string) => {
  const launchPadConfigPda = getLaunchPadTokenPda(tokenAddress);
  const mintPublicKey = new anchor.web3.PublicKey(tokenAddress);
  const [launchPadTokenAccountPda] = await findAssociatedTokenPda({
    /** The wallet address of the associated token account. */
    owner: launchPadConfigPda.toBase58() as Address,
    /** The address of the token program to use. */
    tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
    /** The mint address of the associated token account. */
    mint: mintPublicKey.toBase58() as Address,
  });
  return launchPadTokenAccountPda;
};

export const getInvestorTokenATAPda = async ({
  wallet,
  tokenAddress,
}: {
  wallet: string;
  tokenAddress: string;
}) => {
  const mint = new anchor.web3.PublicKey(tokenAddress);
  const walletPublicKey = new anchor.web3.PublicKey(wallet);
  const [launchPadTokenAccountPda] = await findAssociatedTokenPda({
    /** The wallet address of the associated token account. */
    owner: walletPublicKey.toBase58() as Address,
    /** The address of the token program to use. */
    tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
    /** The mint address of the associated token account. */
    mint: mint.toBase58() as Address,
  });
  return launchPadTokenAccountPda;
};
