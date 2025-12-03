import React from "react";
import Modal from "./Modal";
import SchemaEditFormContent from "./SchemaEditFormContent";
import InfoHelp from "./InfoHelp";
import type { CloudEvent } from "../types/interfaces";

interface SchemaEditFormProps {
  isOpen: boolean;
  onClose: () => void;
  itemType: string;
  itemId: string;
  initialData: Record<string, unknown>;
  onSubmit: (event: CloudEvent) => Promise<void>;
  zaakId: string;
}

// Default labels for known types
const DEFAULT_LABELS: Record<string, string> = {
  issue: "Zaak",
  task: "Taak",
  comment: "Reactie",
  planning: "Planning",
  document: "Document",
  cloudevent: "CloudEvent",
  jsoncommit: "JSONCommit",
  issuestatus: "IssueStatus",
  planningstatus: "PlanningStatus",
  planningmoment: "PlanningMoment",
  itemtype: "ItemType",
};

const SchemaEditForm: React.FC<SchemaEditFormProps> = ({
  isOpen,
  onClose,
  itemType,
  itemId,
  initialData,
  onSubmit,
  zaakId,
}) => {
  if (!isOpen) return null;

  const typeLabel = DEFAULT_LABELS[itemType.toLowerCase()] || itemType;
  const schemaUrl = `http://localhost:8000/schemas/${itemType.charAt(0).toUpperCase() + itemType.slice(1).toLowerCase()}`;

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`${typeLabel} bewerken`}
      maxWidth="600px"
      headerActions={<InfoHelp variant="schemas" schemaUrl={schemaUrl} />}
    >
      <div style={{ backgroundColor: "var(--bg-secondary)" }}>
        <SchemaEditFormContent
          itemType={itemType}
          itemId={itemId}
          initialData={initialData}
          onSubmit={onSubmit}
          onCancel={onClose}
          zaakId={zaakId}
        />
      </div>
    </Modal>
  );
};

export default SchemaEditForm;
