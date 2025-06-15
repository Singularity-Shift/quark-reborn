'use client';
import { quarkServer, tools } from '../services';
import { createContext, ReactNode, useContext } from 'react';

export type BackendContextProp = {
};

const BackendContext = createContext<BackendContextProp>(
  {} as BackendContextProp
);

export const BackendProvider = ({ children }: { children: ReactNode }) => {
  const values: BackendContextProp = {
  };

  return (
    <BackendContext.Provider value={values}>{children}</BackendContext.Provider>
  );
};

export const useBackend = () => {
  return useContext(BackendContext);
};
