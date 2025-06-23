"use client";

import { QuarkUserAbi } from "@/aptos";
import { useAbiClient } from "@/context/AbiProvider";
import { useWallet } from "@aptos-labs/wallet-adapter-react";
import { sendData } from "@telegram-apps/sdk-react";
import { useEffect, useState } from "react";
import { useWalletClient } from "@thalalabs/surf/hooks";
import { ACCOUNT_SEED, EXPLORER_URL } from "@/config/env";
import { AccountStatus } from "./components/AccountStatus";
import { Message } from "@/components/Message/Message";
import { useMessage } from "@/hooks/useMessage";
import { Section } from "@telegram-apps/telegram-ui";
import { useSearchParams } from "next/navigation";
import { useActionDelay } from "@/hooks/useActionDelay";

const LoginPage = () => {
  const { abi } = useAbiClient();
  const { account } = useWallet();
  const { client } = useWalletClient();
  const searchParams = useSearchParams();
  const [resourceAccount, setResourceAccount] = useState<string>("");
  const [txHash, setTxHash] = useState<string>();
  const { message, showMessage } = useMessage();
  const { isDelaying, delayAction } = useActionDelay(1500);

  // Get user ID from URL parameters
  const userId = searchParams.get("userId") || searchParams.get("user_id");

  useEffect(() => {
    if (!userId || !abi || !account?.address) return;

    handleLogin(parseInt(userId));
  }, [userId, abi, account?.address]);

  const handleLogin = async (userId: number) => {
    const resourceAccount = await abi
      ?.useABI(QuarkUserAbi)
      .view.exists_resource_account({
        typeArguments: [],
        functionArguments: [account?.address.toString() as `0x${string}`],
      });

    if (!resourceAccount?.[0]) {
      const seed = ACCOUNT_SEED;
      const tx = await client?.useABI(QuarkUserAbi).create_account({
        type_arguments: [],
        arguments: [`${userId.toString()}_${seed}`],
      });

      showMessage(
        "Account created successfully! Check the transaction details below.",
        "success"
      );

      setTxHash(tx?.hash);
    }

    const resourceAccountResponse = await abi
      ?.useABI(QuarkUserAbi)
      .view.get_resource_account({
        typeArguments: [],
        functionArguments: [account?.address.toString() as `0x${string}`],
      });

    const resourceAccountAddress = resourceAccountResponse?.[0] as string;

    showMessage(
      "Account created successfully! Check the transaction details below.",
      "success"
    );

    setResourceAccount(resourceAccountAddress);

    await delayAction(3000);

    if (sendData.isAvailable()) {
      sendData(
        JSON.stringify({
          accountAddress: account?.address.toString(),
          resourceAccountAddress: resourceAccountAddress,
        })
      );
    }
  };

  return (
    <Section className="min-h-screen">
      <Message message={message} />
      <AccountStatus txHash={txHash} resourceAddress={resourceAccount} />
    </Section>
  );
};

export default LoginPage;
