import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useMemo,
  useRef,
  type ReactNode,
} from "react";
import type { CloudEvent, Issue } from "../types";
import { useActor } from "./ActorContext";
import { useAuth } from "../contexts/AuthContext";
import { createTaskCompletionEvent } from "../utils/taskUtils";
import toast from "react-hot-toast";

interface IssueWithActivity extends Issue {
  lastActivity?: string;
}

interface SSEContextType {
  events: CloudEvent[];
  issues: Record<string, IssueWithActivity>;
  items: Record<string, Record<string, unknown>>; // Unified item store for all item types
  connectionStatus: "connecting" | "connected" | "disconnected" | "error";
  errorMessage: string | null;
  retryConnection: () => void;
  sendEvent: (event: CloudEvent) => Promise<void>;
  completeTask: (taskId: string, issueId?: string) => Promise<void>;
}

const SSEContext = createContext<SSEContextType | undefined>(undefined);

interface SSEProviderProps {
  children: ReactNode;
}

export const SSEProvider: React.FC<SSEProviderProps> = ({ children }) => {
  const [events, setEvents] = useState<CloudEvent[]>([]);
  const [items, setItems] = useState<Record<string, Record<string, unknown>>>(
    {},
  );
  const [connectionStatus, setConnectionStatus] = useState<
    "connecting" | "connected" | "disconnected" | "error"
  >("connecting");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);
  const eventSourceRef = useRef<EventSource | null>(null);
  const retryTimeoutRef = useRef<number | null>(null);
  const { actor } = useActor();
  const { token, logout } = useAuth();

  // Derive issues from items store with lastActivity calculated from events
  const issues = useMemo(() => {
    const issuesMap: Record<string, IssueWithActivity> = {};
    const activityMap: Record<string, string> = {};
    const creationMap: Record<string, string> = {}; // To store creation time

    // Build activity and creation map from events
    for (const event of events) {
      if (event.subject) {
        const eventTime = event.time || new Date().toISOString();
        if (
          !activityMap[event.subject] ||
          eventTime > activityMap[event.subject]
        ) {
          activityMap[event.subject] = eventTime;
        }
        // Store the first event time as creation time
        if (!creationMap[event.subject]) {
          creationMap[event.subject] = eventTime;
        }
      }
    }

    // Extract issues from items and add lastActivity
    for (const [itemId, itemData] of Object.entries(items)) {
      // Check if this is an issue (issues have their ID as the item_id)
      if (itemData.status && itemData.title) {
        const issue = itemData as unknown as Issue;
        issuesMap[itemId] = {
          ...issue,
          lastActivity: activityMap[itemId] || creationMap[itemId],
        };
      }
    }

    return issuesMap;
  }, [items, events]);

  // Apply JSON Merge Patch (RFC 7396)
  const applyMergePatch = useCallback(
    (target: unknown, patch: unknown): unknown => {
      if (patch === null || typeof patch !== "object" || Array.isArray(patch)) {
        return patch;
      }

      if (
        target === null ||
        typeof target !== "object" ||
        Array.isArray(target)
      ) {
        target = {};
      }

      const result = { ...(target as Record<string, unknown>) };

      for (const [key, value] of Object.entries(
        patch as Record<string, unknown>,
      )) {
        if (value === null) {
          delete result[key];
        } else if (
          typeof value === "object" &&
          !Array.isArray(value) &&
          value !== null
        ) {
          result[key] = applyMergePatch(
            (target as Record<string, unknown>)[key],
            value,
          );
        } else {
          result[key] = value;
        }
      }

      return result;
    },
    [],
  );

  // Process a single CloudEvent into items store
  const processCloudEventToItems = useCallback(
    (
      cloudEvent: CloudEvent,
      items: Record<string, Record<string, unknown>>,
    ) => {
      if (!cloudEvent.data) return items;

      const JSONCommit = cloudEvent.data as Record<string, unknown>;
      const resourceId = (JSONCommit.resource_id ||
        JSONCommit.item_id) as string;
      if (!resourceId) return items;

      const newItems = { ...items };

      // Handle both old event types and new json.commit type
      if (cloudEvent.type === "json.commit") {
        const resourceData = JSONCommit.resource_data || JSONCommit.item_data;
        const patch = JSONCommit.patch;
        const deleted = JSONCommit.deleted;

        // Check if this is a deletion
        if (deleted === true) {
          delete newItems[resourceId];
        }
        // If resource_data exists, it's a create (full resource)
        else if (resourceData) {
          newItems[resourceId] = resourceData as Record<string, unknown>;
        }
        // If patch exists, apply it to existing resource or create new one
        else if (patch) {
          if (newItems[resourceId]) {
            const patched = applyMergePatch(
              newItems[resourceId],
              patch,
            ) as Record<string, unknown>;
            newItems[resourceId] = patched;
          } else {
            // Resource doesn't exist yet, create it with the patch data
            const patchData = patch as Record<string, unknown>;
            newItems[resourceId] = patchData;
          }
        }
      }

      return newItems;
    },
    [applyMergePatch],
  );

  // Process items (comments, tasks, planning, documents, etc.) into unified items store
  const processItemEvent = useCallback(
    (cloudEvent: CloudEvent) => {
      setItems((prevItems) => processCloudEventToItems(cloudEvent, prevItems));
    },
    [processCloudEventToItems],
  );

  // Process a single CloudEvent
  const processCloudEvent = useCallback(
    (cloudEvent: CloudEvent) => {
      // Process all items into the unified items store
      processItemEvent(cloudEvent);

      // Handle system reset
      if (cloudEvent.type === "system.reset") {
        window.location.reload();
      }
    },
    [processItemEvent],
  );

  // Process snapshot events to build initial state
  const processSnapshot = useCallback(
    (snapshotEvents: CloudEvent[]) => {
      let initialItems: Record<string, Record<string, unknown>> = {};

      for (const event of snapshotEvents) {
        initialItems = processCloudEventToItems(event, initialItems);
      }

      // Set the items state (issues will be derived from this)
      setItems(initialItems);
    },
    [processCloudEventToItems],
  );

  // Send CloudEvent to server
  const sendEvent = useCallback(
    async (event: CloudEvent) => {
      try {
        const headers: Record<string, string> = {
          "Content-Type": "application/json",
        };

        if (token) {
          headers["Authorization"] = `Bearer ${token}`;
        }

        const response = await fetch("/events", {
          method: "POST",
          headers,
          body: JSON.stringify(event),
        });

        if (!response.ok) {
          let errorMessage = `Failed to send event: ${response.statusText}`;
          try {
            const errorData = await response.json();
            if (errorData.error) {
              errorMessage = errorData.error;
            } else if (errorData.message) {
              errorMessage = errorData.message;
            }
          } catch (_e) {
            const text = await response.text();
            if (text) errorMessage = text;
          }
          throw new Error(errorMessage);
        }

        // Process the event locally for immediate UI responsiveness
        setEvents((prev) => [...prev, event]);
        processCloudEvent(event);
      } catch (error) {
        console.error("Error sending event:", error);
        toast.error(error instanceof Error ? error.message : "Failed to send event");
        throw error;
      }
    },
    [processCloudEvent, token],
  );

  // Complete a task
  const completeTask = useCallback(
    async (taskId: string, issueId?: string) => {
      try {
        if (!issueId) {
          throw new Error("Issue ID is required to complete a task");
        }

        const taskUpdateEvent = createTaskCompletionEvent(
          taskId,
          issueId,
          actor,
        );
        await sendEvent(taskUpdateEvent);
      } catch (error) {
        console.error("Error completing task:", error);
        throw error;
      }
    },
    [sendEvent, actor],
  );

  // Setup SSE connection handlers
  const setupEventSourceHandlers = useCallback(
    (eventSource: EventSource, url: string) => {
      eventSource.addEventListener("open", () => {
        setConnectionStatus("connected");
      });

      eventSource.addEventListener("error", async (event) => {
        console.error("[SSE] Connection error:", event);

        // Check if it's an auth error
        try {
            const res = await fetch(url);
            if (res.status === 401) {
                toast.error("Sessie verlopen. Log opnieuw in.");
                logout();
                eventSource.close();
                return;
            }
        } catch (e) {
            // Network error, continue to retry logic
        }

        setConnectionStatus("error");

        // Show toast for immediate feedback
        if (connectionStatus === "connected") {
           toast.error("Connection to server lost");
        } else if (retryCount === 0) {
           toast.error("Unable to connect to server");
        }

        // Determine error message based on retry count
        if (retryCount > 3) {
          setErrorMessage(
            "Unable to connect to server. Please check your connection and try again."
          );
        } else {
          setErrorMessage("Connection lost. Reconnecting...");
        }

        // Exponential backoff: 1s, 2s, 4s, 8s, max 10s
        const backoffDelay = Math.min(1000 * Math.pow(2, retryCount), 10000);

        if (retryTimeoutRef.current) {
          clearTimeout(retryTimeoutRef.current);
        }

        retryTimeoutRef.current = window.setTimeout(() => {
          setConnectionStatus("connecting");
          setRetryCount((prev) => prev + 1);
        }, backoffDelay);
      });

      // Handle snapshot (initial full state)
      eventSource.addEventListener("snapshot", (e) => {
        try {
          const events = JSON.parse(e.data) as CloudEvent[];

          setEvents(events);
          processSnapshot(events);
          setConnectionStatus("connected");
          setErrorMessage(null);
        } catch (error) {
          console.error("Error processing snapshot:", error);
          setErrorMessage("Failed to parse server data");
        }
      });

      // Handle deltas (live updates)
      eventSource.addEventListener("delta", (e) => {
        try {
          const cloudEvent = JSON.parse(e.data) as CloudEvent;

          // Check for system reset event
          if (cloudEvent.type === "system.reset") {
            window.location.reload();
            return;
          }

          // Debug: log all deltas
          console.log("[SSE DEBUG] Received delta:", cloudEvent);

          // Add to events list if not already present (prevent duplicates from local optimistic updates)
          setEvents((prevEvents) => {
            if (prevEvents.some(e => e.id === cloudEvent.id)) {
              return prevEvents;
            }
            return [...prevEvents, cloudEvent];
          });

          // Process the event to update issues state
          processCloudEvent(cloudEvent);
        } catch (error) {
          console.error("Error processing delta:", error);
        }
      });
    },
    [processSnapshot, processCloudEvent, retryCount],
  );

  // Connect to SSE endpoint
  useEffect(() => {
    const connectSSE = () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }

      setConnectionStatus("connecting");
      if (retryCount === 0) {
        setErrorMessage(null);
      }

      // Append token to URL if available
      const url = token ? `/events?token=${encodeURIComponent(token)}` : "/events";
      const eventSource = new EventSource(url);
      eventSourceRef.current = eventSource;

      setupEventSourceHandlers(eventSource, url);
    };

    connectSSE();

    // Cleanup on unmount
    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
      if (retryTimeoutRef.current) {
        clearTimeout(retryTimeoutRef.current);
      }
    };
  }, [setupEventSourceHandlers, token]);

  // Manual retry function
  const retryConnection = useCallback(() => {
    setRetryCount(0);
    setErrorMessage(null);
    if (retryTimeoutRef.current) {
      clearTimeout(retryTimeoutRef.current);
    }
    setConnectionStatus("connecting");
  }, []);

  const contextValue: SSEContextType = {
    events,
    issues,
    items,
    connectionStatus,
    errorMessage,
    retryConnection,
    sendEvent,
    completeTask,
  };

  return (
    <SSEContext.Provider value={contextValue}>{children}</SSEContext.Provider>
  );
};

export const useSSE = (): SSEContextType => {
  const context = useContext(SSEContext);
  if (context === undefined) {
    throw new Error("useSSE must be used within an SSEProvider");
  }
  return context;
};
