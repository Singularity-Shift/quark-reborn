'use client';
import { getAptosClient } from '../aptos';
import { Aptos } from '@aptos-labs/ts-sdk';
import {
  APTOS_INDEXER,
  APTOS_NODE_URL,
} from '../config/env';
import { createContext, useContext, useEffect, useState } from 'react';

type ChainProviderContextProp = {
  aptos: Aptos;
  createChainClient: () => void;
};

const ChainProviderContext = createContext<ChainProviderContextProp>(
  {} as ChainProviderContextProp
);

export const ChainProvider = ({ children }: { children: React.ReactNode }) => {
  const [aptos, setAptos] = useState<Aptos>({} as Aptos);

  useEffect(() => {
    const fullnode = APTOS_NODE_URL;
    const indexer = APTOS_INDEXER;

    setAptos(getAptosClient(fullnode as string, indexer as string));
  }, []);

  const createChainClient = () => {
    const fullnode = APTOS_NODE_URL;
    const indexer = APTOS_INDEXER;

    setAptos(getAptosClient(fullnode as string, indexer as string));
  };

  return (
    <ChainProviderContext.Provider value={{ aptos, createChainClient }}>
      {children}
    </ChainProviderContext.Provider>
  );
};

export const useChain = () => {
  return useContext(ChainProviderContext);
};
