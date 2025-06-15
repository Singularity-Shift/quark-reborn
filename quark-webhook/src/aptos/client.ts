import {
  Aptos,
  AptosConfig,
  Ed25519PublicKey,
  Ed25519Signature,
  Network,
} from '@aptos-labs/ts-sdk';
import { APTOS_NETWORK } from '../config/env';
import { createSurfClient } from '@thalalabs/surf';

export const getAptosClient = (
    fullnode: string,
    indexer: string,
    network?: Network
  ) =>
    new Aptos(
      new AptosConfig({
        network:
          network || APTOS_NETWORK === 'mainnet'
            ? Network.MAINNET
            : Network.TESTNET,
        fullnode,
        indexer,
      })
    );
  
  export const abis = (fullnode: string, indexer: string) =>
    createSurfClient(getAptosClient(fullnode, indexer));