"use client";

import { useWallet } from "@aptos-labs/wallet-adapter-react";
import { Button } from "@/components/ui/button";
import { Avatar } from "@/components/ui/avatar";
import { Wallet, User } from "lucide-react";
import {
  SshiftWallet,
  SshiftWalletDisconnect,
} from "@/components/SShiftWallet";

const truncateAddress = (address: string) => {
  if (!address) return "Unknown";
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

export const DaoWallet = () => {
  const { account, connected } = useWallet();

  // Only show when wallet is connected
  if (!connected) {
    return null;
  }

  return (
    <div className="flex items-center justify-between p-4 bg-white border-b border-gray-200">
      <div className="flex items-center space-x-3">
        <Avatar className="h-8 w-8 bg-green-600">
          <User className="h-4 w-4 text-white" />
        </Avatar>
        <div>
          <div className="text-sm font-medium text-gray-900">
            {account?.ansName ||
              truncateAddress(account?.address?.toString() || "")}
          </div>
          <div className="flex items-center space-x-2">
            <span className="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800">
              Connected
            </span>
            <span className="text-xs text-gray-500">
              {truncateAddress(account?.address?.toString() || "")}
            </span>
          </div>
        </div>
      </div>

      <SshiftWalletDisconnect />
    </div>
  );
};
