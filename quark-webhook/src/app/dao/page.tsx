"use client";

import { useEffect, useState } from "react";
import { useAbiClient } from "@/context/AbiProvider";
import { QuarkGroupAbi } from "@/aptos";
import { useWalletClient } from "@thalalabs/surf/hooks";
import { useWallet } from "@aptos-labs/wallet-adapter-react";
import { Page } from "@/components/Page";
import { useMessage } from "@/hooks/useMessage";
import { useActionDelay } from "@/hooks/useActionDelay";
import { useSearchParams } from "next/navigation";
import { closeMiniApp } from "@telegram-apps/sdk-react";
import { IDAOProposal } from "@/helpers";

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
  const [paramError, setParamError] = useState<string | null>(null);
  const { message, showMessage } = useMessage();
  const { isDelaying, delayAction } = useActionDelay(1500);
  const searchParams = useSearchParams();

  // Get query parameters
  const groupId = searchParams.get("group_id");
  const daoId = searchParams.get("dao_id");
  const choiceId = searchParams.get("choice_id");
  const coinType = searchParams.get("coin_type");
  const coinVersion = searchParams.get("coin_version");
  const daoName = searchParams.get("dao_name");
  const daoDescription = searchParams.get("dao_description");

  // Check for required parameters
  useEffect(() => {
    if (!groupId || !daoId) {
      setParamError(
        "Missing required parameters. Please access this page through a valid DAO voting link."
      );
      setLoading(false);
    } else {
      setParamError(null);
    }
  }, [groupId, daoId]);

  // Detect if opened in external browser vs mini app
  const isExternalBrowser = (() => {
    if (typeof window === "undefined") return false;

    // Check for Telegram WebApp object
    const hasWebApp = !!(window as any).Telegram?.WebApp;
    if (!hasWebApp) return true;

    // Additional check: see if we have proper launch parameters
    try {
      const webApp = (window as any).Telegram.WebApp;
      // If WebApp exists but initData is empty/invalid, we might be in a problematic context
      const hasValidInitData = webApp.initData && webApp.initData.length > 0;
      const hasValidPlatform = webApp.platform && webApp.platform !== "unknown";

      // If we have WebApp but no valid data, treat as external browser for better UX
      if (!hasValidInitData && !hasValidPlatform) {
        console.log(
          "Telegram WebApp detected but with invalid/empty data, treating as external browser"
        );
        return true;
      }

      return false; // Valid Telegram Mini App context
    } catch (e) {
      console.log(
        "Error checking Telegram context, treating as external browser:",
        e
      );
      return true;
    }
  })();

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
        const dao = daoData[0] as IDAOProposal;
        setDaoInfo({
          name: daoName || "",
          description: daoDescription || "",
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
            try {
              if (closeMiniApp.isAvailable()) {
                closeMiniApp();
              }
            } catch (e) {
              console.log("Could not close mini app:", e);
              // Fallback: show a message to user that they can close manually
              showMessage(
                "Vote submitted successfully! You can close this tab.",
                "success"
              );
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

  // Show parameter error if required parameters are missing
  if (paramError) {
    return (
      <Page>
        <div
          className={`max-w-4xl mx-auto p-4 ${
            isExternalBrowser ? "min-h-screen bg-gray-50" : ""
          }`}
        >
          <div className="bg-white rounded-lg shadow-lg p-6">
            <h2 className="text-2xl font-bold text-gray-900 mb-4">
              ⚠️ Invalid Access
            </h2>
            <p className="text-gray-700 mb-4">{paramError}</p>
            {isExternalBrowser && (
              <p className="text-gray-600">
                Make sure you&apos;re accessing this page through a valid DAO
                voting link from your Telegram group.
              </p>
            )}
          </div>
        </div>
      </Page>
    );
  }

  if (loading) {
    return (
      <Page>
        <div
          className={`max-w-4xl mx-auto p-4 ${
            isExternalBrowser ? "min-h-screen bg-gray-50" : ""
          }`}
        >
          <div className="bg-white rounded-lg shadow-lg p-6">
            <div className="flex flex-col items-center py-12">
              <div className="w-10 h-10 border-4 border-gray-200 border-t-blue-600 rounded-full animate-spin mb-4"></div>
              <p className="text-gray-700">Loading DAO information...</p>
            </div>
          </div>
        </div>
      </Page>
    );
  }

  if (!daoInfo) {
    return (
      <Page>
        <div
          className={`max-w-4xl mx-auto p-4 ${
            isExternalBrowser ? "min-h-screen bg-gray-50" : ""
          }`}
        >
          <div className="bg-white rounded-lg shadow-lg p-6">
            <div className="text-center">
              <h2 className="text-2xl font-bold text-gray-900 mb-4">
                ❌ DAO not found or failed to load
              </h2>
              <p className="text-gray-600">
                Please check the URL and try again.
              </p>
            </div>
          </div>
        </div>
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

      <div
        className={`max-w-4xl mx-auto p-4 ${
          isExternalBrowser ? "min-h-screen bg-gray-50" : ""
        }`}
      >
        {/* DAO Header */}
        <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
          <div className="text-center mb-6">
            <h1 className="text-3xl font-bold text-gray-900 mb-3">
              🏛️ {daoInfo.name}
            </h1>
            <p className="text-lg text-gray-600 leading-relaxed">
              {daoInfo.description}
            </p>
          </div>

          <div className="grid grid-cols-2 gap-4 mb-6">
            <div className="text-center p-4 bg-gray-50 rounded-lg">
              <p className="text-sm font-medium text-gray-500 mb-2">
                Voting Period
              </p>
              <p className="text-sm text-gray-900">
                {formatTimestamp(daoInfo.from)} - {formatTimestamp(daoInfo.to)}
              </p>
            </div>
            <div className="text-center p-4 bg-gray-50 rounded-lg">
              <p className="text-sm font-medium text-gray-500 mb-2">
                Total Votes
              </p>
              <p className="text-lg font-bold text-gray-900">
                {getTotalVotes()}
              </p>
            </div>
          </div>

          <div className="text-center">
            <span
              className={`inline-flex items-center px-4 py-2 rounded-full text-sm font-medium ${
                isVotingActive
                  ? "bg-green-100 text-green-800"
                  : "bg-red-100 text-red-800"
              }`}
            >
              {isVotingActive ? "🟢 Voting Active" : "🔴 Voting Ended"}
            </span>
          </div>
        </div>

        {/* Voting Options */}
        {isVotingActive && !hasVoted && (
          <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
            <h2 className="text-xl font-bold text-gray-900 mb-4">
              Choose Your Option
            </h2>
            <div className="space-y-3">
              {daoInfo.choices.map((choice, index) => (
                <div
                  key={index}
                  onClick={() => setSelectedChoice(index)}
                  className={`p-4 rounded-lg border-2 cursor-pointer transition-all duration-200 ${
                    selectedChoice === index
                      ? "border-blue-500 bg-blue-50"
                      : "border-gray-200 hover:border-gray-300"
                  }`}
                >
                  <div className="flex items-center space-x-3">
                    <div
                      className={`w-6 h-6 rounded-full border-2 flex items-center justify-center ${
                        selectedChoice === index
                          ? "border-blue-500 bg-blue-500"
                          : "border-gray-300"
                      }`}
                    >
                      {selectedChoice === index && (
                        <div className="w-3 h-3 rounded-full bg-white" />
                      )}
                    </div>
                    <span
                      className={`text-lg ${
                        selectedChoice === index
                          ? "font-semibold text-blue-900"
                          : "text-gray-700"
                      }`}
                    >
                      {choice}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Vote Results */}
        <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
          <h2 className="text-xl font-bold text-gray-900 mb-4">
            Current Results
          </h2>
          <div className="space-y-4">
            {daoInfo.choices.map((choice, index) => (
              <div key={index} className="w-full">
                <div className="flex justify-between items-center mb-2">
                  <span className="font-semibold text-gray-900">{choice}</span>
                  <span className="text-sm text-gray-600">
                    {daoInfo.choices_weights[index]} votes (
                    {getVotePercentage(daoInfo.choices_weights[index])}%)
                  </span>
                </div>
                <div className="w-full h-3 bg-gray-200 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-blue-500 transition-all duration-300 ease-in-out"
                    style={{
                      width: `${getVotePercentage(
                        daoInfo.choices_weights[index]
                      )}%`,
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Vote Button */}
        {isVotingActive && !hasVoted && (
          <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
            {/* Platform-specific instructions */}
            {isExternalBrowser && (
              <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-4">
                <p className="text-sm text-blue-800 text-center">
                  🌐 <strong>Browser Mode:</strong> Make sure your wallet is
                  connected to vote
                </p>
              </div>
            )}

            <button
              onClick={handleVote}
              disabled={selectedChoice === null || voting || isDelaying}
              className={`w-full py-3 px-6 rounded-lg font-medium transition-all duration-200 ${
                selectedChoice !== null && !voting && !isDelaying
                  ? "bg-blue-600 hover:bg-blue-700 text-white"
                  : "bg-gray-300 text-gray-500 cursor-not-allowed"
              }`}
            >
              {voting || isDelaying ? (
                <div className="flex items-center justify-center space-x-2">
                  <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                  <span>Submitting Vote...</span>
                </div>
              ) : (
                `🗳️ Vote for: ${
                  selectedChoice !== null
                    ? daoInfo.choices[selectedChoice]
                    : "Select an option"
                }`
              )}
            </button>

            {!isExternalBrowser && (
              <p className="text-xs text-gray-500 text-center mt-3">
                📱 Mini app will close automatically after voting
              </p>
            )}
          </div>
        )}

        {/* Already Voted Message */}
        {hasVoted && (
          <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
            <div className="text-center">
              <h3 className="text-xl font-semibold text-gray-900 mb-2">
                ✅ You have already voted!
              </h3>
              <p className="text-gray-600">
                Thank you for participating in this DAO vote.
              </p>
            </div>
          </div>
        )}

        {/* Voting Ended Message */}
        {!isVotingActive && (
          <div className="bg-white rounded-lg shadow-lg p-6 mb-6">
            <div className="text-center">
              <h3 className="text-xl font-semibold text-gray-900 mb-2">
                🔒 Voting has ended
              </h3>
              <p className="text-gray-600">
                The voting period for this DAO has concluded. Results are shown
                above.
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Message Component */}
      {message?.text && (
        <div
          className={`fixed top-5 left-1/2 transform -translate-x-1/2 px-4 py-3 rounded-lg text-white font-medium z-50 ${
            message.type === "error" ? "bg-red-500" : "bg-green-500"
          }`}
        >
          {message.text}
        </div>
      )}
    </Page>
  );
}
