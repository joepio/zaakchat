import React, {
  createContext,
  useContext,
  useEffect,
} from "react";
import type { ReactNode } from "react";
import { useAuth } from "../contexts/AuthContext";

interface ActorContextType {
  actor: string;
  setActor: (actor: string) => void;
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

  const value: ActorContextType = {
    actor,
    setActor,
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
