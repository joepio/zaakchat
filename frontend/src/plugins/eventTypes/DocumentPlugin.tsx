import React from "react";
import type { EventPluginProps } from "./types";
import type { Document } from "../../types";
import {
  ResourcePluginWrapper,
  EventPluginWrapper,
} from "../shared/TimelineEventUI";
import SchemaEditFormContent from "../../components/SchemaEditFormContent";
import DeletedItem from "../../components/DeletedItem";
import { Button } from "../../components/ActionButton";
import { useSSE } from "../../contexts/SSEContext";

const DocumentPlugin: React.FC<EventPluginProps> = ({
  event,
  data,
  timeInfo,
}) => {
  const { sendEvent, items } = useSSE();
  const eventData = data as Record<string, unknown>;

  // Support both new (resource_id) and old (item_id) field names
  const documentId = (eventData.resource_id || eventData.item_id) as string;

  if (!documentId) {
    return <p>Document ID not found</p>;
  }

  // Get the current state of the document from the items store
  const documentData = items[documentId] as Partial<Document> | undefined;

  console.log("doc data", documentData);

  // If the document is not in the store, it's been deleted
  if (!documentData) {
    return (
      <DeletedItem
        itemId={documentId}
        itemType="document"
        actor={event.actor || "onbekend"}
        timeLabel={timeInfo.relative}
        onTimeClick={() => {}} // Will be handled by wrapper
        title={undefined} // We don't have the title since the item is deleted
      />
    );
  }

  // Handle create events with incomplete data - these are simple one-liners
  if (!documentData.title || !documentData.url) {
    return (
      <EventPluginWrapper event={event} data={data} timeInfo={timeInfo}>
        <span>document toegevoegd</span>
      </EventPluginWrapper>
    );
  }

  // Format file size
  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  const handleDownload = () => {
    if (documentData.url) {
      // Create a temporary anchor element to trigger download
      const link = document.createElement("a");
      link.href = documentData.url;
      link.download = documentData.title || "document";
      link.target = "_blank";
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
    }
  };

  const editFormComponent = (
    <SchemaEditFormContent
      itemType="document"
      itemId={documentId} // Use documentId here
      initialData={{
        title: documentData.title,
        url: documentData.url,
        size: documentData.size,
      }}
      onSubmit={sendEvent}
      onCancel={() => {}} // Will be handled by wrapper
      zaakId={event.originalEvent.subject || ""}
    />
  );

  return (
    <ResourcePluginWrapper
      event={event}
      data={data}
      timeInfo={timeInfo}
      actionText="document toegevoegd"
      editFormComponent={editFormComponent}
      showEditButton={true}
      editModalTitle="Document bewerken"
    >
      <div className="flex flex-col sm:flex-row sm:items-start gap-3 sm:gap-4">
        <div className="flex items-center gap-3 flex-1 min-w-0">
          <span className="text-xl">
            <i className="fa-regular fa-file-lines" aria-hidden="true"></i>
          </span>
          <div className="flex-1 min-w-0">
            <h4
              className="font-semibold m-0 leading-tight text-base sm:text-lg lg:text-xl xl:text-2xl"
              style={{ color: "var(--text-primary)" }}
            >
              {documentData.title}
            </h4>
            {documentData.size && (
              <p
                className="text-xs sm:text-sm lg:text-sm xl:text-base m-0 mt-1"
                style={{ color: "var(--text-secondary)" }}
              >
                {formatFileSize(documentData.size)}
              </p>
            )}
          </div>
        </div>

        <Button
          variant="secondary"
          size="md"
          onClick={handleDownload}
          className="self-start sm:self-auto flex-shrink-0"
        >
          <span>
            <i className="fa-solid fa-download" aria-hidden="true"></i>
          </span>
          Download
        </Button>
      </div>
    </ResourcePluginWrapper>
  );
};

export default DocumentPlugin;
