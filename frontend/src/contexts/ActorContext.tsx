import React, {
  createContext,
  useContext,
  useEffect,
  useMemo,
} from "react";
import type { ReactNode } from "react";
import { useAuth } from "./AuthContext";

interface ActorContextType {
  actor: string;
  setActor: (actor: string) => void;
  formattedUserName: string;
  userInitial: string;
}

const ActorContext = createContext<ActorContextType | undefined>(undefined);

interface ActorProviderProps {
  children: ReactNode;
}

export const ActorProvider: React.FC<ActorProviderProps> = ({ children }) => {
  const { user, login } = useAuth();

  // Use the authenticated user as the actor
  const actor = user || "";

  // Map setActor to login
  const setActor = (newActor: string) => {
    login(newActor).catch(console.error);
  };

  // Listen for service worker requests for current actor
  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      if (event.data?.type === "GET_CURRENT_ACTOR") {
        // Respond with current actor
        if (event.ports && event.ports[0]) {
          event.ports[0].postMessage({ actor });
        }
      }
    };

    navigator.serviceWorker?.addEventListener("message", handleMessage);

    return () => {
      navigator.serviceWorker?.removeEventListener("message", handleMessage);
    };
  }, [actor]);

  const { formattedUserName, userInitial } = useMemo(() => {
    if (!actor) {
      return { formattedUserName: "Gebruiker", userInitial: "U" };
    }
    const userInitial = actor.charAt(0).toUpperCase();
    const userName = actor.split("@")[0].replace(".", " ");
    const formattedUserName = userName
      .split(" ")
      .map((name) => name.charAt(0).toUpperCase() + name.slice(1))
      .join(" ");
    return { formattedUserName, userInitial };
  }, [actor]);

  const value: ActorContextType = {
    actor,
    setActor,
    formattedUserName,
    userInitial,
  };

  return (
    <ActorContext.Provider value={value}>{children}</ActorContext.Provider>
  );
};

export const useActor = (): ActorContextType => {
  const context = useContext(ActorContext);
  if (context === undefined) {
    throw new Error("useActor must be used within an ActorProvider");
  }
  return context;
};
