"use client";

import { useWallet } from "@aptos-labs/wallet-adapter-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Avatar } from "@/components/ui/avatar";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Copy, LogOut, Wallet, User } from "lucide-react";
import { useToast } from "@/components/ui/use-toast";

const truncateAddress = (address: string) => {
  if (!address) return "Unknown";
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
};

export const DaoWallet = () => {
  const { account, connected, connect, disconnect } = useWallet();
  const [connecting, setConnecting] = useState(false);
  const { toast } = useToast();

  const handleConnect = async () => {
    try {
      setConnecting(true);
      await connect("Continue with Google");
      toast({
        title: "Success",
        description: "Wallet connected successfully!",
      });
    } catch (error) {
      console.error("Connection error:", error);
      toast({
        variant: "destructive",
        title: "Error",
        description: "Failed to connect wallet",
      });
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    try {
      await disconnect();
      toast({
        title: "Success",
        description: "Wallet disconnected",
      });
    } catch (error) {
      console.error("Disconnect error:", error);
      toast({
        variant: "destructive",
        title: "Error",
        description: "Failed to disconnect wallet",
      });
    }
  };

  const copyAddress = async () => {
    if (!account?.address) return;
    try {
      await navigator.clipboard.writeText(account.address.toString());
      toast({
        title: "Success",
        description: "Address copied to clipboard",
      });
    } catch (error) {
      toast({
        variant: "destructive",
        title: "Error",
        description: "Failed to copy address",
      });
    }
  };

  if (!connected) {
    return (
      <div className="flex items-center justify-between p-4 bg-white border-b border-gray-200">
        <div className="flex items-center space-x-3">
          <Avatar className="h-8 w-8 bg-blue-600">
            <Wallet className="h-4 w-4 text-white" />
          </Avatar>
          <div>
            <div className="text-sm font-medium text-gray-900">DAO Voting</div>
            <div className="text-xs text-gray-500">Connect wallet to vote</div>
          </div>
        </div>
        <Button
          onClick={handleConnect}
          disabled={connecting}
          className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg"
        >
          {connecting ? "Connecting..." : "Connect Wallet"}
        </Button>
      </div>
    );
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

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="sm" className="h-8 w-8 p-0">
            <User className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-48">
          <DropdownMenuItem onClick={copyAddress}>
            <Copy className="mr-2 h-4 w-4" />
            Copy Address
          </DropdownMenuItem>
          <DropdownMenuItem onClick={handleDisconnect}>
            <LogOut className="mr-2 h-4 w-4" />
            Disconnect
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
};
