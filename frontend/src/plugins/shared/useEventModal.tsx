import { useState } from "react";
import type { CloudEvent } from "../../types";

export interface EventModalState {
  showEventModal: boolean;
  showEditModal: boolean;
  setShowEventModal: (show: boolean) => void;
  setShowEditModal: (show: boolean) => void;
  openEventModal: () => void;
  closeEventModal: () => void;
  openEditModal: () => void;
  closeEditModal: () => void;
}

/**
 * Hook for managing event and edit modal state
 */
export const useEventModal = (): EventModalState => {
  const [showEventModal, setShowEventModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);

  const openEventModal = () => setShowEventModal(true);
  const closeEventModal = () => setShowEventModal(false);
  const openEditModal = () => setShowEditModal(true);
  const closeEditModal = () => setShowEditModal(false);

  return {
    showEventModal,
    showEditModal,
    setShowEventModal,
    setShowEditModal,
    openEventModal,
    closeEventModal,
    openEditModal,
    closeEditModal,
  };
};

/**
 * Props for the EventModalWrapper component
 */
export interface EventModalWrapperProps {
  event: CloudEvent;
  data: Record<string, unknown>;
  showEventModal: boolean;
  onCloseEventModal: () => void;
  showEditModal: boolean;
  onCloseEditModal: () => void;
  children: React.ReactNode;
  editFormComponent?: React.ReactNode;
}

/**
 * Wrapper component that provides consistent modal handling for all event plugins
 */
export const EventModalWrapper: React.FC<EventModalWrapperProps> = ({
  event,
  data,
  showEventModal,
  onCloseEventModal,
  showEditModal,
  onCloseEditModal,
  children,
  editFormComponent,
}) => {
  return (
    <>
      {children}

      {/* CloudEvent Modal */}
      <CloudEventModal
        open={showEventModal}
        onClose={onCloseEventModal}
        cloudEvent={event}
        schemaUrl={data?.schema as string | undefined}
      />

      {/* Edit Modal */}
      {editFormComponent && (
        <Modal
          isOpen={showEditModal}
          onClose={onCloseEditModal}
          title="Bewerken"
          maxWidth="600px"
        >
          {editFormComponent}
        </Modal>
      )}
    </>
  );
};

// Import Modal and CloudEventModal here to avoid circular dependencies
import Modal from "../../components/Modal";
import { CloudEventModal } from "./TimelineEventUI";
