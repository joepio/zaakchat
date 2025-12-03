import React, { useMemo, useState, useEffect } from "react";
import { useParams, Link, useLocation } from "react-router-dom";
import { useSSE } from "../contexts/SSEContext";
import type { CloudEvent, TimelineEvent, TimelineItemType } from "../types";

import IssueHeader from "./IssueHeader";
import PageHeader from "./PageHeader";
import ActiveTaskSection from "./ActiveTaskSection";
import ActivePlanningSection from "./ActivePlanningSection";
import CommentForm from "./CommentForm";
import TimelineEventsList from "./TimelineEventsList";
import SchemaForm from "./SchemaForm";
import SchemaEditForm from "./SchemaEditForm";
import SectionLabel from "./SectionLabel";

const IssueTimeline: React.FC = () => {
  const { zaakId } = useParams<{ zaakId: string }>();
  const location = useLocation();
  const { events, issues, sendEvent } = useSSE();

  const [showEditModal, setShowEditModal] = useState(false);

  const issue = zaakId ? issues[zaakId] : null;

  // Scroll to item if hash is present in URL
  useEffect(() => {
    if (location.hash) {
      const itemId = location.hash.substring(1); // Remove the '#'
      const element = document.getElementById(itemId);
      if (element) {
        setTimeout(() => {
          element.scrollIntoView({ behavior: "smooth", block: "center" });
          // Add a subtle highlight effect
          const originalBg = window.getComputedStyle(element).backgroundColor;
          element.style.transition = "background-color 0.3s ease";
          element.style.backgroundColor = "rgba(251, 191, 36, 0.2)";
          setTimeout(() => {
            element.style.backgroundColor = originalBg;
          }, 2000);
        }, 100);
      }
    }
  }, [location.hash]);

  // Convert CloudEvents to TimelineEvents for this specific issue
  const timelineEvents = useMemo(() => {
    if (!zaakId) return [];

    return events
      .filter((event) => event.subject === zaakId)
      .map((event): TimelineEvent => {
        const timestamp = event.time || new Date().toISOString();
        let type: "created" | "updated" | "deleted" = "created";
        let actor = "system";

        // Extract actor from event data
        if (
          event.data &&
          typeof event.data === "object" &&
          event.data !== null
        ) {
          const data = event.data as Record<string, unknown>;
          if (data.actor && typeof data.actor === "string") {
            actor = data.actor;
          } else if (data.assignee && typeof data.assignee === "string") {
            actor = data.assignee;
          }
        }

        // Determine event type based on CloudEvent type
        if (event.type.includes("patch") || event.type.includes("updated")) {
          type = "updated";
        } else if (event.type.includes("delete")) {
          type = "deleted";
        }

        return {
          id: event.id,
          type,
          timestamp,
          actor,
          data: event.data || {},
          originalEvent: event,
        };
      })
      .sort(
        (a, b) =>
          new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
      );
  }, [events, zaakId]);

  // Determine timeline item type from CloudEvent
  const getTimelineItemType = (event: CloudEvent): TimelineItemType => {
    // Check for json.commit or item events
    if (
      event.type === "json.commit"
    ) {
      if (event.data && typeof event.data === "object" && event.data !== null) {
        const data = event.data as Record<string, unknown>;

        // Extract schema name from schema URL (e.g., "http://localhost:8000/schemas/Comment" -> "comment")
        // Falls back to item_type for backwards compatibility
        let itemType: string;
        if (data.schema && typeof data.schema === "string") {
          const schemaUrl = data.schema as string;
          const schemaName = schemaUrl.split("/").pop() || "";
          itemType = schemaName.toLowerCase();
        } else if (data.item_type) {
          itemType = data.item_type as string;
        } else {
          return "system_event";
        }

        // Determine if this is a create, update, or delete based on presence of fields
        const isCreate = !!data.resource_data || !!data.item_data;
        const isUpdate = !!data.patch && !data.resource_data && !data.item_data;
        const isDelete = data.deleted === true;

        // Map item types to timeline types
        if (itemType === "issue") {
          if (isCreate) return "issue_created";
          if (isUpdate) return "issue_updated";
          if (isDelete) return "issue_deleted";
        }

        // For updates/deletes of other items, use system_event (renders as line)
        if (isUpdate || isDelete) {
          return "system_event";
        }

        // For creates, use the item type (renders as card with full content)
        return (itemType as TimelineItemType) || "system_event";
      }
    }

    return "system_event";
  };

  const handleCommentSubmit = async (event: CloudEvent) => {
    await sendEvent(event);
  };

  const handleEditIssue = () => {
    setShowEditModal(true);
  };

  if (!zaakId) {
    return (
      <div className="min-h-screen flex flex-col items-center justify-center text-center p-8 bg-bg-primary">
        <h1 className="text-3xl text-text-primary mb-4">Zaak niet gevonden</h1>
        <Link
          to="/"
          className="text-link-primary hover:text-link-hover hover:underline font-medium"
        >
          ← Terug naar Zaken
        </Link>
      </div>
    );
  }

  if (!issue) {
    return (
      <div className="min-h-screen flex flex-col items-center justify-center text-center p-8 bg-bg-primary">
        <h1 className="text-3xl text-text-primary mb-4">Zaak laden...</h1>
        <Link
          to="/"
          className="text-link-primary hover:text-link-hover hover:underline font-medium"
        >
          ← Terug naar Zaken
        </Link>
      </div>
    );
  }

  return (
    <div
      className="min-h-screen font-sans"
      style={{ backgroundColor: "var(--bg-primary)" }}
    >
      <PageHeader currentZaakId={zaakId} />

      {/* Main content */}
      <div className="max-w-3xl lg:max-w-4xl xl:max-w-5xl mx-auto p-4 md:p-8 lg:p-12 xl:p-16 pt-8 lg:pt-12 xl:pt-16">
        {/* Zaak header - show as standalone section like active task and planning */}
        <div
          id={zaakId}
          className="mb-6 md:mb-8 lg:mb-10 xl:mb-12 relative"
          data-testid="issue-header"
        >
          {issue && (
            <IssueHeader
              issue={issue}
              onEdit={handleEditIssue}
            />
          )}
        </div>

        {/* Active task section - completely separate from timeline */}
        {zaakId && <ActiveTaskSection events={events} zaakId={zaakId} />}

        {/* Active planning section - completely separate from timeline */}
        {zaakId && <ActivePlanningSection events={events} zaakId={zaakId} />}

        {/* Timeline section */}
        {timelineEvents.length > 0 && (
          <div className="mb-6 lg:mb-8 xl:mb-10">
            <SectionLabel>Tijdlijn</SectionLabel>
            <TimelineEventsList
              events={timelineEvents}
              getTimelineItemType={getTimelineItemType}
            />
          </div>
        )}

        {/* Comment form */}
        {zaakId && (
          <CommentForm zaakId={zaakId} onSubmit={handleCommentSubmit} />
        )}

        {/* Schema-driven form for creating new items */}
        {zaakId && (
          <div data-testid="schema-form-section">
            <SchemaForm zaakId={zaakId} onSubmit={handleCommentSubmit} />
          </div>
        )}
      </div>

      {/* Issue Edit Modal */}
      {zaakId && issue && (
        <SchemaEditForm
          isOpen={showEditModal}
          onClose={() => setShowEditModal(false)}
          itemType="issue"
          itemId={issue.id}
          initialData={{
            title: issue.title,
            description: issue.description,
            status: issue.status,
            assignee: issue.assignee,
            resolution: issue.resolution,
            involved: issue.involved,
          }}
          onSubmit={handleCommentSubmit}
          zaakId={zaakId}
        />
      )}
    </div>
  );
};

export default IssueTimeline;
