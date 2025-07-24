"use client";

import { useEffect, useState } from "react";
import { useAbiClient } from "@/context/AbiProvider";
import { QuarkGroupAbi } from "@/aptos";
import { useWalletClient } from "@thalalabs/surf/hooks";
import { useWallet } from "@aptos-labs/wallet-adapter-react";
import {
  Section,
  Cell,
  List,
  Button,
  Card,
  Title,
  Text,
  Caption,
  Badge,
  Spinner,
} from "@telegram-apps/telegram-ui";
import { Page } from "@/components/Page";
import { useMessage } from "@/hooks/useMessage";
import { useActionDelay } from "@/hooks/useActionDelay";
import { useSearchParams } from "next/navigation";
import { closeMiniApp } from "@telegram-apps/sdk-react";

interface DaoInfo {
  name: string;
  description: string;
  choices: string[];
  choices_weights: number[];
  from: number;
  to: number;
  currency: string;
}

export default function DaoPage() {
  const { account, connected } = useWallet();
  const { abi } = useAbiClient();
  const { client } = useWalletClient();
  const [daoInfo, setDaoInfo] = useState<DaoInfo | null>(null);
  const [selectedChoice, setSelectedChoice] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [voting, setVoting] = useState(false);
  const [hasVoted, setHasVoted] = useState(false);
  const { message, showMessage } = useMessage();
  const { isDelaying, delayAction } = useActionDelay(1500);
  const searchParams = useSearchParams();

  // Get query parameters
  const groupId = searchParams.get("group_id");
  const daoId = searchParams.get("dao_id");
  const choiceId = searchParams.get("choice_id");
  const coinType = searchParams.get("coin_type");
  const coinVersion = searchParams.get("coin_version");

  // Detect if opened in external browser vs mini app
  const isExternalBrowser =
    typeof window !== "undefined" && !(window as any).Telegram?.WebApp;

  useEffect(() => {
    if (!groupId || !daoId || !abi || !connected) return;

    fetchDaoInfo();
    checkIfUserVoted();
  }, [groupId, daoId, abi, connected, account?.address]);

  // Pre-select choice if provided in URL
  useEffect(() => {
    if (choiceId && !hasVoted) {
      const choiceIndex = parseInt(choiceId);
      if (!isNaN(choiceIndex)) {
        setSelectedChoice(choiceIndex);
      }
    }
  }, [choiceId, hasVoted]);

  const fetchDaoInfo = async () => {
    if (!groupId || !daoId || !abi) return;

    try {
      setLoading(true);

      // Determine which version to use based on coinVersion
      const viewFunction =
        coinVersion === "V1" ? "get_group_dao_v1" : "get_group_dao_v2";

      const daoData = await abi?.useABI(QuarkGroupAbi).view[viewFunction]({
        typeArguments: [],
        functionArguments: [groupId, daoId],
      });

      if (daoData && Array.isArray(daoData) && daoData[0]) {
        const dao = daoData[0] as any;
        setDaoInfo({
          name: dao.dao_id, // Using dao_id as name for now
          description: `DAO: ${dao.dao_id}`,
          choices: dao.choices,
          choices_weights: dao.choices_weights,
          from: dao.from,
          to: dao.to,
          currency: coinVersion === "V1" ? dao.coin_type : dao.currency,
        });
      }
    } catch (error) {
      console.error("Error fetching DAO info:", error);
      showMessage("Failed to load DAO information", "error");
    } finally {
      setLoading(false);
    }
  };

  const checkIfUserVoted = async () => {
    if (!groupId || !daoId || !abi || !account?.address) return;

    try {
      const viewFunction =
        coinVersion === "V1"
          ? "exist_group_user_choice_v1"
          : "exist_group_user_choice_v2";

      const hasVotedResult = await abi
        ?.useABI(QuarkGroupAbi)
        .view[viewFunction]({
          typeArguments: [],
          functionArguments: [
            groupId,
            daoId,
            account.address.toString(),
          ] as any,
        });

      if (hasVotedResult) {
        setHasVoted(hasVotedResult[0]);
      }
    } catch (error) {
      console.error("Error checking vote status:", error);
    }
  };

  const handleVote = async () => {
    if (
      !groupId ||
      !daoId ||
      selectedChoice === null ||
      !client ||
      !account?.address
    ) {
      showMessage("Missing required information for voting", "error");
      return;
    }

    setVoting(true);

    try {
      let tx;

      if (coinVersion === "V1") {
        // For V1, we need to pass the coin type as a type argument
        tx = await client?.useABI(QuarkGroupAbi).vote_group_dao_v1({
          type_arguments: [coinType || "0x1::aptos_coin::AptosCoin"],
          arguments: [groupId, daoId, selectedChoice.toString()],
        });
      } else {
        // For V2, currency is passed as an argument
        tx = await client?.useABI(QuarkGroupAbi).vote_group_dao_v2({
          type_arguments: [],
          arguments: [
            groupId,
            daoId,
            selectedChoice.toString(),
            coinType || "",
          ] as any,
        });
      }

      if (tx?.hash) {
        showMessage("Vote submitted successfully!", "success");
        setHasVoted(true);

        // Refresh DAO info to get updated vote counts
        await delayAction(2000);
        await fetchDaoInfo();

        // Close mini app after successful vote (only in mini app mode)
        if (!isExternalBrowser) {
          setTimeout(() => {
            if (closeMiniApp.isAvailable()) {
              closeMiniApp();
            }
          }, 3000);
        }
      }
    } catch (error) {
      console.error("Voting error:", error);
      showMessage("Failed to submit vote. Please try again.", "error");
    } finally {
      setVoting(false);
    }
  };

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const getTotalVotes = () => {
    if (!daoInfo) return 0;
    return daoInfo.choices_weights.reduce((sum, weight) => sum + weight, 0);
  };

  const getVotePercentage = (votes: number) => {
    const total = getTotalVotes();
    return total > 0 ? ((votes / total) * 100).toFixed(1) : "0";
  };

  if (loading) {
    return (
      <Page>
        <div
          style={{
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            height: "200px",
            flexDirection: "column",
            gap: "16px",
          }}
        >
          <Spinner size="l" />
          <Text>Loading DAO information...</Text>
        </div>
      </Page>
    );
  }

  if (!daoInfo) {
    return (
      <Page>
        <Card>
          <Text>❌ DAO not found or failed to load</Text>
        </Card>
      </Page>
    );
  }

  const now = Math.floor(Date.now() / 1000);
  const isVotingActive = now >= daoInfo.from && now <= daoInfo.to;

  return (
    <Page>
      {/* Platform Indicator */}
      <div
        style={{
          position: "fixed",
          top: "10px",
          right: "10px",
          padding: "6px 12px",
          backgroundColor: isExternalBrowser ? "#2196F3" : "#4CAF50",
          color: "white",
          borderRadius: "16px",
          fontSize: "12px",
          fontWeight: "bold",
          zIndex: 1000,
          boxShadow: "0 2px 8px rgba(0,0,0,0.2)",
        }}
      >
        {isExternalBrowser ? "🌐 Browser" : "📱 Mini App"}
      </div>

      <List>
        {/* DAO Header */}
        <Section>
          <Card style={{ padding: "16px", marginBottom: "16px" }}>
            <div style={{ textAlign: "center", marginBottom: "16px" }}>
              <Title
                level="1"
                style={{ fontSize: "24px", marginBottom: "8px" }}
              >
                🏛️ {daoInfo.name}
              </Title>
              <Text style={{ fontSize: "16px", opacity: 0.8 }}>
                {daoInfo.description}
              </Text>
            </div>

            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                marginBottom: "16px",
              }}
            >
              <div style={{ textAlign: "center" }}>
                <Caption style={{ display: "block", marginBottom: "4px" }}>
                  Voting Period
                </Caption>
                <Text style={{ fontSize: "12px" }}>
                  {formatTimestamp(daoInfo.from)} -{" "}
                  {formatTimestamp(daoInfo.to)}
                </Text>
              </div>
              <div style={{ textAlign: "center" }}>
                <Caption style={{ display: "block", marginBottom: "4px" }}>
                  Total Votes
                </Caption>
                <Text style={{ fontSize: "14px", fontWeight: "bold" }}>
                  {getTotalVotes()}
                </Text>
              </div>
            </div>

            <div style={{ textAlign: "center" }}>
              <Badge
                type="number"
                style={{
                  backgroundColor: isVotingActive ? "#4CAF50" : "#f44336",
                  color: "white",
                }}
              >
                {isVotingActive ? "🟢 Voting Active" : "🔴 Voting Ended"}
              </Badge>
            </div>
          </Card>
        </Section>

        {/* Voting Options */}
        {isVotingActive && !hasVoted && (
          <Section header="Choose Your Option">
            {daoInfo.choices.map((choice, index) => (
              <Cell
                key={index}
                onClick={() => setSelectedChoice(index)}
                style={{
                  cursor: "pointer",
                  backgroundColor:
                    selectedChoice === index
                      ? "var(--tg-theme-button-color)"
                      : "transparent",
                  color:
                    selectedChoice === index
                      ? "var(--tg-theme-button-text-color)"
                      : "inherit",
                  borderRadius: "8px",
                  margin: "4px 0",
                  padding: "12px",
                  border:
                    selectedChoice === index
                      ? "2px solid var(--tg-theme-button-color)"
                      : "1px solid var(--tg-theme-secondary-bg-color)",
                }}
              >
                <div
                  style={{ display: "flex", alignItems: "center", gap: "12px" }}
                >
                  <div
                    style={{
                      width: "24px",
                      height: "24px",
                      borderRadius: "50%",
                      backgroundColor:
                        selectedChoice === index ? "white" : "transparent",
                      border: "2px solid",
                      borderColor:
                        selectedChoice === index
                          ? "white"
                          : "var(--tg-theme-text-color)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                    }}
                  >
                    {selectedChoice === index && (
                      <div
                        style={{
                          width: "12px",
                          height: "12px",
                          borderRadius: "50%",
                          backgroundColor: "var(--tg-theme-button-color)",
                        }}
                      />
                    )}
                  </div>
                  <Text
                    style={{
                      fontSize: "16px",
                      fontWeight: selectedChoice === index ? "bold" : "normal",
                    }}
                  >
                    {choice}
                  </Text>
                </div>
              </Cell>
            ))}
          </Section>
        )}

        {/* Vote Results */}
        <Section header="Current Results">
          {daoInfo.choices.map((choice, index) => (
            <Cell key={index}>
              <div style={{ width: "100%" }}>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    marginBottom: "8px",
                  }}
                >
                  <Text style={{ fontWeight: "bold" }}>{choice}</Text>
                  <Text>
                    {daoInfo.choices_weights[index]} votes (
                    {getVotePercentage(daoInfo.choices_weights[index])}%)
                  </Text>
                </div>
                <div
                  style={{
                    width: "100%",
                    height: "8px",
                    backgroundColor: "var(--tg-theme-secondary-bg-color)",
                    borderRadius: "4px",
                    overflow: "hidden",
                  }}
                >
                  <div
                    style={{
                      width: `${getVotePercentage(
                        daoInfo.choices_weights[index]
                      )}%`,
                      height: "100%",
                      backgroundColor: "var(--tg-theme-button-color)",
                      transition: "width 0.3s ease",
                    }}
                  />
                </div>
              </div>
            </Cell>
          ))}
        </Section>

        {/* Vote Button */}
        {isVotingActive && !hasVoted && (
          <Section>
            {/* Platform-specific instructions */}
            {isExternalBrowser && (
              <Card
                style={{
                  padding: "12px",
                  marginBottom: "16px",
                  backgroundColor: "rgba(33, 150, 243, 0.1)",
                  border: "1px solid rgba(33, 150, 243, 0.2)",
                }}
              >
                <Text style={{ fontSize: "14px", textAlign: "center" }}>
                  🌐 <strong>Browser Mode:</strong> Make sure your wallet is
                  connected to vote
                </Text>
              </Card>
            )}

            <Button
              size="l"
              onClick={handleVote}
              disabled={selectedChoice === null || voting || isDelaying}
              style={{
                width: "100%",
                backgroundColor:
                  selectedChoice !== null
                    ? "var(--tg-theme-button-color)"
                    : "var(--tg-theme-secondary-bg-color)",
                color:
                  selectedChoice !== null
                    ? "var(--tg-theme-button-text-color)"
                    : "var(--tg-theme-hint-color)",
              }}
            >
              {voting || isDelaying ? (
                <div
                  style={{ display: "flex", alignItems: "center", gap: "8px" }}
                >
                  <Spinner size="s" />
                  Submitting Vote...
                </div>
              ) : (
                `🗳️ Vote for: ${
                  selectedChoice !== null
                    ? daoInfo.choices[selectedChoice]
                    : "Select an option"
                }`
              )}
            </Button>

            {!isExternalBrowser && (
              <Text
                style={{
                  fontSize: "12px",
                  textAlign: "center",
                  marginTop: "8px",
                  opacity: 0.7,
                }}
              >
                📱 Mini app will close automatically after voting
              </Text>
            )}
          </Section>
        )}

        {/* Already Voted Message */}
        {hasVoted && (
          <Section>
            <Card
              style={{
                padding: "16px",
                textAlign: "center",
                backgroundColor: "var(--tg-theme-secondary-bg-color)",
              }}
            >
              <Text style={{ fontSize: "18px", marginBottom: "8px" }}>
                ✅ You have already voted!
              </Text>
              <Caption>Thank you for participating in this DAO vote.</Caption>
            </Card>
          </Section>
        )}

        {/* Voting Ended Message */}
        {!isVotingActive && (
          <Section>
            <Card
              style={{
                padding: "16px",
                textAlign: "center",
                backgroundColor: "var(--tg-theme-secondary-bg-color)",
              }}
            >
              <Text style={{ fontSize: "18px", marginBottom: "8px" }}>
                🔒 Voting has ended
              </Text>
              <Caption>
                The voting period for this DAO has concluded. Results are shown
                above.
              </Caption>
            </Card>
          </Section>
        )}
      </List>

      {/* Message Component */}
      {message?.text && (
        <div
          style={{
            position: "fixed",
            top: "20px",
            left: "50%",
            transform: "translateX(-50%)",
            padding: "12px 16px",
            backgroundColor: message.type === "error" ? "#f44336" : "#4CAF50",
            color: "white",
            borderRadius: "8px",
            zIndex: 1000,
          }}
        >
          {message.text}
        </div>
      )}
    </Page>
  );
}
