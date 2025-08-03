"use client";

import { type PropsWithChildren, useEffect, useState } from "react";
import {
  initData,
  miniApp,
  useLaunchParams,
  useSignal,
} from "@telegram-apps/sdk-react";
import { usePathname } from "next/navigation";

import { AppRoot } from "@telegram-apps/telegram-ui";

import { ErrorBoundary } from "@/components/ErrorBoundary";
import { ErrorPage } from "@/components/ErrorPage";
import { useDidMount } from "@/hooks/useDidMount";
import { setLocale } from "@/core/i18n/locale";
import { WalletNavbar } from "@/components/WalletNavbar/WalletNavbar";
import { DaoWallet } from "@/components/DaoWallet/DaoWallet";

import "./styles.css";

// Component that renders the appropriate wallet component based on the current route
function WalletComponent() {
  const pathname = usePathname();

  // Show DaoWallet on DAO page (only when connected), WalletNavbar on other pages
  if (pathname === "/dao") {
    return <DaoWallet />;
  }

  return <WalletNavbar />;
}

// Component that renders with Telegram context
function TelegramRootInner({ children }: PropsWithChildren) {
  const lp = useLaunchParams();
  const isDark = useSignal(miniApp.isDark);
  const initDataUser = useSignal(initData.user);

  // Set the user locale.
  useEffect(() => {
    if (initDataUser && initDataUser.language_code) {
      setLocale(initDataUser.language_code);
    }
  }, [initDataUser]);

  return (
    <AppRoot
      appearance={isDark ? "dark" : "light"}
      platform={
        ["macos", "ios", "android"].includes(lp.tgWebAppPlatform)
          ? "ios"
          : "base"
      }
    >
      <WalletComponent />
      {children}
    </AppRoot>
  );
}

// Component that renders without Telegram context (browser mode)
function BrowserRootInner({ children }: PropsWithChildren) {
  return (
    <AppRoot appearance="light" platform="base">
      <WalletComponent />
      {children}
    </AppRoot>
  );
}

// Check if we're in Telegram context
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

function RootInner({ children }: PropsWithChildren) {
  const [isClient, setIsClient] = useState(false);
  const [isTelegram, setIsTelegram] = useState(false);

  useEffect(() => {
    setIsClient(true);
    setIsTelegram(isTelegramContext());
  }, []);

  // Server-side rendering fallback
  if (!isClient) {
    return (
      <AppRoot appearance="light" platform="base">
        <WalletComponent />
        {children}
      </AppRoot>
    );
  }

  // Render based on context
  if (isTelegram) {
    return <TelegramRootInner>{children}</TelegramRootInner>;
  } else {
    return <BrowserRootInner>{children}</BrowserRootInner>;
  }
}

export function Root(props: PropsWithChildren) {
  // Unfortunately, Telegram Mini Apps does not allow us to use all features of
  // the Server Side Rendering. That's why we are showing loader on the server
  // side.
  const didMount = useDidMount();

  return didMount ? (
    <ErrorBoundary fallback={ErrorPage}>
      <RootInner {...props} />
    </ErrorBoundary>
  ) : (
    <div className="root__loading">Loading</div>
  );
}
