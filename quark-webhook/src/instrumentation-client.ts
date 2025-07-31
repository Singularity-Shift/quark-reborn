// This file is normally used for setting up analytics and other
// services that require one-time initialization on the client.

import { retrieveLaunchParams } from "@telegram-apps/sdk-react";
import { init } from "./core/init";
import { mockEnv } from "./mockEnv";

// Check if we're running in Telegram context
function isTelegramContext(): boolean {
  if (typeof window === "undefined") return false;

  try {
    // Check if Telegram WebApp is available and has proper data
    const webApp = (window as any).Telegram?.WebApp;
    if (!webApp) return false;

    // Try to access launch parameters - if this fails, we're not in proper Telegram context
    const urlParams = new URLSearchParams(window.location.search);
    const hasLaunchParams =
      urlParams.has("tgWebAppPlatform") ||
      urlParams.has("tgWebAppStartParam") ||
      !!webApp.initData;

    return hasLaunchParams;
  } catch {
    return false;
  }
}

mockEnv().then(() => {
  try {
    const launchParams = retrieveLaunchParams();
    const { tgWebAppPlatform: platform } = launchParams;
    const debug =
      (launchParams.tgWebAppStartParam || "").includes("debug") ||
      process.env.NODE_ENV === "development";

    // Configure all application dependencies.
    init({
      debug,
      eruda: debug && ["ios", "android"].includes(platform),
      mockForMacOS: platform === "macos",
      isTelegram: true,
    });
  } catch (e) {
    // Gracefully handle cases where launch parameters aren't available (browser mode)
    console.log(
      "Launch parameters not available, initializing in browser mode:",
      e
    );
    init({
      debug: process.env.NODE_ENV === "development",
      eruda: false,
      mockForMacOS: false,
      isTelegram: false,
    });
  }
});
