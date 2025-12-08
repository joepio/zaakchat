import React, { useState, useRef, useEffect, useMemo } from "react";
import { Link } from "react-router-dom";
import { useSSE } from "../contexts/SSEContext";
import { useActor } from "../contexts/ActorContext";
import { usePushNotifications } from "../hooks/usePushNotifications";
import { Button } from "./ActionButton";

interface NotificationBellProps {
  currentZaakId?: string;
}

const NotificationBell: React.FC<NotificationBellProps> = ({
  currentZaakId,
}) => {
  const { events, issues, connectionStatus } = useSSE();
  const { actor } = useActor();
  const {
    isSupported,
    isSubscribed,
    permission,
    subscribe,
    unsubscribe,
  } = usePushNotifications();
  const [isNotificationOpen, setIsNotificationOpen] = useState(false);
  const [newEventsCount, setNewEventsCount] = useState(0);
  const [lastEventId, setLastEventId] = useState<string | null>(null);
  const [lastSeenTime, setLastSeenTime] = useState<string>(
    new Date().toISOString(),
  );
  const notificationRef = useRef<HTMLDivElement>(null);

  // Get recent activities (events for other issues) to show in dropdown
  const recentActivities = useMemo(() => {
    const recentEvents = events
      .filter((event) => {
        // Filter out events for current zaak
        if (!event.subject || event.subject === currentZaakId) {
          return false;
        }

        // Filter out events from current actor
        if (event.type === "json.commit" && event.data) {
          const data = event.data as any;
          const eventActor = data.actor;
          if (eventActor && eventActor === actor) {
            return false;
          }
        }

        return true;
      })
      .slice(-10) // Get last 10 events
      .reverse(); // Most recent first

    // Group by issue and show the most recent activity per issue
    const activitiesByIssue = new Map();

    recentEvents.forEach((event) => {
      const issueId = event.subject;
      const issue = issueId ? issues[issueId] : null;

      if (issue && !activitiesByIssue.has(issueId)) {
        let activityDescription = "Activiteit";

        if (event.type === "json.commit" && event.data) {
          const data = event.data as any;
          const schema = data.schema as string;
          const hasResourceData = !!data.resource_data || !!data.item_data;
          const hasPatch = !!data.patch;

          if (hasResourceData) {
            // New resource created
            if (schema?.endsWith("/Comment")) {
              activityDescription = "Nieuwe reactie";
            } else if (schema?.endsWith("/Task")) {
              activityDescription = "Nieuwe taak";
            } else if (schema?.endsWith("/Issue")) {
              activityDescription = "Nieuwe zaak";
            } else {
              activityDescription = "Nieuw item";
            }
          } else if (hasPatch) {
            // Resource updated
            activityDescription = "Update";
          }
        }

        activitiesByIssue.set(issueId, {
          issueId,
          issue,
          event,
          activityDescription,
          timestamp: event.time || new Date().toISOString(),
          isNew: (event.time || new Date().toISOString()) > lastSeenTime,
        });
      }
    });

    return Array.from(activitiesByIssue.values()).slice(0, 5);
  }, [events, issues, currentZaakId, actor, lastSeenTime]);

  // Close notification dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        notificationRef.current &&
        !notificationRef.current.contains(event.target as Node)
      ) {
        setIsNotificationOpen(false);
      }
    };

    if (isNotificationOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isNotificationOpen]);

  // Track new events and update notification counter
  useEffect(() => {
    if (events.length === 0) return;

    const latestEvent = events[events.length - 1];

    // Skip if this is the first load (no previous lastEventId)
    if (lastEventId === null) {
      setLastEventId(latestEvent.id);
      return;
    }

    // Skip if this event is the same as the last one we tracked
    if (latestEvent.id === lastEventId) {
      return;
    }

    // Only count events that are NOT for the current zaak (since we're viewing it)
    // and NOT from the current actor
    if (latestEvent.subject !== currentZaakId) {
      // Check if event is from current actor
      let isFromCurrentActor = false;
      if (latestEvent.type === "json.commit" && latestEvent.data) {
        const data = latestEvent.data as any;
        const eventActor = data.actor;
        if (eventActor && eventActor === actor) {
          isFromCurrentActor = true;
        }
      }

      if (!isFromCurrentActor) {
        setNewEventsCount((prev) => prev + 1);
      }
    }

    setLastEventId(latestEvent.id);
  }, [events, lastEventId, currentZaakId, actor]);

  // Reset notification counter when changing zaak
  useEffect(() => {
    setNewEventsCount(0);
    setLastEventId(null);
  }, [currentZaakId]);

  const handleBellClick = () => {
    setIsNotificationOpen(!isNotificationOpen);
    // Reset counter when opening notifications
    if (!isNotificationOpen) {
      setNewEventsCount(0);
      setLastSeenTime(new Date().toISOString());
    }
  };

  const getConnectionStatusText = () => {
    switch (connectionStatus) {
      case "connected":
        return "Verbonden";
      case "connecting":
        return "Verbinden...";
      case "disconnected":
        return "Verbinding verbroken";
      case "error":
        return "Fout";
      default:
        return "Onbekend";
    }
  };

  const getConnectionStatusColor = () => {
    switch (connectionStatus) {
      case "connected":
        return "#10b981"; // green
      case "connecting":
        return "#f59e0b"; // yellow
      case "disconnected":
        return "#ef4444"; // red
      case "error":
        return "#ef4444"; // red
      default:
        return "#6b7280"; // gray
    }
  };

  const handleTogglePushNotifications = async () => {
    if (isSubscribed) {
      await unsubscribe();
    } else {
      await subscribe();
    }
  };

  const getPushButtonContent = () => {
    if (isSubscribed)
      return (
        <>
          <i className="fa-regular fa-bell-slash" aria-hidden="true"></i>{" "}
          Notificaties uitschakelen
        </>
      );
    if (permission === "denied")
      return (
        <>
          <i className="fa-regular fa-bell-slash" aria-hidden="true"></i>{" "}
          Notificaties geblokkeerd
        </>
      );
    return (
      <>
        <i className="fa-regular fa-bell" aria-hidden="true"></i>{" "}
        Notificaties inschakelen
      </>
    );
  };

  return (
    <div
      className="relative ml-auto flex items-center"
      ref={notificationRef}
    >
      <Button
        variant="ghost"
        size="md"
        icon={true}
        onClick={handleBellClick}
        className="relative"
      >
        <i className="fa-solid fa-bell" aria-hidden="true"></i>
        {newEventsCount > 0 && (
          <span
            className="absolute top-0 right-0 text-xs font-semibold px-1 rounded-full min-w-[16px] text-center text-white border-2"
            style={{
              backgroundColor: "var(--text-error)",
              borderColor: "var(--bg-secondary)"
            }}
          >
            {newEventsCount}
          </span>
        )}
      </Button>

      {isNotificationOpen && (
        <div
          className="absolute top-full mt-1 right-0 rounded-lg shadow-theme-lg z-50 w-80 border"
          style={{
            backgroundColor: "var(--bg-primary)",
            borderColor: "var(--border-primary)",
          }}
        >
          <div
            className="p-4 border-b rounded-t-lg"
            style={{
              borderBottomColor: "var(--border-primary)",
              backgroundColor: "var(--bg-tertiary)",
            }}
          >
            <div className="flex items-center justify-between">
              <h3
                className="m-0 text-sm md:text-xs font-semibold"
                style={{ color: "var(--text-primary)" }}
              >
                Recente Activiteit
              </h3>
              <div className="flex items-center gap-2">
                <div
                  className="w-2 h-2 rounded-full"
                  style={{ backgroundColor: getConnectionStatusColor() }}
                  data-testid="connection-indicator"
                />
                <span
                  className="text-xs font-medium"
                  style={{ color: "var(--text-secondary)" }}
                  data-testid="connection-status"
                >
                  {getConnectionStatusText()}
                </span>
              </div>
            </div>
            {isSupported && (
              <Button
                variant={isSubscribed ? "secondary" : "primary"}
                size="sm"
                onClick={handleTogglePushNotifications}
                disabled={permission === "denied"}
                className="w-full text-xs mt-3"
                title={
                  permission === "denied"
                    ? "Notificaties zijn geblokkeerd. Schakel ze in via de browserinstellingen."
                    : ""
                }
              >
                {getPushButtonContent()}
              </Button>
            )}
          </div>
          <div className="max-h-[300px] overflow-y-auto">
            {recentActivities.length === 0 ? (
              <div
                className="block px-4 py-3 italic text-center cursor-default"
                style={{ color: "var(--text-tertiary)" }}
              >
                Geen recente activiteit
              </div>
            ) : (
              recentActivities.map((activity) => {
                const eventId = activity.event?.id as string | undefined;
                const hash = eventId ? `#${eventId}` : "";
                return (
                <Link
                  key={activity.issueId}
                  to={`/zaak/${activity.issueId}${hash}`}
                  className={`block px-4 py-3 border-b last:border-b-0 text-inherit no-underline transition-colors duration-200 ${
                    activity.isNew ? "relative" : ""
                  }`}
                  style={{
                    borderBottomColor: "var(--border-primary)",
                    backgroundColor: activity.isNew
                      ? "rgba(59, 130, 246, 0.05)"
                      : "",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.backgroundColor = activity.isNew
                      ? "rgba(59, 130, 246, 0.1)"
                      : "var(--bg-hover)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = activity.isNew
                      ? "rgba(59, 130, 246, 0.05)"
                      : "";
                  }}
                  onClick={() => setIsNotificationOpen(false)}
                >
                  {activity.isNew && (
                    <div
                      className="absolute left-2 top-1/2 transform -translate-y-1/2 w-2 h-2 rounded-full"
                      style={{ backgroundColor: "rgb(59, 130, 246)" }}
                    />
                  )}
                  <div
                    className={`font-medium mb-1 overflow-hidden text-ellipsis whitespace-nowrap md:text-sm ${
                      activity.isNew ? "ml-2" : ""
                    }`}
                    style={{ color: "var(--text-primary)" }}
                  >
                    {activity.issue.title || "Zaak zonder titel"}
                  </div>
                  <div
                    className={`text-xs md:text-xs flex items-center gap-1 ${
                      activity.isNew ? "ml-2" : ""
                    }`}
                    style={{ color: "var(--text-secondary)" }}
                  >
                    {activity.activityDescription}
                    {activity.isNew && (
                      <span
                        className="text-xs font-semibold px-1.5 py-0.5 rounded-full"
                        style={{
                          backgroundColor: "rgb(59, 130, 246)",
                          color: "white",
                        }}
                      >
                        Nieuw
                      </span>
                    )}
                  </div>
                </Link>
              )})
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default NotificationBell;
