import * as anchor from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

const raydiumCpmmProgramId = new anchor.web3.PublicKey(
  "DRaycpLY18LhpbydsBWbVJtxpNv9oXPgjRSfpF2bWpYb" // Raydium CPMM Program ID Devnet
);

export const sortTokenMints = ({
  mintA,
  mintB,
}: {
  mintA: string;
  mintB: string;
}) => {
  const tokenArray = [
    new anchor.web3.PublicKey(mintA),
    new anchor.web3.PublicKey(mintB),
  ];
  tokenArray.sort((a, b) => {
    const bufferA = a.toBuffer();
    const bufferB = b.toBuffer();
    return Buffer.compare(bufferA, bufferB);
  });
  const token0Mint = tokenArray[0];
  const token1Mint = tokenArray[1];
  return { token0Mint, token1Mint };
};

export const getPoolStatePda = ({
  mintA,
  mintB,
  ammConfigPda,
}: {
  mintA: string;
  mintB: string;
  ammConfigPda: string;
}) => {
  const { token0Mint, token1Mint } = sortTokenMints({ mintA, mintB });
  const ammConfigPubKey = new anchor.web3.PublicKey(ammConfigPda);
  const [poolStatePda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("pool"),
      ammConfigPubKey.toBuffer(),
      token0Mint.toBuffer(),
      token1Mint.toBuffer(),
    ],
    raydiumCpmmProgramId
  );
  return poolStatePda;
};

export const getPoolTokenVaultsPda = ({
  mintA,
  mintB,
  ammConfigPda,
}: {
  mintA: string;
  mintB: string;
  ammConfigPda: string;
}) => {
  const { token0Mint, token1Mint } = sortTokenMints({ mintA, mintB });
  const poolStatePda = getPoolStatePda({ mintA, mintB, ammConfigPda });
  const [token0VaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("pool_vault"), poolStatePda.toBuffer(), token0Mint.toBuffer()],
    raydiumCpmmProgramId
  );

  const [token1VaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("pool_vault"), poolStatePda.toBuffer(), token1Mint.toBuffer()],
    raydiumCpmmProgramId
  );
  return { token0VaultPda, token1VaultPda };
};

export const getPoolLpMintPda = ({
  mintA,
  mintB,
  ammConfigPda,
}: {
  mintA: string;
  mintB: string;
  ammConfigPda: string;
}) => {
  const poolStatePda = getPoolStatePda({ mintA, mintB, ammConfigPda });
  const [lpMintPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("pool_lp_mint"), poolStatePda.toBuffer()],
    raydiumCpmmProgramId
  );
  return lpMintPda;
};

export const getInvestorLpTokenPda = ({
  investor,
  mintA,
  mintB,
  ammConfigPda,
}: {
  investor: string;
  mintA: string;
  mintB: string;
  ammConfigPda: string;
}) => {
  const lpMintPda = getPoolLpMintPda({ mintA, mintB, ammConfigPda });
  const investorPubKey = new anchor.web3.PublicKey(investor);
  const [lpTokenPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      investorPubKey.toBuffer(), // owner
      TOKEN_PROGRAM_ID.toBuffer(),
      lpMintPda.toBuffer(),
    ],
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
  return lpTokenPda;
};
