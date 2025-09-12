import * as anchor from "@coral-xyz/anchor";
import {
  Commitment,
  TransactionMessageWithBlockhashLifetime,
  Rpc,
  RpcSubscriptions,
  SolanaRpcApi,
  SolanaRpcSubscriptionsApi,
  TransactionSigner,
  airdropFactory,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createTransactionMessage,
  generateKeyPairSigner,
  getSignatureFromTransaction,
  lamports,
  pipe,
  sendAndConfirmTransactionFactory,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
  appendTransactionMessageInstructions,
  Address,
  SOLANA_ERROR__JSON_RPC__SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE,
  isSolanaError,
  getBase58Encoder,
  getBase64EncodedWireTransaction,
} from "@solana/kit";
import { expect } from "chai";
import * as programClient from "../clients/js/src/generated";
import { getSetComputeUnitLimitInstruction } from "@solana-program/compute-budget";
import {
  BuyTokenInstructionDataArgs,
  CreateTokenInstructionDataArgs,
  InitializeInstructionDataArgs,
  SellTokenInstructionDataArgs,
} from "../clients/js/src/generated";
import {
  TOKEN_2022_PROGRAM_ID,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { findAssociatedTokenPda } from "@solana-program/token";

type TestEnvironment = {
  rpcClient: RpcClient;
  authority: TransactionSigner;
  creator: TransactionSigner;
  mint: TransactionSigner;
  programClient: typeof programClient;
};

const createTestEnvironment = async (): Promise<TestEnvironment> => {
  const rpcClient = createDefaultSolanaClient();
  const authority = await generateKeyPairSignerWithSol(
    rpcClient,
    10_000_000_000n
  );
  const creator = await generateKeyPairSignerWithSol(
    rpcClient,
    10_000_000_000n
  );
  let mint = await generateKeyPairSigner();
  // while (true) {
  //   const tokenArray = [NATIVE_MINT, new anchor.web3.PublicKey(mint.address)];
  //   if (tokenArray[0].toBuffer() > tokenArray[1].toBuffer()) {
  //     console.log(
  //       `Regenerating mint keypair to avoid token sort order issues: ${new anchor.web3.PublicKey(
  //         mint.address
  //       ).toBase58()} < ${NATIVE_MINT.toBase58()}`
  //     );
  //     break;
  //   }
  //   mint = await generateKeyPairSigner();
  // }

  return { rpcClient, authority, creator, mint, programClient };
};

type RpcClient = {
  rpc: Rpc<SolanaRpcApi>;
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  connection: anchor.web3.Connection;
};

const createDefaultSolanaClient = (): RpcClient => {
  const endpoint = "http://127.0.0.1:8899";
  const rpc = createSolanaRpc(endpoint);
  const connection = new anchor.web3.Connection(endpoint);
  const rpcSubscriptions = createSolanaRpcSubscriptions("ws://127.0.0.1:8900");
  return { rpc, rpcSubscriptions, connection };
};

const generateKeyPairSignerWithSol = async (
  rpcClient: RpcClient,
  putativeLamports: bigint = 1_000_000_000n
) => {
  const signer = await generateKeyPairSigner();
  await airdropFactory(rpcClient)({
    recipientAddress: signer.address,
    lamports: lamports(putativeLamports),
    commitment: "confirmed",
  });
  return signer;
};

const createDefaultTransaction = async (testEnv: TestEnvironment) => {
  const { rpcClient, authority: feePayer } = testEnv;
  const { value: latestBlockhash } = await rpcClient.rpc
    .getLatestBlockhash()
    .send();
  return pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(feePayer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx)
  );
};

const signAndSendTransaction = async (
  rpcClient: RpcClient,
  transactionMessage: any & TransactionMessageWithBlockhashLifetime,
  commitment: Commitment = "confirmed",
  skipPreflight: boolean = true
) => {
  try {
    const signedTransaction = await signTransactionMessageWithSigners(
      transactionMessage
    );
    const signature = getSignatureFromTransaction(signedTransaction);
    await sendAndConfirmTransactionFactory(rpcClient)(
      signedTransaction as any,
      {
        skipPreflight,
        commitment,
      }
    );
    return signature;
  } catch (e: any) {
    console.error("sendAndConfirmTransaction failed:", e?.message ?? e);

    // Try to extract signature from known error shapes
    const maybeSig =
      e?.signature ||
      e?.cause?.signature ||
      (e?.context && e?.context?.signature);

    // Try fetching transaction details (on-chain) to get logs
    if (maybeSig) {
      try {
        const txRes = await rpcClient.rpc
          .getTransaction(maybeSig, {
            commitment: "confirmed",
            encoding: "base58",
          })
          .send();
        const logs = txRes?.meta?.logMessages;
        if (logs && logs.length) {
          console.error("On-chain logs:");
          logs.forEach((l: string) => console.error(l));
        } else {
          console.error("getTransaction returned no logs", txRes?.meta);
        }
      } catch (getTxErr) {
        console.error(
          "Error fetching transaction via getTransaction:",
          getTxErr
        );
      }
    }

    // If we don't have a signature, try simulating the signed transaction to get logs
    try {
      const signed = await signTransactionMessageWithSigners(
        transactionMessage
      );

      const base64EncodedTransaction = getBase64EncodedWireTransaction(signed);

      // serialize to base64
      // const raw = (signed as any).serialize().toString("base64");
      const sim = await rpcClient.rpc
        .simulateTransaction(base64EncodedTransaction, { encoding: "base64" })
        .send();
      if (sim?.value?.logs && sim.value.logs.length) {
        console.error("Simulated logs:");
        sim.value.logs.forEach((l: string) => console.error(l));
      } else {
        console.error("Simulation returned no logs:", sim?.value);
      }
    } catch (simErr) {
      console.error("Simulation attempt failed:", simErr);
    }

    // rethrow so tests still see the failure
    throw e;
  }
};

const displaySolanaError = (e: any) => {
  if (
    isSolanaError(
      e,
      SOLANA_ERROR__JSON_RPC__SERVER_ERROR_SEND_TRANSACTION_PREFLIGHT_FAILURE
    )
  ) {
    const preflightErrorContext = e.context;
    const preflightErrorMessage = e.message;
    const errorDetailMessage = e.cause?.message
      ? e?.cause?.message
      : e?.cause?.context;
    if (errorDetailMessage !== undefined) {
      console.error(
        "%O %s: %s",
        preflightErrorContext,
        preflightErrorMessage,
        errorDetailMessage
      );
    } else {
      console.error("%O %s", preflightErrorContext, preflightErrorMessage);
    }
  } else {
    throw e;
  }
};

describe("Launch Pad Fun", () => {
  let testEnv: TestEnvironment;

  before(async () => {
    testEnv = await createTestEnvironment();
  });

  it("initializes the launch pad config PDA and vault", async () => {
    // const authority = new anchor.web3.Keypair();
    const { rpcClient, programClient: program, authority } = testEnv;
    const programId = new anchor.web3.PublicKey(
      program.LAUNCHPAD_FUN_PROGRAM_ADDRESS
    );

    // derive PDAs
    const [launchPadConfigPda, launchPadConfigBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("launch_pad_config:")],
        programId
      );

    const [vaultPda, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault:")],
      programId
    );

    // prepare args
    const args = {
      assetRate: 300000n,
      creatorSellDelay: BigInt(60 * 60 * 1), // 1 hour
      graduateThreshold: 85_000_000_000n, // 100 SOLs
      protocolBuyFee: 5000, // basis points
      protocolSellFee: 7000,
    } as InitializeInstructionDataArgs;

    // call initialize
    const ix = await program.getInitializeInstructionAsync({
      authority: authority,
      ...args,
    });

    const instructions = [
      getSetComputeUnitLimitInstruction({ units: 200_000 }), // <- Here we add the CU limit instruction.
      ix,
    ];

    const txSignature = await pipe(
      await createDefaultTransaction(testEnv),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
      (tx) => signAndSendTransaction(testEnv.rpcClient, tx, "confirmed", false)
    );
    console.log("tx", txSignature.toString());

    let cfg = await testEnv.programClient.fetchLaunchPadConfig(
      testEnv.rpcClient.rpc,
      launchPadConfigPda.toString() as Address,
      { commitment: "confirmed" }
    );

    expect(cfg.data.authority.toString()).to.equal(
      authority.address.toString()
    );
    expect(cfg.data.assetRate).to.equal(args.assetRate);
    expect(cfg.data.protocolBuyFee).to.equal(args.protocolBuyFee);
    expect(cfg.data.protocolSellFee).to.equal(args.protocolSellFee);
    expect(cfg.data.creatorSellDelay).to.equal(args.creatorSellDelay);
    expect(cfg.data.graduateThreshold).to.equal(args.graduateThreshold);
    expect(cfg.data.status).to.equal(1); // ProtocolStatus::Active (enum idx)
    expect(cfg.data.bump).to.equal(launchPadConfigBump);
    expect(cfg.data.vaultBump).to.equal(vaultBump);

    // verify vault exists and has rent-exempt lamports
    const vaultAcct = await rpcClient.rpc
      .getAccountInfo(vaultPda.toString() as Address)
      .send();

    expect(vaultAcct).to.not.be.null;
    // has some lamports deposited (rent exempt at least)
    expect(BigInt(vaultAcct.value.lamports.toString())).to.equal(890880n);
  });

  it("creates a token", async () => {
    // const authority = new anchor.web3.Keypair();
    const { rpcClient, programClient: program, creator, mint } = testEnv;
    const programId = new anchor.web3.PublicKey(
      program.LAUNCHPAD_FUN_PROGRAM_ADDRESS
    );

    const codec = getBase58Encoder();
    const mintAddressBytes = codec.encode(mint.address.toString());

    const [launchPadTokenPda, launchPadTokenBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("launch_pad_token:"), Buffer.from(mintAddressBytes)],
        programId
      );

    const [graduationVaultPda, graduationVaultBump] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault_graduation:"), Buffer.from(mintAddressBytes)],
        programId
      );

    const [launchPadConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("launch_pad_config:")],
      programId
    );

    const [launchPadTokenAccountPda] = await findAssociatedTokenPda({
      /** The wallet address of the associated token account. */
      owner: launchPadConfigPda.toBase58() as Address,
      /** The address of the token program to use. */
      tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
      /** The mint address of the associated token account. */
      mint: mint.address,
    });

    // prepare args
    const args = {
      name: "New Meme Token",
      symbol: "NMT",
      uri: "https://example.com/nmt.json",
    } as CreateTokenInstructionDataArgs;

    // call initialize
    const ix = await program.getCreateTokenInstructionAsync({
      creator: creator,
      mint: mint,
      ...args,
    });

    const instructions = [
      getSetComputeUnitLimitInstruction({ units: 200_000 }), // <- Here we add the CU limit instruction.
      ix,
    ];

    const txSignature = await pipe(
      await createDefaultTransaction(testEnv),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
      (tx) => signAndSendTransaction(testEnv.rpcClient, tx, "confirmed", false)
    );
    console.log("tx", txSignature.toString());

    let token = await testEnv.programClient.fetchLaunchPadToken(
      testEnv.rpcClient.rpc,
      launchPadTokenPda.toString() as Address,
      { commitment: "confirmed" }
    );

    expect(token.data.creator.toString()).to.equal(creator.address.toString());
    expect(token.data.mint.toString()).to.equal(mint.address.toString());
    expect(token.data.virtualAssetAmount).to.equal(100000000000n);
    expect(token.data.virtualTokenAmount).to.equal(1000000000000000000n);
    expect(token.data.currentK).to.equal(100000000000000000000000000000n);
    expect(token.data.virtualGraduationAmount).to.equal(0n);
    expect(token.data.status).to.equal(1); // LaunchPadTokenStatus::TradingEnabled (enum idx)
    expect(token.data.bump).to.equal(launchPadTokenBump);
    expect(token.data.vaultBump).to.equal(graduationVaultBump);

    // verify vault exists and has rent-exempt lamports
    const graduationVault = await rpcClient.rpc
      .getAccountInfo(graduationVaultPda.toString() as Address)
      .send();

    expect(graduationVault).to.not.be.null;
    // has some lamports deposited (rent exempt at least)
    expect(BigInt(graduationVault.value.lamports.toString())).to.equal(890880n);

    const launchPadTokenAccount = await rpcClient.rpc
      .getTokenAccountBalance(launchPadTokenAccountPda.toString() as Address)
      .send();

    expect(launchPadTokenAccount).to.not.be.null;
    expect(launchPadTokenAccount.value.amount).to.equal("1000000000000000000");
  });

  it("buys a token", async () => {
    const { rpcClient, programClient: program, creator, mint } = testEnv;
    const programId = new anchor.web3.PublicKey(
      program.LAUNCHPAD_FUN_PROGRAM_ADDRESS
    );

    const codec = getBase58Encoder();
    const mintAddressBytes = codec.encode(mint.address.toString());

    const [launchPadTokenPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("launch_pad_token:"), Buffer.from(mintAddressBytes)],
      programId
    );

    const [graduationVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_graduation:"), Buffer.from(mintAddressBytes)],
      programId
    );

    const [vaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault:")],
      programId
    );

    const [launchPadConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("launch_pad_config:")],
      programId
    );

    const [launchPadTokenAccountPda] = await findAssociatedTokenPda({
      /** The wallet address of the associated token account. */
      owner: launchPadConfigPda.toBase58() as Address,
      /** The address of the token program to use. */
      tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
      /** The mint address of the associated token account. */
      mint: mint.address,
    });

    const [investorTokenAccountPda] = await findAssociatedTokenPda({
      /** The wallet address of the associated token account. */
      owner: creator.address,
      /** The address of the token program to use. */
      tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
      /** The mint address of the associated token account. */
      mint: mint.address,
    });

    // prepare args
    const args = {
      amount: 1_000_000_000n,
    } as BuyTokenInstructionDataArgs;

    const ix = await program.getBuyTokenInstructionAsync({
      investor: creator,
      mint: mint.address,
      ...args,
    });

    const instructions = [
      getSetComputeUnitLimitInstruction({ units: 200_000 }), // <- Here we add the CU limit instruction.
      ix,
    ];
    const txSignature = await pipe(
      await createDefaultTransaction(testEnv),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
      (tx) => signAndSendTransaction(testEnv.rpcClient, tx, "confirmed", false)
    );
    console.log("tx", txSignature.toString());

    let token = await testEnv.programClient.fetchLaunchPadToken(
      testEnv.rpcClient.rpc,
      launchPadTokenPda.toString() as Address,
      { commitment: "confirmed" }
    );

    expect(token.data.creator.toString()).to.equal(creator.address.toString());
    expect(token.data.mint.toString()).to.equal(mint.address.toString());
    expect(token.data.virtualAssetAmount).to.equal(100995000000n);
    expect(token.data.virtualTokenAmount).to.equal(990148027130055943n);
    expect(token.data.currentK).to.equal(100000000000000000000000000000n);
    expect(token.data.virtualGraduationAmount).to.equal(995000000n);
    expect(token.data.status).to.equal(1); // LaunchPadTokenStatus::TradingEnabled (enum idx)

    const graduationVault = await rpcClient.rpc
      .getAccountInfo(graduationVaultPda.toString() as Address)
      .send();

    expect(graduationVault).to.not.be.null;

    expect(BigInt(graduationVault.value.lamports.toString())).to.equal(
      995_890_880n
    );

    const launchPadTokenAccount = await rpcClient.rpc
      .getTokenAccountBalance(launchPadTokenAccountPda.toString() as Address)
      .send();

    expect(launchPadTokenAccount).to.not.be.null;
    expect(launchPadTokenAccount.value.amount).to.equal("990148027130055943");

    // verify vault exists and has rent-exempt lamports
    const vault = await rpcClient.rpc
      .getAccountInfo(vaultPda.toString() as Address)
      .send();

    expect(vault).to.not.be.null;
    expect(BigInt(vault.value.lamports.toString())).to.equal(5_890_880n);

    const investorTokenAccount = await rpcClient.rpc
      .getTokenAccountBalance(investorTokenAccountPda.toString() as Address)
      .send();

    expect(investorTokenAccount).to.not.be.null;
    expect(investorTokenAccount.value.amount).to.equal("9851972869944057"); //9.851.972,869944057
  });

  it("sells a token", async () => {
    const { rpcClient, programClient: program, creator, mint } = testEnv;
    const programId = new anchor.web3.PublicKey(
      program.LAUNCHPAD_FUN_PROGRAM_ADDRESS
    );

    const codec = getBase58Encoder();
    const mintAddressBytes = codec.encode(mint.address.toString());

    const [launchPadTokenPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("launch_pad_token:"), Buffer.from(mintAddressBytes)],
      programId
    );

    const [graduationVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_graduation:"), Buffer.from(mintAddressBytes)],
      programId
    );

    const [vaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault:")],
      programId
    );

    const [launchPadConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("launch_pad_config:")],
      programId
    );

    const [launchPadTokenAccountPda] = await findAssociatedTokenPda({
      /** The wallet address of the associated token account. */
      owner: launchPadConfigPda.toBase58() as Address,
      /** The address of the token program to use. */
      tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
      /** The mint address of the associated token account. */
      mint: mint.address,
    });

    const [investorTokenAccountPda] = await findAssociatedTokenPda({
      /** The wallet address of the associated token account. */
      owner: creator.address,
      /** The address of the token program to use. */
      tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
      /** The mint address of the associated token account. */
      mint: mint.address,
    });

    // prepare args
    const args = {
      amount: 9_000_000_869_944_000n,
    } as SellTokenInstructionDataArgs;

    const ix = await program.getSellTokenInstructionAsync({
      investor: creator,
      mint: mint.address,
      ...args,
    });

    const instructions = [
      getSetComputeUnitLimitInstruction({ units: 200_000 }), // <- Here we add the CU limit instruction.
      ix,
    ];
    const txSignature = await pipe(
      await createDefaultTransaction(testEnv),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
      (tx) => signAndSendTransaction(testEnv.rpcClient, tx, "confirmed", false)
    );
    console.log("tx", txSignature.toString());

    let token = await testEnv.programClient.fetchLaunchPadToken(
      testEnv.rpcClient.rpc,
      launchPadTokenPda.toString() as Address,
      { commitment: "confirmed" }
    );

    expect(token.data.creator.toString()).to.equal(creator.address.toString());
    expect(token.data.mint.toString()).to.equal(mint.address.toString());
    expect(token.data.virtualAssetAmount).to.equal(100085269847n);
    expect(token.data.virtualTokenAmount).to.equal(999148027999999943n);
    expect(token.data.currentK).to.equal(100000000000000000000000000000n);
    expect(token.data.virtualGraduationAmount).to.equal(85269847n);
    expect(token.data.status).to.equal(1); // LaunchPadTokenStatus::TradingEnabled (enum idx)

    const graduationVault = await rpcClient.rpc
      .getAccountInfo(graduationVaultPda.toString() as Address)
      .send();

    expect(graduationVault).to.not.be.null;

    expect(BigInt(graduationVault.value.lamports.toString())).to.equal(
      86_160_727n
    );

    const launchPadTokenAccount = await rpcClient.rpc
      .getTokenAccountBalance(launchPadTokenAccountPda.toString() as Address)
      .send();

    expect(launchPadTokenAccount).to.not.be.null;
    expect(launchPadTokenAccount.value.amount).to.equal("999148027999999943");

    // verify vault exists and has rent-exempt lamports
    const vault = await rpcClient.rpc
      .getAccountInfo(vaultPda.toString() as Address)
      .send();

    expect(vault).to.not.be.null;
    expect(BigInt(vault.value.lamports.toString())).to.equal(12_258_991n);

    const investorTokenAccount = await rpcClient.rpc
      .getTokenAccountBalance(investorTokenAccountPda.toString() as Address)
      .send();

    expect(investorTokenAccount).to.not.be.null;
    expect(investorTokenAccount.value.amount).to.equal("851972000000057"); //851.972,000000057

    const investor = await rpcClient.rpc.getAccountInfo(creator.address).send();

    expect(investor).to.not.be.null;
    expect(BigInt(investor.value.lamports.toString())).to.equal(9_893_151_722n);
  });

  it("buys a token and graduate", async () => {
    const { rpcClient, programClient: program, creator, mint } = testEnv;
    const programId = new anchor.web3.PublicKey(
      program.LAUNCHPAD_FUN_PROGRAM_ADDRESS
    );

    await airdropFactory(rpcClient)({
      recipientAddress: creator.address,
      lamports: lamports(110_000_000_000n),
      commitment: "confirmed",
    });

    const codec = getBase58Encoder();
    const mintAddressBytes = codec.encode(mint.address.toString());

    const [launchPadTokenPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("launch_pad_token:"), Buffer.from(mintAddressBytes)],
      programId
    );

    const [graduationVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_graduation:"), Buffer.from(mintAddressBytes)],
      programId
    );

    const [vaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault:")],
      programId
    );

    const [launchPadConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("launch_pad_config:")],
      programId
    );

    const [launchPadTokenAccountPda] = await findAssociatedTokenPda({
      /** The wallet address of the associated token account. */
      owner: launchPadConfigPda.toBase58() as Address,
      /** The address of the token program to use. */
      tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
      /** The mint address of the associated token account. */
      mint: mint.address,
    });

    // const [investorTokenAccountPda] = await findAssociatedTokenPda({
    //   /** The wallet address of the associated token account. */
    //   owner: creator.address,
    //   /** The address of the token program to use. */
    //   tokenProgram: TOKEN_2022_PROGRAM_ID.toBase58() as Address,
    //   /** The mint address of the associated token account. */
    //   mint: mint.address,
    // });

    // const raydiumCpmmProgramId = new anchor.web3.PublicKey(
    //   "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"
    // );
    // const ammConfigPda = new anchor.web3.PublicKey(
    //   "D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2"
    // );

    const raydiumCpmmProgramId = new anchor.web3.PublicKey(
      "DRaycpLY18LhpbydsBWbVJtxpNv9oXPgjRSfpF2bWpYb"
    );
    const ammConfigPda = new anchor.web3.PublicKey(
      "A9qBhPy4k5UYW72hSgAkh1Epr2do69P54yzzcMV3yv6b"
    );

    const tokenArray = [NATIVE_MINT, new anchor.web3.PublicKey(mint.address)];
    tokenArray.sort((a, b) => {
      const bufferA = a.toBuffer();
      const bufferB = b.toBuffer();
      return Buffer.compare(bufferA, bufferB);
    });
    const token0Mint = tokenArray[0];
    const token1Mint = tokenArray[1];

    const [poolStatePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
        ammConfigPda.toBuffer(),
        token0Mint.toBuffer(),
        token1Mint.toBuffer(),
      ],
      raydiumCpmmProgramId
    );

    const [token0VaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool_vault"),
        poolStatePda.toBuffer(),
        token0Mint.toBuffer(),
      ],
      raydiumCpmmProgramId
    );

    const [token1VaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool_vault"),
        poolStatePda.toBuffer(),
        token1Mint.toBuffer(),
      ],
      raydiumCpmmProgramId
    );
    const [lpMintPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("pool_lp_mint"), poolStatePda.toBuffer()],
      raydiumCpmmProgramId
    );

    const creatorAddressBytes = codec.encode(creator.address.toString());
    const [lpTokenPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from(creatorAddressBytes), // owner
        TOKEN_PROGRAM_ID.toBuffer(),
        lpMintPda.toBuffer(),
      ],
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const graduateIx = await program.getGraduateToRaydiumInstructionAsync({
      investor: creator,
      ammConfig: ammConfigPda.toBase58() as Address,
      poolState: poolStatePda.toBase58() as Address,
      lpToken: lpTokenPda.toBase58() as Address,
      token0Vault: token0VaultPda.toBase58() as Address,
      token1Vault: token1VaultPda.toBase58() as Address,
      mint: mint.address,
      wsolMint: NATIVE_MINT.toBase58() as Address,
      // wsolMint: mint.address,
      // mint: NATIVE_MINT.toBase58() as Address,
    });

    // prepare args
    const args = {
      amount: 110_000_000_000n,
    } as BuyTokenInstructionDataArgs;

    const buyTokenIx = await program.getBuyTokenInstructionAsync({
      investor: creator,
      mint: mint.address,
      ...args,
    });

    const instructions = [
      getSetComputeUnitLimitInstruction({ units: 600_000 }), // <- Here we add the CU limit instruction.
      buyTokenIx,
      graduateIx,
    ];
    const txSignature = await pipe(
      await createDefaultTransaction(testEnv),
      (tx) => appendTransactionMessageInstructions(instructions, tx),
      (tx) => signAndSendTransaction(testEnv.rpcClient, tx, "confirmed", false)
    );
    console.log("tx", txSignature.toString());

    let token = await testEnv.programClient.fetchLaunchPadToken(
      testEnv.rpcClient.rpc,
      launchPadTokenPda.toString() as Address,
      { commitment: "confirmed" }
    );

    expect(token.data.creator.toString()).to.equal(creator.address.toString());
    expect(token.data.mint.toString()).to.equal(mint.address.toString());
    expect(token.data.virtualAssetAmount).to.equal(209535269847n);
    expect(token.data.virtualTokenAmount).to.equal(477246623315581827n);
    expect(token.data.currentK).to.equal(100000000000000000000000000000n);
    expect(token.data.virtualGraduationAmount).to.equal(109535269847n);
    expect(token.data.status).to.equal(3); // LaunchPadTokenStatus::Graduated (enum idx)

    const graduationVault = await rpcClient.rpc
      .getAccountInfo(graduationVaultPda.toString() as Address)
      .send();

    expect(graduationVault.value).to.be.null;

    const launchPadTokenAccount = await rpcClient.rpc
      .getAccountInfo(launchPadTokenAccountPda.toString() as Address)
      .send();

    expect(launchPadTokenAccount.value).to.be.null;

    const vault = await rpcClient.rpc
      .getAccountInfo(vaultPda.toString() as Address)
      .send();

    expect(vault.value).to.not.be.null;
    expect(BigInt(vault.value.lamports.toString())).to.equal(564_333_071n);

    // Raydium info

    const token0VaultPdaAccount = await rpcClient.rpc
      .getTokenAccountBalance(token0VaultPda.toString() as Address)
      .send();

    expect(token0VaultPdaAccount?.value).to.not.be.null;
    expect(token0VaultPdaAccount.value.amount).to.equal("109536160727");

    const token1VaultPdaAccount = await rpcClient.rpc
      .getTokenAccountBalance(token1VaultPda.toString() as Address)
      .send();

    expect(token1VaultPdaAccount?.value).to.not.be.null;
    expect(token1VaultPdaAccount.value.amount).to.equal("477246623315581827");

    const lpTokenPdaAccount = await rpcClient.rpc
      .getTokenAccountBalance(lpTokenPda.toString() as Address)
      .send();

    expect(lpTokenPdaAccount?.value).to.not.be.null;
    expect(lpTokenPdaAccount.value.amount).to.equal("228638935524699");
  });
});
