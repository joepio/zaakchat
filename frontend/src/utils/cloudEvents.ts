import type {
  CloudEvent,
  ItemType,
  Comment,
  Issue,
  Task,
  Planning,
  Document,
} from "../types";
import { generateUUID } from "./uuid";
import { getJSONCommitSchemaUrl, getSchemaUrl } from "../config";

interface CloudEventOptions {
  source?: string;
  subject?: string;
  actor: string; // Required - must be provided from session
}

/**
 * Generic function to create a JSONCommit CloudEvent for resource creation
 * Takes the actual schema type as a parameter to ensure type safety
 */
export function createItemCreatedEvent<T extends { id: string }>(
  itemType: ItemType,
  itemData: T,
  options: CloudEventOptions,
): CloudEvent {
  const resourceId = itemData.id;

  return {
    specversion: "1.0",
    id: generateUUID(),
    source: options.source || "frontend-create",
    subject: options.subject || resourceId,
    type: "json.commit",
    time: new Date().toISOString(),
    datacontenttype: "application/json",
    dataschema: getJSONCommitSchemaUrl(),
    data: {
      schema: getSchemaUrl(itemType),
      resource_id: resourceId,
      actor: options.actor,
      resource_data: itemData as any,
    },
  };
}

/**
 * Generic function to create a JSONCommit CloudEvent for resource updates
 */
export function createItemUpdatedEvent<T = Record<string, unknown>>(
  itemType: ItemType,
  resourceId: string,
  patch: Partial<T>,
  options: CloudEventOptions,
): CloudEvent {
  return {
    specversion: "1.0",
    id: generateUUID(),
    source: options.source || "frontend-edit",
    subject: options.subject || resourceId,
    type: "json.commit",
    time: new Date().toISOString(),
    datacontenttype: "application/json",
    dataschema: getJSONCommitSchemaUrl(),
    data: {
      schema: getSchemaUrl(itemType),
      resource_id: resourceId,
      actor: options.actor,
      patch: patch as any,
    },
  };
}

/**
 * Generic function to create a JSONCommit CloudEvent for resource deletion
 * Deletion is represented with the deleted field set to true
 */
export function createItemDeletedEvent(
  itemType: ItemType,
  resourceId: string,
  options: CloudEventOptions,
): CloudEvent {
  return {
    specversion: "1.0",
    id: generateUUID(),
    source: options.source || "frontend-delete",
    subject: options.subject || resourceId,
    type: "json.commit",
    time: new Date().toISOString(),
    datacontenttype: "application/json",
    dataschema: getJSONCommitSchemaUrl(),
    data: {
      schema: getSchemaUrl(itemType),
      resource_id: resourceId,
      actor: options.actor,
      deleted: true,
    },
  };
}

// Convenience functions for specific item types

/**
 * Create a CloudEvent for creating an Issue (uses Issue schema)
 */
export function createIssueCreatedEvent(
  issue: Issue,
  options: CloudEventOptions,
): CloudEvent {
  return createItemCreatedEvent("issue", issue, {
    subject: issue.id,
    ...options,
  });
}

/**
 * Create a CloudEvent for creating a Comment (uses Comment schema)
 */
export function createCommentCreatedEvent(
  comment: Comment,
  zaakId: string,
  options: CloudEventOptions,
): CloudEvent {
  return createItemCreatedEvent("comment", comment, {
    source: "frontend-demo-event",
    subject: zaakId,
    ...options,
  });
}

/**
 * Create a CloudEvent for creating a Task (uses Task schema)
 */
export function createTaskCreatedEvent(
  task: Task,
  zaakId: string,
  options: CloudEventOptions,
): CloudEvent {
  return createItemCreatedEvent("task", task, {
    subject: zaakId,
    ...options,
  });
}

/**
 * Create a CloudEvent for creating a Planning (uses Planning schema)
 */
export function createPlanningCreatedEvent(
  planning: Planning,
  zaakId: string,
  options: CloudEventOptions,
): CloudEvent {
  return createItemCreatedEvent("planning", planning, {
    subject: zaakId,
    ...options,
  });
}

/**
 * Create a CloudEvent for creating a Document (uses Document schema)
 */
export function createDocumentCreatedEvent(
  document: Document,
  zaakId: string,
  options: CloudEventOptions,
): CloudEvent {
  return createItemCreatedEvent("document", document, {
    subject: zaakId,
    ...options,
  });
}
