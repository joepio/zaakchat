import React, { useState } from "react";
import { useAuth } from "../contexts/AuthContext";
import { Button } from "./ActionButton";

const Login: React.FC = () => {
  const { login } = useAuth();
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      await login(email);
    } catch (error) {
      console.error("Login failed:", error);
      setError("Failed to login. Please try again.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="min-h-screen flex items-center justify-center p-4"
      style={{
        backgroundColor: "var(--bg-primary)",
      }}
    >
      <div
        className="max-w-md w-full p-8 rounded-lg shadow-lg"
        style={{
          backgroundColor: "var(--bg-secondary)",
          border: "1px solid var(--border-primary)",
        }}
      >
        <h1
          className="text-2xl font-bold mb-6 text-center"
          style={{ color: "var(--text-primary)" }}
        >
          ZaakChat Login
        </h1>

        <form onSubmit={handleSubmit} className="space-y-6">
          <div>
            <label
              htmlFor="email"
              className="block text-sm font-medium mb-2"
              style={{ color: "var(--text-secondary)" }}
            >
              Email Address
            </label>
            <input
              id="email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              className="w-full p-3 rounded border outline-none transition-colors"
              style={{
                backgroundColor: "var(--bg-primary)",
                borderColor: "var(--border-primary)",
                color: "var(--text-primary)",
              }}
              placeholder="name@example.com"
            />
          </div>

          {error && (
            <div
              className="text-sm text-center p-2 rounded border"
              style={{
                backgroundColor: "var(--error-bg)",
                color: "var(--error-text)",
                borderColor: "var(--error-border)",
              }}
            >
              {error}
            </div>
          )}

          <Button
            type="submit"
            variant="primary"
            disabled={loading}
            className="w-full justify-center"
          >
            {loading ? "Signing in..." : "Sign In"}
          </Button>
        </form>
      </div>
    </div>
  );
};

export default Login;
