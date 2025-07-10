"use client";

import { useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { Section, Button } from "@telegram-apps/telegram-ui";
import { Message } from "@/components/Message/Message";
import { useMessage } from "@/hooks/useMessage";

const TwitterLoginPage = () => {
  const searchParams = useSearchParams();
  const [isLoading, setIsLoading] = useState(false);
  const { message, showMessage } = useMessage();

  // Get parameters from URL
  const userId = searchParams.get("userId");
  const state = searchParams.get("state");
  const challenge = searchParams.get("challenge");

  const handleTwitterLogin = async () => {
    if (!userId || !state || !challenge) {
      showMessage("Missing required parameters for authentication", "error");
      return;
    }

    setIsLoading(true);

    try {
      // Request OAuth URL from secure API endpoint
      const response = await fetch("/api/twitter/oauth-url", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          userId,
          state,
          challenge,
        }),
      });

      const result = await response.json();

      if (response.ok && result.success) {
        // Redirect to Twitter OAuth
        window.location.href = result.authUrl;
      } else {
        showMessage(result.error || "Failed to generate authentication URL", "error");
        setIsLoading(false);
      }
    } catch (error) {
      console.error("Twitter login error:", error);
      showMessage("Failed to initiate Twitter login", "error");
      setIsLoading(false);
    }
  };

  const isReadyToLogin = userId && state && challenge;

  return (
    <Section className="min-h-screen">
      <Message message={message} />

      <div className="p-4 space-y-4">
        <div className="text-center">
          <h1
            className="text-2xl font-bold mb-2"
            style={{ color: "var(--tg-theme-text-color)" }}
          >
            🐦 Connect X (Twitter)
          </h1>
          <p
            className="text-sm mb-4"
            style={{ color: "var(--tg-theme-hint-color)" }}
          >
            Link your X account to participate in Twitter-based raids
          </p>
        </div>

        <div
          className="bg-gradient-to-r from-blue-50 to-indigo-50 p-4 rounded-lg border border-blue-200"
          style={{
            backgroundColor: "var(--tg-theme-secondary-bg-color)",
            borderColor: "var(--tg-theme-hint-color)",
          }}
        >
          <h3
            className="font-semibold mb-2"
            style={{ color: "var(--tg-theme-text-color)" }}
          >
            Requirements for qualification:
          </h3>
          <ul
            className="text-sm space-y-1"
            style={{ color: "var(--tg-theme-hint-color)" }}
          >
            <li>• At least 50 followers</li>
            <li>• Profile picture</li>
            <li>• Banner image</li>
            <li>• Not verified (no blue checkmark)</li>
          </ul>
        </div>

        {isReadyToLogin ? (
          <div className="flex justify-center">
            <Button
              onClick={handleTwitterLogin}
              disabled={isLoading}
              className="px-8 py-3 text-lg font-semibold"
              style={{
                backgroundColor: isLoading
                  ? "var(--tg-theme-secondary-bg-color)"
                  : "#1DA1F2",
                color: isLoading
                  ? "var(--tg-theme-hint-color)"
                  : "white",
                borderRadius: "12px",
                minWidth: "200px",
              }}
            >
              {isLoading ? "Connecting..." : "Connect X Account"}
            </Button>
          </div>
        ) : (
          <div className="text-center">
            <p
              className="text-sm"
              style={{ color: "var(--tg-theme-hint-color)" }}
            >
              Invalid authentication parameters. Please try again from the bot.
            </p>
          </div>
        )}
      </div>
    </Section>
  );
};

export default TwitterLoginPage; 