import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useMemo,
} from "react";
import type { ReactNode } from "react";

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

// Generate a random email-like actor for the session
const generateRandomActor = (): string => {
  const domains = ["gmail.com", "outlook.com"];
  const firstNames = [
    "alice",
    "bob",
    "charlie",
    "diana",
    "eve",
    "frank",
    "grace",
    "henry",
    "iris",
    "jack",
  ];
  const lastNames = [
    "jansen",
    "de-vries",
    "bakker",
    "visser",
    "smit",
    "meijer",
    "de-jong",
    "mulder",
    "de-groot",
    "janssen",
  ];

  const firstName = firstNames[Math.floor(Math.random() * firstNames.length)];
  const lastName = lastNames[Math.floor(Math.random() * lastNames.length)];
  const domain = domains[Math.floor(Math.random() * domains.length)];

  return `${firstName}.${lastName}@${domain}`;
};

export const ActorProvider: React.FC<ActorProviderProps> = ({ children }) => {
  const [actor, setActor] = useState<string>("");

  const updateActor = (newActor: string) => {
    setActor(newActor);
    localStorage.setItem("session-actor", newActor);
  };

  // Generate actor on mount or if not in localStorage
  useEffect(() => {
    const storedActor = localStorage.getItem("session-actor");
    if (storedActor) {
      setActor(storedActor);
    } else {
      const newActor = generateRandomActor();
      updateActor(newActor);
    }
  }, []);

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
    setActor: updateActor,
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
