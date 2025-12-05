import React, { useEffect, useState } from "react";
import { useSearchParams, useNavigate } from "react-router-dom";
import { useAuth } from "../contexts/AuthContext";

const VerifyLogin: React.FC = () => {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { verifyLogin } = useAuth();
  const [status, setStatus] = useState<"verifying" | "success" | "error">("verifying");

  const verificationAttempted = React.useRef(false);

  useEffect(() => {
    const verify = async () => {
      const token = searchParams.get("token");
      if (!token) {
        setStatus("error");
        return;
      }

      // Prevent double verification in Strict Mode
      if (verificationAttempted.current) return;
      verificationAttempted.current = true;

      try {
        await verifyLogin(token);
        setStatus("success");
        // Redirect to home or specified path after a short delay
        const redirectPath = searchParams.get("redirect") || "/";
        setTimeout(() => {
          navigate(redirectPath);
        }, 1500);
      } catch (error) {
        console.error("Verification failed:", error);
        setStatus("error");
      }
    };

    verify();
  }, [searchParams, verifyLogin, navigate]);

  return (
    <div
      className="min-h-screen flex items-center justify-center p-4"
      style={{ backgroundColor: "var(--bg-primary)" }}
    >
      <div
        className="max-w-md w-full p-8 rounded-lg shadow-lg text-center"
        style={{
          backgroundColor: "var(--bg-secondary)",
          border: "1px solid var(--border-primary)",
        }}
      >
        {status === "verifying" && (
          <h1 className="text-2xl font-bold" style={{ color: "var(--text-primary)" }}>
            Verifying login...
          </h1>
        )}
        {status === "success" && (
          <>
            <h1 className="text-2xl font-bold mb-2" style={{ color: "var(--text-primary)" }}>
              Login Successful!
            </h1>
            <p style={{ color: "var(--text-secondary)" }}>Redirecting you to the app...</p>
          </>
        )}
        {status === "error" && (
          <>
            <h1 className="text-2xl font-bold mb-2" style={{ color: "var(--error-text)" }}>
              Login Failed
            </h1>
            <p className="mb-4" style={{ color: "var(--text-secondary)" }}>
              The link may be invalid or expired.
            </p>
            <button
              onClick={() => navigate("/login")}
              className="px-4 py-2 rounded bg-blue-600 text-white hover:bg-blue-700"
            >
              Back to Login
            </button>
          </>
        )}
      </div>
    </div>
  );
};

export default VerifyLogin;
