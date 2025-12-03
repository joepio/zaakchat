import React, { useState } from "react";
import { Button } from "../../components/ActionButton";
import Modal from "../../components/Modal";
import InfoHelp from "../../components/InfoHelp";
import Card from "../../components/Card";

export interface EventHeaderProps {
  actor?: string;
  timeLabel: string;
  onTimeClick: () => void;
  rightExtra?: React.ReactNode; // e.g., edit button
}

export const EventHeader: React.FC<EventHeaderProps> = ({
  actor,
  timeLabel,
  onTimeClick,
  rightExtra,
}) => (
  <div className="flex items-center justify-between gap-4 w-full mb-3">
    {actor && actor !== "system" ? (
      <span className="font-semibold text-sm sm:text-base lg:text-lg xl:text-xl">{actor}</span>
    ) : (
      <span />
    )}
    <div className="flex items-center gap-2">
      {rightExtra}
      <Button variant="link" size="sm" onClick={onTimeClick}>
        {timeLabel}
      </Button>
    </div>
  </div>
);

interface CloudEventModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  cloudEvent: unknown;
  schemaUrl?: string;
}

export const CloudEventModal: React.FC<CloudEventModalProps> = ({
  open,
  onClose,
  title = "CloudEvent",
  cloudEvent,
  schemaUrl,
}) => (
  <Modal isOpen={open} onClose={onClose} title={title} maxWidth="800px">
    <div className="relative">
      <InfoHelp variant="cloudevent" schemaUrl={schemaUrl} />
      <pre
        className="border rounded-md p-4 font-mono text-xs sm:text-sm lg:text-base xl:text-lg leading-relaxed overflow-x-auto m-0 whitespace-pre-wrap break-words"
        style={{
          backgroundColor: "var(--bg-tertiary)",
          borderColor: "var(--border-primary)",
          color: "var(--text-primary)",
        }}
      >
        {JSON.stringify(cloudEvent, null, 2)}
      </pre>
    </div>
  </Modal>
);

/**
 * A comprehensive wrapper for event plugins that handles all common modal patterns
 */
export interface EventPluginWrapperProps {
  event: any; // CloudEvent from timeline
  data: Record<string, unknown>;
  timeInfo: { relative: string; date: string; time: string };
  children: React.ReactNode;
  editFormComponent?: React.ReactNode;
  showEditButton?: boolean;
  editModalTitle?: string;
  onDelete?: () => void;
  showDeleteButton?: boolean;
}

export const EventPluginWrapper: React.FC<EventPluginWrapperProps> = ({
  event,
  data,
  timeInfo,
  children,
  editFormComponent,
  showEditButton = false,
  onDelete,
  showDeleteButton = false,
}) => {
  const [showEventModal, setShowEventModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);

  return (
    <>
      <div className="flex items-center justify-between w-full py-2">
        <span
          className="text-sm sm:text-base lg:text-lg xl:text-xl"
          style={{ color: "var(--text-secondary)" }}
        >
          {event.actor && event.actor !== "system" && (
            <strong style={{ color: "var(--text-primary)" }}>
              {event.actor}
            </strong>
          )}{" "}
          {children}
        </span>
        <div className="flex items-center gap-2">
          {showEditButton && editFormComponent && (
            <Button
              variant="link"
              size="sm"
              onClick={() => setShowEditModal(true)}
              title="Bewerken"
            >
              <i className="fa-solid fa-pen" aria-hidden="true"></i>
            </Button>
          )}
          {showDeleteButton && onDelete && (
            <Button
              variant="link"
              size="sm"
              onClick={onDelete}
              title="Verwijderen"
              className="text-red-500 hover:text-red-700"
            >
              <i className="fa-solid fa-trash" aria-hidden="true"></i>
            </Button>
          )}
          <Button
            variant="link"
            size="sm"
            title={`${timeInfo.date} at ${timeInfo.time}`}
            onClick={() => setShowEventModal(true)}
          >
            {timeInfo.relative}
          </Button>
        </div>
      </div>

      {/* CloudEvent Modal */}
      <CloudEventModal
        open={showEventModal}
        onClose={() => setShowEventModal(false)}
        cloudEvent={event.originalEvent}
        schemaUrl={data?.schema as string | undefined}
      />

      {/* Edit Modal */}
      {editFormComponent && (
        <Modal
          isOpen={showEditModal}
          onClose={() => setShowEditModal(false)}
          title="Bewerken"
          maxWidth="600px"
        >
          {React.isValidElement(editFormComponent) &&
            React.cloneElement(editFormComponent, {
              onCancel: () => setShowEditModal(false)
            } as any)
          }
        </Modal>
      )}
    </>
  );
};

/**
 * Wrapper for new resources (documents, comments) that need to be displayed as cards
 */
export interface ResourcePluginWrapperProps {
  event: any; // CloudEvent from timeline
  data: Record<string, unknown>;
  timeInfo: { relative: string; date: string; time: string };
  actionText: string; // The action text (e.g., "document toegevoegd", "reactie toegevoegd")
  children: React.ReactNode; // The resource content
  editFormComponent?: React.ReactNode;
  showEditButton?: boolean;
  editModalTitle?: string;
  onDelete?: () => void;
  showDeleteButton?: boolean;
}

export const ResourcePluginWrapper: React.FC<ResourcePluginWrapperProps> = ({
  event,
  data,
  timeInfo,
  actionText,
  children,
  editFormComponent,
  showEditButton = false,
  editModalTitle = "Bewerken",
  onDelete,
  showDeleteButton = false,
}) => {
  const [showEventModal, setShowEventModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);

  return (
    <>
      <Card padding="sm" id={data.resource_id as string || data.item_id as string}>
        <div className="flex items-center justify-between w-full mb-3">
          <span
            className="text-sm sm:text-base lg:text-lg xl:text-xl"
            style={{ color: "var(--text-secondary)" }}
          >
            {event.actor && event.actor !== "system" && (
              <strong style={{ color: "var(--text-primary)" }}>
                {event.actor}
              </strong>
            )}{" "}
            {actionText}
          </span>
          <div className="flex items-center gap-2">
            {showEditButton && editFormComponent && (
              <Button
                variant="link"
                size="sm"
                onClick={() => setShowEditModal(true)}
                title="Bewerken"
              >
                <i className="fa-solid fa-pen" aria-hidden="true"></i>
              </Button>
            )}
            {showDeleteButton && onDelete && (
              <Button
                variant="link"
                size="sm"
                onClick={onDelete}
                title="Verwijderen"
                className="text-red-500 hover:text-red-700"
              >
                <i className="fa-solid fa-trash" aria-hidden="true"></i>
              </Button>
            )}
            <Button
              variant="link"
              size="sm"
              title={`${timeInfo.date} at ${timeInfo.time}`}
              onClick={() => setShowEventModal(true)}
            >
              {timeInfo.relative}
            </Button>
          </div>
        </div>
        {children}
      </Card>

      {/* CloudEvent Modal */}
      <CloudEventModal
        open={showEventModal}
        onClose={() => setShowEventModal(false)}
        cloudEvent={event.originalEvent}
        schemaUrl={data?.schema as string | undefined}
      />

      {/* Edit Modal */}
      {editFormComponent && (
        <Modal
          isOpen={showEditModal}
          onClose={() => setShowEditModal(false)}
          title={editModalTitle}
          maxWidth="600px"
        >
          {React.isValidElement(editFormComponent) &&
            React.cloneElement(editFormComponent, {
              onCancel: () => setShowEditModal(false)
            } as any)
          }
        </Modal>
      )}
    </>
  );
};
