'use client';
import {
  createContext,
  Dispatch,
  ReactNode,
  SetStateAction,
  useContext,
  useEffect,
  useState,
  FC,
  PropsWithChildren,
} from 'react';
import { useAbiClient } from './AbiProvider';
import { useWallet } from '@aptos-labs/wallet-adapter-react';

export interface AppManagmentContextType {
}

export const AppManagmentContext = createContext<AppManagmentContextType>({
});

export const AppManagmentProvider: FC<PropsWithChildren> = ({ children }) => {


  const values = {};

  return (
    <AppManagmentContext.Provider value={values}>
      {children}
    </AppManagmentContext.Provider>
  );
};

export const useAppManagment = () => {
  return useContext(AppManagmentContext);
};
