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
} from "@solana/kit";
import { expect } from "chai";
import * as programClient from "../clients/js/src/generated";
import { getSetComputeUnitLimitInstruction } from "@solana-program/compute-budget";
import {
  CreateLaunchPadTokenInstructionDataArgs,
  InitializeInstructionDataArgs,
} from "../clients/js/src/generated";
import {
  getAccount,
  getAssociatedTokenAddressSync,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
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
  const authority = await generateKeyPairSignerWithSol(rpcClient);
  const creator = await generateKeyPairSignerWithSol(rpcClient);
  const mint = await generateKeyPairSigner();
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
  const signedTransaction = await signTransactionMessageWithSigners(
    transactionMessage
  );
  const signature = getSignatureFromTransaction(signedTransaction);
  await sendAndConfirmTransactionFactory(rpcClient)(signedTransaction as any, {
    skipPreflight,
    commitment,
  });
  return signature;
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

describe("init_launch_pad_config", () => {
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
      assetRate: 7n,
      creatorSellDelay: BigInt(Math.floor(Date.now() / 1000) + 60 * 60 * 1), // 1 hour in future
      graduateThreshold: 50n,
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
    try {
      const txSignature = await pipe(
        await createDefaultTransaction(testEnv),
        (tx) => appendTransactionMessageInstructions(instructions, tx),
        (tx) => signAndSendTransaction(testEnv.rpcClient, tx)
      );
      console.log("tx", txSignature.toString());
    } catch (e) {
      if (
        testEnv.programClient.isLaunchpadFunError(e, {
          instructions,
        })
      ) {
        console.error(
          "Program error:",
          e,
          testEnv.programClient.getLaunchpadFunErrorMessage(e.context.code)
        );
      } else {
        displaySolanaError(e);
      }
    }

    let cfg = await testEnv.programClient.fetchLaunchPadConfig(
      testEnv.rpcClient.rpc,
      launchPadConfigPda.toString() as Address,
      { commitment: "confirmed" }
    );

    expect(cfg.data.authority.toString()).to.equal(
      authority.address.toString()
    );
    expect(cfg.data.assetRate).to.equal(7n);
    expect(cfg.data.protocolBuyFee).to.equal(5000);
    expect(cfg.data.protocolSellFee).to.equal(7000);
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

  it("creates a launch pad token", async () => {
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
    console.log(
      "launchPadTokenAccountPda",
      launchPadTokenAccountPda.toString()
    );

    // prepare args
    const args = {
      name: "New Meme Token",
      symbol: "NMT",
      uri: "https://example.com/nmt.json",
    } as CreateLaunchPadTokenInstructionDataArgs;

    // call initialize
    const ix = await program.getCreateLaunchPadTokenInstructionAsync({
      creator: creator,
      mint: mint,
      ...args,
    });

    const instructions = [
      getSetComputeUnitLimitInstruction({ units: 200_000 }), // <- Here we add the CU limit instruction.
      ix,
    ];
    try {
      const txSignature = await pipe(
        await createDefaultTransaction(testEnv),
        (tx) => appendTransactionMessageInstructions(instructions, tx),
        (tx) => signAndSendTransaction(testEnv.rpcClient, tx, "confirmed")
      );
      console.log("tx", txSignature.toString());
    } catch (e) {
      if (
        testEnv.programClient.isLaunchpadFunError(e, {
          instructions,
        })
      ) {
        console.error(
          "Program error:",
          e,
          testEnv.programClient.getLaunchpadFunErrorMessage(e.context.code)
        );
      } else {
        displaySolanaError(e);
      }
    }

    let token = await testEnv.programClient.fetchLaunchPadToken(
      testEnv.rpcClient.rpc,
      launchPadTokenPda.toString() as Address,
      { commitment: "confirmed" }
    );

    expect(token.data.creator.toString()).to.equal(creator.address.toString());
    expect(token.data.mint.toString()).to.equal(mint.address.toString());
    expect(token.data.virtualAssetAmount).to.equal(4285714285700000n);
    expect(token.data.virtualTokenAmount).to.equal(1000000000000000000n);
    expect(token.data.currentK).to.equal(4285714285700000000000000000000000n);
    expect(token.data.virtualGraduationAmount).to.equal(0n);
    expect(token.data.status).to.equal(1); // ProtocolStatus::Active (enum idx)
    expect(token.data.bump).to.equal(launchPadTokenBump);
    expect(token.data.vaultBump).to.equal(graduationVaultBump);

    // verify vault exists and has rent-exempt lamports
    const graduationVault = await rpcClient.rpc
      .getAccountInfo(graduationVaultPda.toString() as Address)
      .send();

    expect(graduationVault).to.not.be.null;
    // has some lamports deposited (rent exempt at least)
    expect(BigInt(graduationVault.value.lamports.toString())).to.equal(890880n);

    const launchPadTokenAccount = await rpcClient.rpc.getTokenAccountBalance(
      launchPadTokenAccountPda.toString() as Address
    ).send();

    expect(launchPadTokenAccount).to.not.be.null;
    expect(launchPadTokenAccount.value.amount).to.equal('1000000000000000000');
  });
});
