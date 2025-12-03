import React, { useState } from "react";
import { useNavigate } from "react-router-dom";
import type { CloudEvent } from "../types";
import SchemaEditFormContent from "./SchemaEditFormContent";
import { useActor } from "../contexts/ActorContext";

interface CreateIssueFormProps {
  onCreateIssue: (event: CloudEvent) => Promise<void>;
  onCancel: () => void;
}

const CreateIssueForm: React.FC<CreateIssueFormProps> = ({ onCreateIssue, onCancel }) => {
  const navigate = useNavigate();
  const { actor } = useActor();
  const [error, setError] = useState<string>("");

  const handleSubmit = async (event: CloudEvent) => {
    try {
      setError("");

      // Extract the issue ID from the event data
      const issueData = event.data as { resource_data?: { id?: string } };
      const issueId = issueData.resource_data?.id;

      if (!issueId) {
        throw new Error("Failed to get issue ID from event");
      }

      // Submit the event
      await onCreateIssue(event);

      // Navigate to the newly created zaak
      navigate(`/zaak/${issueId}`);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Aanmaken van zaak mislukt",
      );
      throw err; // Re-throw so SchemaEditFormContent knows it failed
    }
  };

  // Pre-fill form with default values
  const initialData = {
    status: "open", // Default status for new issues
    involved: actor ? [actor] : [], // Pre-fill with current user
  };

  return (
    <div>
      {error && (
        <div
          className="mb-4 p-3 rounded-md text-sm"
          style={{
            backgroundColor: "var(--error-bg)",
            color: "var(--error-text)",
            border: "1px solid var(--error-border)",
          }}
        >
          {error}
        </div>
      )}

      <SchemaEditFormContent
        itemType="issue"
        initialData={initialData}
        onSubmit={handleSubmit}
        onCancel={onCancel}
        zaakId="" // Empty for new issues
        isCreateMode={true}
      />
    </div>
  );
};

export default CreateIssueForm;
