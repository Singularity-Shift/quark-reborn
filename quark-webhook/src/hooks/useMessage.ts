import { useState } from "react";

interface MessageState {
  text: string;
  type: "success" | "error";
}

export const useMessage = () => {
  const [message, setMessage] = useState<MessageState | null>(null);

  const showMessage = (text: string, type: "success" | "error") => {
    setMessage({ text, type });
    setTimeout(() => setMessage(null), 2000);
  };

  return { message, showMessage };
};
