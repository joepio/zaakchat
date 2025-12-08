import React from "react";
import type { EventPluginProps } from "./types";
import { EventPluginWrapper } from "../shared/TimelineEventUI";
import PropertiesRenderer from "../../components/PropertiesRenderer";

const GenericResourcePlugin: React.FC<EventPluginProps> = ({
  event,
  data,
  timeInfo,
}) => {
  const schemaUrl =
    (data.schema as string) || (event.originalEvent?.dataschema as string);

  // Extract item type
  let itemType = "resource";
  if (schemaUrl) {
    const schemaName = schemaUrl.split("/").pop();
    if (schemaName) {
      itemType = schemaName.toLowerCase();
    }
  } else if (data.item_type) {
    itemType = data.item_type as string;
  } else if (data.resource_type) {
    itemType = data.resource_type as string;
  }

  // Determine action text
  const isCreate = !!(data.resource_data || data.item_data);
  const isDelete = data.deleted === true;
  const isUpdate = (data.patch && !isDelete) || event.type === "updated";

  let actionText = isUpdate ? `${itemType} bijgewerkt` : `System event: ${itemType}`;
  if (isCreate) actionText = `${itemType} aangemaakt`;
  if (isDelete) actionText = `${itemType} verwijderd`;

  return (
    <EventPluginWrapper
      event={event}
      data={data}
      timeInfo={timeInfo}
    >
      <div className="flex flex-col gap-2">
        <div className="text-sm italic" style={{ color: "var(--text-tertiary)" }}>
          {actionText}
        </div>
        <div className="mt-1 pl-2 border-l-2" style={{ borderColor: "var(--border-primary)" }}>
          <PropertiesRenderer
            data={data}
            schemaUrl={schemaUrl}
            ignoredProperties={[
              "id",
              "resource_id",
              "item_id",
              "created_at",
              "updated_at",
              "schema",
              "resource_type",
              "item_type",
              "patch",
              "resource_data",
              "item_data",
              "deleted"
            ]}
          />
        </div>
      </div>
    </EventPluginWrapper>
  );
};

export default GenericResourcePlugin;
