import React from "react";
import type { EventPluginProps } from "./types";
import { EventPluginWrapper } from "../shared/TimelineEventUI";

const IssueCreatedPlugin: React.FC<EventPluginProps> = ({
  event,
  data,
  timeInfo,
}) => {
  const changeText = "zaak aangemaakt";

  const resourceId = data.resource_id as string;

  return (
    <EventPluginWrapper
      event={event}
      data={data}
      timeInfo={timeInfo}
    >
        {resourceId ? (
            <a
                href={`#${resourceId}`}
                className="hover:underline hover:text-blue-600 transition-colors cursor-pointer"
                onClick={(e) => {
                    const element = document.getElementById(resourceId);
                    if (element) {
                        e.preventDefault();
                        element.scrollIntoView({ behavior: "smooth", block: "center" });
                    }
                }}
            >
                {changeText}
            </a>
        ) : (
            <span>{changeText}</span>
        )}
    </EventPluginWrapper>
  );
};

export default IssueCreatedPlugin;
