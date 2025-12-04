import React, { createContext, useState } from "react";

interface AuthContextType {
  token: string | null;
  user: string | null;
  login: (email: string) => Promise<void>;
  verifyLogin: (token: string) => Promise<void>;
  logout: () => void;
  isAuthenticated: boolean;
}

export const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [token, setToken] = useState<string | null>(
    localStorage.getItem("auth_token")
  );
  const [user, setUser] = useState<string | null>(
    localStorage.getItem("auth_user")
  );

  const login = async (email: string) => {
    const response = await fetch("/login", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ email }),
    });

    if (!response.ok) {
      throw new Error("Login failed");
    }
    // No token returned here anymore
  };

  const verifyLogin = async (verifyToken: string) => {
    const response = await fetch(`/auth/verify?token=${verifyToken}`);

    if (!response.ok) {
      throw new Error("Verification failed");
    }

    const data = await response.json();
    // Decode JWT to get email (sub)
    const payload = JSON.parse(atob(data.token.split('.')[1]));
    const email = payload.sub;

    setToken(data.token);
    setUser(email);
    localStorage.setItem("auth_token", data.token);
    localStorage.setItem("auth_user", email);
  };

  const logout = () => {
    setToken(null);
    setUser(null);
    localStorage.removeItem("auth_token");
    localStorage.removeItem("auth_user");
  };

  return (
    <AuthContext.Provider
      value={{
        token,
        user,
        login,
        verifyLogin,
        logout,
        isAuthenticated: !!token,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
};
