// components/AccountStatus.tsx
"use client";
import { useState } from "react";
import { truncateAddress, useWallet } from "@aptos-labs/wallet-adapter-react";
import { closeMiniApp, hapticFeedback } from "@telegram-apps/sdk-react";
import { Message } from "@/components/Message/Message";
import { useMessage } from "@/hooks/useMessage";
import {
  Button,
  Card,
  Section,
  Cell,
  Text,
  Title,
  Subheadline,
  Caption,
  Avatar,
  Badge,
} from "@telegram-apps/telegram-ui";

interface AccountStatusProps {
  txHash?: string;
  resourceAddress: string;
  onClose?: () => void;
}

export const AccountStatus = ({
  txHash,
  resourceAddress,
}: AccountStatusProps) => {
  const { connected, account } = useWallet();
  const [copied, setCopied] = useState(false);
  const { message, showMessage } = useMessage();

  const isReady = connected && account?.address;

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      hapticFeedback.notificationOccurred("success");
      showMessage("Copied to clipboard!", "success");
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      hapticFeedback.notificationOccurred("error");
      showMessage("Failed to copy", "error");
    }
  };

  const closeApp = () => {
    hapticFeedback.impactOccurred("light");
    closeMiniApp();
  };

  if (!isReady) {
    return (
      <Section className="min-h-screen flex flex-col items-center justify-center">
        <div className="flex flex-col items-center">
          <div
            className="w-20 h-20 border-4 rounded-full animate-spin"
            style={{
              marginBottom: "24px",
            }}
          />
          <Title style={{ marginBottom: "8px" }}>Connecting to Quark AI</Title>
          <Caption>Please wait while we set up your AI wallet...</Caption>
        </div>
      </Section>
    );
  }

  return (
    <div className="min-h-screen">
      <Message message={message} />
      {/* Header Section */}
      <Section>
        <div className="text-center" style={{ padding: "32px 0" }}>
          <Title style={{ marginBottom: "8px" }}>Quark AI Ready! 🚀</Title>

          <Subheadline>
            {txHash
              ? "Your Quark account has been created successfully"
              : "Welcome back to Quark!"}
          </Subheadline>
        </div>
      </Section>

      {/* Transaction Hash Card */}
      {txHash && (
        <Section>
          <Card>
            <Cell
              before={
                <Avatar size={48}>
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                    <path
                      d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </Avatar>
              }
              after={
                <Button
                  size="s"
                  mode="plain"
                  onClick={() =>
                    window.open(
                      `https://explorer.aptoslabs.com/txn/${txHash}`,
                      "_blank"
                    )
                  }
                >
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                    <path
                      d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                    <path
                      d="M15 3h6v6"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                    <path
                      d="M10 14L21 3"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </Button>
              }
              subtitle={
                <Caption
                  className="font-mono break-all"
                  style={{
                    marginTop: "8px",
                    lineHeight: "1.4",
                  }}
                >
                  {txHash}
                </Caption>
              }
            >
              <div style={{ display: "flex", alignItems: "center" }}>
                <Text weight="2">Transaction Hash</Text>
                <Badge
                  type="dot"
                  mode="secondary"
                  style={{ marginLeft: "8px" }}
                >
                  Verified
                </Badge>
              </div>
            </Cell>
          </Card>
        </Section>
      )}

      {/* Account Address Card */}
      <Section
        style={{
          textAlign: "center",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          gap: "16px",
        }}
      >
        <Card>
          <Cell
            before={
              <Avatar size={48}>
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                  <circle
                    cx="12"
                    cy="7"
                    r="4"
                    stroke="currentColor"
                    strokeWidth="2"
                  />
                </svg>
              </Avatar>
            }
            after={
              <Button
                size="s"
                mode="plain"
                onClick={() => copyToClipboard(resourceAddress)}
              >
                {copied ? (
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                    <path
                      d="M9 12l2 2 4-4"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                ) : (
                  <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                    <rect
                      x="9"
                      y="9"
                      width="13"
                      height="13"
                      rx="2"
                      ry="2"
                      stroke="currentColor"
                      strokeWidth="2"
                    />
                    <path
                      d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"
                      stroke="currentColor"
                      strokeWidth="2"
                    />
                  </svg>
                )}
              </Button>
            }
            subtitle={
              <Caption
                className="font-mono break-all"
                style={{
                  marginTop: "8px",
                  lineHeight: "1.4",
                }}
              >
                {truncateAddress(resourceAddress)}
              </Caption>
            }
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
              }}
            >
              <Text weight="2">Quark Wallet Address</Text>
              <Badge type="dot" mode="primary" style={{ marginLeft: "8px" }}>
                Active
              </Badge>
            </div>
          </Cell>
        </Card>
        <Button
          size="l"
          stretched
          mode="filled"
          onClick={closeApp}
          style={{
            backgroundColor: "#009F11",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: "8px",
            }}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
              <path
                d="M18 6L6 18"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path
                d="M6 6l12 12"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <span>Close Quark AI</span>
          </div>
        </Button>
      </Section>
    </div>
  );
};
