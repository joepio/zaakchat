import { useMemo } from "react";
import { useAuth } from "../contexts/AuthContext";

export const useUser = () => {
  const { user, login, logout, isAuthenticated } = useAuth();

  const { formattedUserName, userInitial } = useMemo(() => {
    if (!user) {
      return { formattedUserName: "Gebruiker", userInitial: "U" };
    }
    const userInitial = user.charAt(0).toUpperCase();
    const userName = user.split("@")[0].replace(".", " ");
    const formattedUserName = userName
      .split(" ")
      .map((name) => name.charAt(0).toUpperCase() + name.slice(1))
      .join(" ");
    return { formattedUserName, userInitial };
  }, [user]);

  return {
    user,
    login,
    logout,
    isAuthenticated,
    formattedUserName,
    userInitial,
  };
};
