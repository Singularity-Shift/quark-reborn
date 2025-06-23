"use client";

import { useWallet } from "@aptos-labs/wallet-adapter-react";
import {
  Section,
  Cell,
  Button,
  Text,
  Caption,
  Avatar,
  Badge,
} from "@telegram-apps/telegram-ui";
import { useState } from "react";
import toast from "react-hot-toast";

export const WalletNavbar = () => {
  const { account, connected, connect, disconnect } = useWallet();
  const [connecting, setConnecting] = useState(false);

  const handleConnect = async () => {
    try {
      setConnecting(true);
      await connect("Continue with Google");
      toast.success("Wallet connected successfully!");
    } catch (error) {
      console.error("Connection error:", error);
      toast.error("Failed to connect wallet");
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    try {
      await disconnect();
      toast.success("Wallet disconnected");
    } catch (error) {
      console.error("Disconnect error:", error);
      toast.error("Failed to disconnect wallet");
    }
  };

  const truncateAddress = (address: string) => {
    return `${address.slice(0, 6)}...${address.slice(-4)}`;
  };

  return (
    <div
      style={{
        backgroundColor: "var(--tg-theme-bg-color, #FFFFFF)",
        borderBottom:
          "1px solid var(--tg-theme-section-separator-color, #E1E3E6)",
        padding: "12px 16px",
        position: "sticky",
        top: 0,
        zIndex: 10,
        boxShadow: "0 1px 3px rgba(0,0,0,0.05)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          maxWidth: "100%",
        }}
      >
        {/* Left side - Wallet info */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "12px",
            flex: 1,
          }}
        >
          <Avatar
            size={28}
            style={{
              backgroundColor: connected
                ? "var(--tg-theme-button-color, #007AFF)"
                : "var(--tg-theme-hint-color, #999999)",
              color: "var(--tg-theme-button-text-color, #FFFFFF)",
            }}
          >
            <Text
              style={{ fontSize: "14px", fontWeight: "600", color: "inherit" }}
            >
              {connected ? "✓" : "W"}
            </Text>
          </Avatar>

          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <Text
                weight="2"
                style={{
                  color: "var(--tg-theme-text-color, #000000)",
                  fontSize: "16px",
                }}
              >
                {connected ? "Wallet" : "No Wallet"}
              </Text>
              {connected && (
                <Badge type="dot" mode="secondary" style={{ fontSize: "11px" }}>
                  Connected
                </Badge>
              )}
            </div>

            {connected && account?.address ? (
              <Caption
                style={{
                  color: "var(--tg-theme-hint-color, #999999)",
                  fontFamily: "monospace",
                  fontSize: "11px",
                  marginTop: "2px",
                }}
              >
                {truncateAddress(account.address.toString())}
              </Caption>
            ) : (
              <Caption
                style={{
                  color: "var(--tg-theme-hint-color, #999999)",
                  fontSize: "11px",
                  marginTop: "2px",
                }}
              >
                Not connected
              </Caption>
            )}
          </div>
        </div>

        {/* Right side - Action button */}
        <Button
          size="s"
          mode={connected ? "plain" : "filled"}
          onClick={connected ? handleDisconnect : handleConnect}
          disabled={connecting}
          style={{
            padding: "6px 12px",
            backgroundColor: connected
              ? "transparent"
              : "var(--tg-theme-button-color, #007AFF)",
            color: connected
              ? "var(--tg-theme-destructive-text-color, #FF3B30)"
              : "var(--tg-theme-button-text-color, #FFFFFF)",
            borderRadius: "6px",
            fontWeight: "600",
            fontSize: "12px",
            minWidth: "70px",
            border: connected
              ? "1px solid var(--tg-theme-destructive-text-color, #FF3B30)"
              : "none",
            flexShrink: 0,
          }}
        >
          {connecting ? "..." : connected ? "Disconnect" : "Connect"}
        </Button>
      </div>
    </div>
  );
};
