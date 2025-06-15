"use client";
import { DefaultABITable } from "@thalalabs/surf";
import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useState,
} from "react";
import { Client } from "@thalalabs/surf/build/types/core/Client";
import { abis as surfClient } from "../aptos";
import { APTOS_INDEXER, APTOS_NODE_URL } from "../config/env";
import { useChain } from "./ChainProvider";
import { useWallet } from "@aptos-labs/wallet-adapter-react";

export type AbiContextProp = {
  abi: Client<DefaultABITable> | undefined;
};

const AbiContext = createContext<AbiContextProp>({} as AbiContextProp);

export const AbiProvider = ({ children }: { children: ReactNode }) => {
  const [abi, setAbi] = useState<Client<DefaultABITable>>();
  const { aptos } = useChain();
  const { connected, connect } = useWallet();

  useEffect(() => {
    if (!aptos) return;

    if (!connected) {
      connect("Continue with Google");
    }

    setAbi(
      surfClient(
        aptos.config?.fullnode || (APTOS_NODE_URL as string),
        aptos.config?.indexer || (APTOS_INDEXER as string)
      )
    );
  }, [aptos, connected]);

  const values = { abi };

  return <AbiContext.Provider value={values}>{children}</AbiContext.Provider>;
};

export const useAbiClient = () => {
  return useContext(AbiContext);
};
