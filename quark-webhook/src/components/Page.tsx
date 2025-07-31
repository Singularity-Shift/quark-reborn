"use client";
import "tailwindcss/tailwind.css";
import { backButton } from "@telegram-apps/sdk-react";
import { PropsWithChildren, useEffect } from "react";
import { useRouter } from "next/navigation";

// Safe wrapper for backButton operations
function useSafeBackButton() {
  const show = () => {
    try {
      backButton.show();
    } catch (error) {
      console.log("BackButton not available:", error);
    }
  };

  const hide = () => {
    try {
      backButton.hide();
    } catch (error) {
      console.log("BackButton not available:", error);
    }
  };

  const onClick = (callback: () => void) => {
    try {
      return backButton.onClick(callback);
    } catch (error) {
      console.log("BackButton not available:", error);
    }
    // Return empty cleanup function if backButton is not available
    return () => {};
  };

  return { show, hide, onClick };
}

export function Page({
  children,
  back = true,
}: PropsWithChildren<{
  /**
   * True if it is allowed to go back from this page.
   * @default true
   */
  back?: boolean;
}>) {
  const router = useRouter();
  const safeBackButton = useSafeBackButton();

  useEffect(() => {
    if (back) {
      safeBackButton.show();
    } else {
      safeBackButton.hide();
    }
  }, [back, safeBackButton]);

  useEffect(() => {
    return safeBackButton.onClick(() => {
      router.back();
    });
  }, [router, safeBackButton]);

  return <>{children}</>;
}
