import type { TimelineItemType } from "../../types";
import type { EventPluginComponent } from "./types";

// Import all plugins
import CommentPlugin from "./CommentPlugin";
import StatusChangePlugin from "./StatusChangePlugin";

import IssueUpdatedPlugin from "./IssueUpdatedPlugin";
import IssueCreatedPlugin from "./IssueCreatedPlugin";
import IssueDeletedPlugin from "./IssueDeletedPlugin";
import DeploymentPlugin from "./DeploymentPlugin";
import SystemEventPlugin from "./SystemEventPlugin";
import DocumentPlugin from "./DocumentPlugin";

import GenericResourcePlugin from "./GenericResourcePlugin";

// Plugin registry - map event types to components
export const eventPlugins: Record<TimelineItemType, EventPluginComponent> = {
  comment: CommentPlugin,
  document: DocumentPlugin,
  status_change: StatusChangePlugin,
  field_update: SystemEventPlugin,
  system_update: SystemEventPlugin,
  llm_analysis: SystemEventPlugin,
  deployment: DeploymentPlugin,
  system_event: SystemEventPlugin,
  issue_created: IssueCreatedPlugin,
  issue_updated: IssueUpdatedPlugin,
  issue_deleted: IssueDeletedPlugin,
  task: SystemEventPlugin,
  planning: SystemEventPlugin,
};

// Utility function to get plugin for event type
export const getEventPlugin = (
  eventType: TimelineItemType,
): EventPluginComponent => {
  return eventPlugins[eventType] || GenericResourcePlugin;
};

// Export individual plugins for direct use if needed
export {
  CommentPlugin,
  StatusChangePlugin,
  IssueUpdatedPlugin,
  IssueCreatedPlugin,
  IssueDeletedPlugin,
  DeploymentPlugin,
  SystemEventPlugin,
};
