import React from "react";
import type { CloudEvent, TimelineEvent, TimelineItemType } from "../types";
import TimelineItem from "./TimelineItem";
import UserAvatar from "./UserAvatar";

interface TimelineEventsListProps {
  events: TimelineEvent[];
  getTimelineItemType: (event: CloudEvent) => TimelineItemType;
  showSystemEvents: boolean;
}

const TimelineEventsList: React.FC<TimelineEventsListProps> = ({
  events,
  getTimelineItemType,
  showSystemEvents,
}) => {
  // const [showSystemEvents, setShowSystemEvents] = useState(false); // Moved to parent

  if (events.length === 0) {
    return (
      <div className="text-center py-6 sm:py-8">
        <p
          className="text-sm sm:text-base lg:text-base xl:text-lg"
          style={{ color: "var(--text-tertiary)" }}
        >
          Geen tijdlijn gebeurtenissen gevonden.
        </p>
      </div>
    );
  }

  // Define what counts as a system event that should be hidden by default
  const isSystemEvent = (itemType: TimelineItemType) => {
    return [
      "issue_created",
      "issue_updated",
      "issue_deleted",
      "field_update",
      "status_change",
      "system_event",
      "system_update"
    ].includes(itemType);
  };

  const filteredEvents = events.filter((event) => {
    if (showSystemEvents) return true;
    const itemType = getTimelineItemType(event.originalEvent);
    return !isSystemEvent(itemType);
  });



  return (
    <div className="relative">


      {/* Timeline line - responsive positioning to center on avatars */}
      <div
        className="absolute left-5 sm:left-4 lg:left-5 xl:left-6 top-10 bottom-0 w-0.5 z-10"
        style={{ backgroundColor: "var(--border-primary)" }}
      />

      {filteredEvents.map((event) => {
        const itemType = getTimelineItemType(event.originalEvent);
        return (
          <div
            key={event.id}
            id={event.id}
            className="flex mb-4 sm:mb-5 lg:mb-6 xl:mb-8 relative z-20 animate-timeline-appear"
            data-testid="timeline-event"
          >
            <div className="flex-shrink-0 mr-3 sm:mr-4 lg:mr-4 xl:mr-5 w-10 sm:w-8 lg:w-10 xl:w-12">
              <UserAvatar
                name={event.actor || "?"}
                className="w-10 h-10 sm:w-8 sm:h-8 lg:w-10 lg:h-10 xl:w-12 xl:h-12 text-sm sm:text-xs lg:text-sm xl:text-base"
              />
            </div>
            <div className="flex-1 min-w-0">
                <TimelineItem event={event} itemType={itemType} />
            </div>
          </div>
        );
      })}


    </div>
  );
};

export default TimelineEventsList;
