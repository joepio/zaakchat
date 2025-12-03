import React from "react";
import { useSSE } from "../contexts/SSEContext";

export const ConnectionErrorBanner: React.FC = () => {
  const { connectionStatus, errorMessage, retryConnection } = useSSE();

  if (connectionStatus !== "error" || !errorMessage) {
    return null;
  }

  return (
    <div
      className="fixed top-0 left-0 right-0 z-50 p-4 text-center"
      style={{
        backgroundColor: "var(--error-bg)",
        borderBottom: "1px solid var(--error-border)",
        color: "var(--error-text)",
      }}
    >
      <div className="flex items-center justify-center gap-4">
        <svg
          className="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
          />
        </svg>
        <span className="font-medium">{errorMessage}</span>
        <button
          onClick={retryConnection}
          className="px-4 py-1 rounded font-medium hover:opacity-80 transition-opacity"
          style={{
            backgroundColor: "var(--error-text)",
            color: "var(--error-bg)",
          }}
        >
          Retry Now
        </button>
      </div>
    </div>
  );
};
