import React, { useState, useEffect } from "react";
import { fetchSchema } from "../types/interfaces";
import { Button } from "./ActionButton";
import SchemaField from "./SchemaField";
import InfoHelp from "./InfoHelp";
import { createItemUpdatedEvent, createItemDeletedEvent } from "../utils/cloudEvents";
import { useActor } from "../contexts/ActorContext";
import type { ItemType } from "../types";
import type { CloudEvent } from "../types/interfaces";

interface SchemaEditFormContentProps {
  itemType: string;
  itemId: string;
  initialData: Record<string, unknown>;
  onSubmit: (event: CloudEvent) => Promise<void>;
  onCancel: () => void;
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

const SchemaEditFormContent: React.FC<SchemaEditFormContentProps> = ({
  itemType,
  itemId,
  initialData,
  onSubmit,
  onCancel,
  zaakId,
}) => {
  const { actor } = useActor();
  const [formData, setFormData] = useState<Record<string, unknown>>({});
  const [changedFields, setChangedFields] = useState<Set<string>>(new Set());
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [currentSchema, setCurrentSchema] = useState<Record<string, unknown> | null>(null);

  // Initialize form data when component mounts
  useEffect(() => {
    setFormData(initialData);
    setChangedFields(new Set()); // Reset changed fields when initialData changes
  }, [initialData]);

  // Load schema on mount
  useEffect(() => {
    const loadSchema = async () => {
      try {
        // Capitalize the first letter to match backend schema names
        const schemaName =
          itemType.charAt(0).toUpperCase() + itemType.slice(1).toLowerCase();
        console.log(
          `Loading schema for item type: ${itemType} -> ${schemaName}`,
        );
        const schema = await fetchSchema(schemaName as ItemType);
        console.log(`Loaded schema for ${schemaName}:`, schema);
        setCurrentSchema(schema);
      } catch (error) {
        console.error("Error loading schema:", error);
      }
    };

    loadSchema();
  }, [itemType]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!currentSchema || isSubmitting) return;

    setIsSubmitting(true);
    try {
      // Only send the fields that were actually changed
      const patch: Record<string, unknown> = {};
      changedFields.forEach(fieldName => {
        patch[fieldName] = formData[fieldName];
      });

      console.log('Submitting patch with only changed fields:', patch);

      const event = createItemUpdatedEvent(
        itemType as ItemType,
        itemId,
        patch,
        { actor, subject: zaakId }
      );
      await onSubmit(event);
      onCancel(); // Close the modal after successful submission
    } catch (error) {
      console.error("Error updating item:", error);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDelete = async () => {
    const typeLabel = DEFAULT_LABELS[itemType.toLowerCase()] || itemType;
    if (!confirm(`Weet je zeker dat je dit ${typeLabel.toLowerCase()} wilt verwijderen?`)) return;

    setIsDeleting(true);
    try {
      const event = createItemDeletedEvent(
        itemType as ItemType,
        itemId,
        { actor, subject: zaakId }
      );
      await onSubmit(event);
      onCancel(); // Close the modal after successful deletion
    } catch (error) {
      console.error("Error deleting item:", error);
    } finally {
      setIsDeleting(false);
    }
  };

  const handleFieldChange = (fieldName: string, value: unknown) => {
    setFormData(prev => ({
      ...prev,
      [fieldName]: value,
    }));
    // Track that this field has been changed
    setChangedFields(prev => new Set(prev).add(fieldName));
  };

  if (!currentSchema) {
    return <div>Schema laden...</div>;
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div className="relative">
        <InfoHelp variant="schemas" schemaUrl={`http://localhost:8000/schemas/${itemType.charAt(0).toUpperCase() + itemType.slice(1).toLowerCase()}`} />
        <h2 className="text-lg font-semibold mb-4">
          {DEFAULT_LABELS[itemType.toLowerCase()] || itemType} bewerken
        </h2>
      </div>

      {currentSchema.properties &&
        typeof currentSchema.properties === "object" &&
        currentSchema.properties !== null
          ? Object.keys(currentSchema.properties as Record<string, unknown>)
              .filter((fieldName) => {
                // Only show fields that exist in initialData or are in the schema
                return (
                  Object.prototype.hasOwnProperty.call(initialData, fieldName) ||
                  Object.prototype.hasOwnProperty.call(currentSchema.properties, fieldName)
                );
              })
              .sort((a, b) => {
                // Always show 'title' first
                if (a === 'title') return -1;
                if (b === 'title') return 1;
                // Then sort alphabetically
                return a.localeCompare(b);
              })
              .map((fieldName) => {
                return (
                  <div key={fieldName}>
                    <SchemaField
                      fieldName={fieldName}
                      fieldSchema={currentSchema}
                      currentSchema={currentSchema}
                      value={formData[fieldName] || ""}
                      onChange={handleFieldChange}
                      selectedType={itemType}
                      idPrefix="edit-field"
                    />
                  </div>
                );
              })
          : null}

      <div className="flex justify-between pt-4">
        <Button
          type="button"
          variant="secondary"
          onClick={handleDelete}
          disabled={isDeleting}
          className="text-red-600 hover:text-red-700"
        >
          {isDeleting ? "Verwijderen..." : "Verwijderen"}
        </Button>

        <div className="flex gap-2">
          <Button
            type="button"
            variant="secondary"
            onClick={onCancel}
            disabled={isSubmitting}
          >
            Annuleren
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={isSubmitting}
          >
            {isSubmitting ? "Opslaan..." : "Opslaan"}
          </Button>
        </div>
      </div>
    </form>
  );
};

export default SchemaEditFormContent;
