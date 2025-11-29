import React, { useState } from "react";
import { useNavigate } from "react-router-dom";
import type { CloudEvent, IssueFormData, Issue } from "../types";
import { Button } from "./ActionButton";
import { generateUUID } from "../utils/uuid";
import { createIssueCreatedEvent } from "../utils/cloudEvents";
import { useActor } from "../contexts/ActorContext";

interface CreateIssueFormProps {
  onCreateIssue: (event: CloudEvent) => Promise<void>;
}

const CreateIssueForm: React.FC<CreateIssueFormProps> = ({ onCreateIssue }) => {
  const navigate = useNavigate();
  const { actor } = useActor();
  const [formData, setFormData] = useState<IssueFormData>({
    title: "",
    description: "",
    assignee: "",
  });

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string>("");

  const handleInputChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>,
  ) => {
    const { name, value } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: value,
    }));
    setError("");
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!formData.title.trim()) {
      setError("Titel is verplicht");
      return;
    }

    setIsSubmitting(true);
    setError("");

    try {
      const issueId = generateUUID();

      // Create an Issue object following the schema
      const issue: Issue = {
        id: issueId,
        title: formData.title.trim(),
        status: "open",
        created_at: new Date().toISOString(),
        description: formData.description.trim() || null,
        assignee: formData.assignee.trim() || null,
        resolution: null,
        involved: actor ? [actor] : [],
      };

      // Use the schema-based CloudEvent utility with session actor
      const cloudEvent = createIssueCreatedEvent(issue, { actor });

      await onCreateIssue(cloudEvent);

      // Navigate to the newly created zaak
      navigate(`/zaak/${issueId}`);

      // Reset form on success (in case navigation fails)
      setFormData({
        title: "",
        description: "",
        assignee: "",
      });
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Aanmaken van zaak mislukt",
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="flex flex-col md:flex-row gap-2 mb-4">
          <input
            type="text"
            name="title"
            value={formData.title}
            onChange={handleInputChange}
            placeholder="Zaak titel"
            required
            disabled={isSubmitting}
            className="form-input flex-1 px-3 py-2 text-base rounded-md border transition-colors duration-150 focus:outline-none disabled:opacity-60"
            style={{
              backgroundColor: "var(--bg-primary)",
              color: "var(--text-primary)",
              borderColor: "var(--border-primary)",
            }}
          />
          <input
            type="text"
            name="description"
            value={formData.description}
            onChange={handleInputChange}
            placeholder="Beschrijving (optioneel)"
            disabled={isSubmitting}
            className="form-input flex-1 px-3 py-2 text-base rounded-md border transition-colors duration-150 focus:outline-none disabled:opacity-60"
            style={{
              backgroundColor: "var(--bg-primary)",
              color: "var(--text-primary)",
              borderColor: "var(--border-primary)",
            }}
          />
        </div>

        <div className="flex flex-col md:flex-row gap-2 mb-4">
          <input
            type="email"
            name="assignee"
            value={formData.assignee}
            onChange={handleInputChange}
            placeholder="Toegewezen aan email (optioneel)"
            disabled={isSubmitting}
            className="form-input flex-1 px-3 py-2 text-base rounded-md border transition-colors duration-150 focus:outline-none disabled:opacity-60"
            style={{
              backgroundColor: "var(--bg-primary)",
              color: "var(--text-primary)",
              borderColor: "var(--border-primary)",
            }}
          />
        </div>

        {error && (
          <div
            className="px-4 py-3 mb-4 text-sm rounded border-l-4"
            style={{
              backgroundColor: "var(--bg-error)",
              color: "var(--text-error)",
              borderLeftColor: "var(--text-error)",
            }}
          >
            {error}
          </div>
        )}

        <Button
          type="submit"
          variant="primary"
          size="md"
          disabled={isSubmitting}
          loading={isSubmitting}
        >
          {isSubmitting ? "Aanmaken..." : "Zaak Aanmaken"}
        </Button>
      </form>
    </div>
  );
};

export default CreateIssueForm;
