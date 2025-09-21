import * as anchor from "@coral-xyz/anchor";
import {
  Rpc,
  RpcSubscriptions,
  SolanaRpcApi,
  SolanaRpcSubscriptionsApi,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
} from "@solana/kit";

export type RpcClient = {
  rpc: Rpc<SolanaRpcApi>;
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  connection: anchor.web3.Connection;
};

export const createSolanaClient = ({
  endpoint,
  wsEndpoint,
}: {
  endpoint: string;
  wsEndpoint: string;
}): RpcClient => {
  const rpc = createSolanaRpc(endpoint);
  const connection = new anchor.web3.Connection(endpoint);
  const rpcSubscriptions = createSolanaRpcSubscriptions(wsEndpoint);
  return { rpc, rpcSubscriptions, connection };
};
