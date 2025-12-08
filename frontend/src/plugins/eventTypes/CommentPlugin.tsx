import React from "react";
import type { EventPluginProps } from "./types";
import type { Comment } from "../../types";
import { ResourcePluginWrapper } from "../shared/TimelineEventUI";
import SchemaEditFormContent from "../../components/SchemaEditFormContent";
import DeletedItem from "../../components/DeletedItem";
import { useSSE } from "../../contexts/SSEContext";
import MarkdownRenderer from "../../components/MarkdownRenderer";

const CommentPlugin: React.FC<EventPluginProps> = ({
  event,
  data,
  timeInfo,
}) => {
  const { sendEvent, items } = useSSE();

  // Get the current state of the comment from the items store
  // Support both new (resource_id) and old (item_id) field names
  const itemId = (data.resource_id || data.item_id) as string;

  if (!itemId) {
    return <p>Comment ID not found</p>;
  }

  const comment = items[itemId] as unknown as Comment;

  if (!comment) {
    return (
      <DeletedItem
        itemId={itemId}
        itemType="comment"
        actor={event.actor || "onbekend"}
        timeLabel={timeInfo.relative}
        onTimeClick={() => {}} // Will be handled by wrapper
      />
    );
  }

  const editFormComponent = (
    <SchemaEditFormContent
      itemType="comment"
      itemId={itemId || event.id}
      initialData={comment as unknown as Record<string, unknown>}
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
      actionText=""
      editFormComponent={editFormComponent}
      showEditButton={true}
    >
      <div className="prose max-w-none">
        <div
          className="m-0 mb-2 leading-relaxed text-sm sm:text-base lg:text-lg xl:text-xl"
          style={{ color: "var(--text-primary)" }}
        >
          {comment.content ? (
            <MarkdownRenderer content={comment.content} />
          ) : (
            "Geen inhoud"
          )}
        </div>
        {comment.mentions && comment.mentions.length > 0 && (
          <div className="mt-2">
            <small
              className="text-xs sm:text-sm lg:text-sm xl:text-base"
              style={{ color: "var(--text-tertiary)" }}
            >
              Vermeldingen: {comment.mentions.join(", ")}
            </small>
          </div>
        )}
      </div>
    </ResourcePluginWrapper>
  );
};

export default CommentPlugin;
