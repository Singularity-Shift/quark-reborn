"use client";

import { QuarkUserAbi } from "@/aptos";
import { useAbiClient } from "@/context/AbiProvider";
import { useWallet } from "@aptos-labs/wallet-adapter-react";
import { sendData } from "@telegram-apps/sdk-react";
import { useEffect, useState } from "react";
import { useWalletClient } from "@thalalabs/surf/hooks";
import toast from "react-hot-toast";
import { EXPLORER_URL } from "@/config/env";
import { AccountStatus } from "./components/AccountStatus";
import { Toaster } from "react-hot-toast";
import { Section } from "@telegram-apps/telegram-ui";
import { useSearchParams } from "next/navigation";

const LoginPage = () => {
  const { abi } = useAbiClient();
  const { account } = useWallet();
  const { client } = useWalletClient();
  const searchParams = useSearchParams();
  const [resourceAccount, setResourceAccount] = useState<string>("");
  const [txHash, setTxHash] = useState<string>();

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
      const tx = await client?.useABI(QuarkUserAbi).create_account({
        type_arguments: [],
        arguments: [userId.toString() as string],
      });

      const explorer = `${EXPLORER_URL}/txn/${tx?.hash}`;

      toast.success(
        <div className="flex flex-col gap-2">
          <div className="font-bold">✅ Account created successfully!</div>
          <div className="text-sm">
            <a
              href={explorer}
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-500 hover:underline"
            >
              🔗 View on Explorer
            </a>
          </div>
          <code className="text-xs bg-gray-100 p-1 rounded break-all">
            {tx?.hash}
          </code>
        </div>,
        { duration: 8000 }
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

    if (sendData.isAvailable()) {
      console.log("Sending Data");
      sendData(resourceAccountAddress);
    }

    setResourceAccount(resourceAccountAddress);
  };

  return (
    <Section className="min-h-screen">
      <AccountStatus txHash={txHash} resourceAddress={resourceAccount} />
      <Toaster position="top-center" />
    </Section>
  );
};

export default LoginPage;
