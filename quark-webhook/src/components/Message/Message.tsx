import React from "react";

interface MessageProps {
  message: {
    text: string;
    type: "success" | "error";
  } | null;
}

export const Message: React.FC<MessageProps> = ({ message }) => {
  if (!message) return null;

  return (
    <>
      <div
        style={{
          position: "fixed",
          top: "20px",
          left: "50%",
          transform: "translateX(-50%)",
          zIndex: 1000,
          backgroundColor:
            message.type === "success"
              ? "var(--tg-theme-button-color, #007AFF)"
              : "var(--tg-theme-destructive-text-color, #FF3B30)",
          color: "var(--tg-theme-button-text-color, #FFFFFF)",
          padding: "12px 20px",
          borderRadius: "12px",
          fontWeight: "500",
          fontSize: "14px",
          boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
          border: `2px solid ${
            message.type === "success"
              ? "var(--tg-theme-button-color, #007AFF)"
              : "var(--tg-theme-destructive-text-color, #FF3B30)"
          }`,
          maxWidth: "90%",
          textAlign: "center",
          animation: "slideDown 0.3s ease-out",
        }}
      >
        {message.type === "success" ? "✅" : "❌"} {message.text}
      </div>

      <style jsx>{`
        @keyframes slideDown {
          0% {
            opacity: 0;
            transform: translateX(-50%) translateY(-20px);
          }
          100% {
            opacity: 1;
            transform: translateX(-50%) translateY(0);
          }
        }
      `}</style>
    </>
  );
};
