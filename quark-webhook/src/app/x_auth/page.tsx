"use client";

import { useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { sendData } from "@telegram-apps/sdk-react";
import { Section, Button } from "@telegram-apps/telegram-ui";
import { Message } from "@/components/Message/Message";
import { useMessage } from "@/hooks/useMessage";

interface TwitterAuthResult {
  success: boolean;
  user?: {
    telegram_username: string;
    twitter_handle: string;
    twitter_id: string;
    follower_count: number;
    qualifies: boolean;
  };
  error?: string;
}

const XAuthPage = () => {
  const searchParams = useSearchParams();
  const [isProcessing, setIsProcessing] = useState(true);
  const [authResult, setAuthResult] = useState<TwitterAuthResult | null>(null);
  const { message, showMessage } = useMessage();

  // Extract OAuth parameters
  const code = searchParams.get("code");
  const state = searchParams.get("state");
  const error = searchParams.get("error");
  const errorDescription = searchParams.get("error_description");

  useEffect(() => {
    const processOAuthCallback = async () => {
      if (error) {
        const errorMessage = `Authentication failed: ${errorDescription || error}`;
        setAuthResult({
          success: false,
          error: errorMessage,
        });
        setIsProcessing(false);

        // Send OAuth error back to bot
        if (sendData.isAvailable()) {
          setTimeout(() => {
            sendData(
              JSON.stringify({
                type: "twitter_auth_failure",
                error: errorMessage,
              })
            );
          }, 1000);
        }
        return;
      }

      if (!code || !state) {
        const errorMessage = "Missing required OAuth parameters";
        setAuthResult({
          success: false,
          error: errorMessage,
        });
        setIsProcessing(false);

        // Send missing parameters error back to bot
        if (sendData.isAvailable()) {
          setTimeout(() => {
            sendData(
              JSON.stringify({
                type: "twitter_auth_failure",
                error: errorMessage,
              })
            );
          }, 1000);
        }
        return;
      }

      try {
        // Call the token exchange API
        const response = await fetch("/api/twitter/auth", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            code,
            state,
          }),
        });

        const result = await response.json();

        if (response.ok && result.success) {
          setAuthResult({
            success: true,
            user: result.user,
          });

          showMessage(
            `🎉 Successfully connected X account @${result.user.twitter_handle}!`,
            "success"
          );

          // Send result back to bot via Telegram Web App
          if (sendData.isAvailable()) {
            setTimeout(() => {
              sendData(
                JSON.stringify({
                  type: "twitter_auth_success",
                  user: result.user,
                })
              );
            }, 2000);
          }
        } else {
          setAuthResult({
            success: false,
            error: result.error || "Authentication failed",
          });
          showMessage("❌ Authentication failed", "error");

          // Send failure result back to bot via Telegram Web App
          if (sendData.isAvailable()) {
            setTimeout(() => {
              sendData(
                JSON.stringify({
                  type: "twitter_auth_failure",
                  error: result.error || "Authentication failed",
                })
              );
            }, 2000);
          }
        }
      } catch (err) {
        console.error("Auth error:", err);
        const errorMessage = "Network error during authentication";
        setAuthResult({
          success: false,
          error: errorMessage,
        });
        showMessage("❌ Network error", "error");

        // Send network error result back to bot via Telegram Web App
        if (sendData.isAvailable()) {
          setTimeout(() => {
            sendData(
              JSON.stringify({
                type: "twitter_auth_failure",
                error: errorMessage,
              })
            );
          }, 2000);
        }
      }

      setIsProcessing(false);
    };

    processOAuthCallback();
  }, [code, state, error, errorDescription, showMessage]);

  return (
    <Section className="min-h-screen">
      <Message message={message} />

      <div className="p-4 space-y-4">
        <div className="text-center">
          <h1
            className="text-2xl font-bold mb-2"
            style={{ color: "var(--tg-theme-text-color)" }}
          >
            🐦 X (Twitter) Authentication
          </h1>
        </div>

        {isProcessing ? (
          <div className="text-center py-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto mb-4"></div>
            <p
              className="text-sm"
              style={{ color: "var(--tg-theme-hint-color)" }}
            >
              Processing authentication...
            </p>
          </div>
        ) : authResult?.success ? (
          <div className="text-center space-y-4">
            <div
              className="p-4 rounded-lg"
              style={{ backgroundColor: "var(--tg-theme-secondary-bg-color)" }}
            >
              <div
                className="text-lg font-semibold mb-2"
                style={{ color: "var(--tg-theme-text-color)" }}
              >
                ✅ Successfully Connected!
              </div>
              <div
                className="text-sm space-y-1"
                style={{ color: "var(--tg-theme-hint-color)" }}
              >
                <p>
                  <strong>X Handle:</strong> @{authResult.user?.twitter_handle}
                </p>
                <p>
                  <strong>Followers:</strong> {authResult.user?.follower_count.toLocaleString()}
                </p>
                <p>
                  <strong>Status:</strong>{" "}
                  <span
                    style={{
                      color: authResult.user?.qualifies
                        ? "var(--tg-theme-link-color)"
                        : "var(--tg-theme-destructive-text-color)",
                    }}
                  >
                    {authResult.user?.qualifies ? "✅ Qualified" : "❌ Not Qualified"}
                  </span>
                </p>
              </div>
            </div>
            <p
              className="text-sm"
              style={{ color: "var(--tg-theme-hint-color)" }}
            >
              Returning to bot...
            </p>
          </div>
        ) : (
          <div className="text-center space-y-4">
            <div
              className="p-4 rounded-lg"
              style={{ 
                backgroundColor: "var(--tg-theme-secondary-bg-color)",
                border: "1px solid var(--tg-theme-destructive-text-color)"
              }}
            >
              <div
                className="text-lg font-semibold mb-2"
                style={{ color: "var(--tg-theme-destructive-text-color)" }}
              >
                ❌ Authentication Failed
              </div>
              <p
                className="text-sm"
                style={{ color: "var(--tg-theme-hint-color)" }}
              >
                {authResult?.error}
              </p>
            </div>
            <Button
              onClick={() => window.close()}
              style={{
                backgroundColor: "var(--tg-theme-button-color)",
                color: "var(--tg-theme-button-text-color)",
              }}
            >
              Close
            </Button>
          </div>
        )}
      </div>
    </Section>
  );
};

export default XAuthPage; 