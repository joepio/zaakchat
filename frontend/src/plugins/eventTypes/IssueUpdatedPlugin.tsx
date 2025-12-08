import React from "react";
import type { EventPluginProps } from "./types";
import { EventPluginWrapper } from "../shared/TimelineEventUI";

const IssueUpdatedPlugin: React.FC<EventPluginProps> = ({
  event,
  data,
  timeInfo,
}) => {
  // Generate a clean summary of changes from item_data
  const itemData = data.item_data as Record<string, unknown>;
  const changeKeys = itemData
    ? Object.keys(itemData).filter(
        (key) => key !== "id" && key !== "created_at" && key !== "updated_at",
      )
    : [];

  let changeText: string;
  if (changeKeys.length === 0) {
    changeText = "zaak bijgewerkt";
  } else if (changeKeys.length === 1) {
    const key = changeKeys[0];
    const value = itemData[key];
    let valueText = String(value);

    // Special handling for common fields
    if (key === "title") {
      if (valueText.length > 30) {
        valueText = valueText.substring(0, 30) + "...";
      }
      changeText = `titel gewijzigd naar "${valueText}"`;
    } else if (key === "status") {
      changeText = `status gewijzigd naar "${valueText}"`;
    } else if (key === "assignee") {
      changeText =
        value && value !== null
          ? `toegewezen aan ${valueText}`
          : "toewijzing verwijderd";
    } else {
      if (valueText.length > 30) {
        valueText = valueText.substring(0, 30) + "...";
      }
      changeText = `${key} gewijzigd naar "${valueText}"`;
    }
  } else if (changeKeys.length === 2) {
    changeText = `${changeKeys[0]} en ${changeKeys[1]} gewijzigd`;
  } else {
    changeText = `${changeKeys.length} velden gewijzigd`;
  }

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

export default IssueUpdatedPlugin;
