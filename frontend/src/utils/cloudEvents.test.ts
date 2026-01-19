import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  createItemCreatedEvent,
  createItemUpdatedEvent,
  createItemDeletedEvent,
  createIssueCreatedEvent,
  createCommentCreatedEvent,
  createTaskCreatedEvent,
} from "./cloudEvents";
import type { Comment, Issue, Task } from "../types";

// Mock the UUID generator for consistent testing
vi.mock("./uuid", () => ({
  generateUUID: () => "test-uuid-123",
}));

describe("cloudEvents", () => {
  beforeEach(() => {
    // Mock Date.now() for consistent timestamps
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2025-01-01T00:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("createItemCreatedEvent", () => {
    it("creates a valid json.commit CloudEvent with a Task schema", () => {
      const task: Task = {
        id: "task-1",
        cta: "Test Task",
        description: "Test description",
        url: "http://example.com",
        completed: false,
        deadline: null,
      };

      const event = createItemCreatedEvent("task", task, {
        actor: "test-actor",
      });

      expect(event).toEqual({
        specversion: "1.0",
        id: "test-uuid-123",
        source: "frontend-create",
        subject: "task-1",
        type: "json.commit",
        time: "2025-01-01T00:00:00.000Z",
        datacontenttype: "application/json",
        dataschema: "http://localhost:8000/schemas/JSONCommit",
        data: {
          schema: "http://localhost:8000/schemas/Task",
          resource_id: "task-1",
          actor: "test-actor",
          resource_data: task,
        },
      });
    });

    it("accepts custom options", () => {
      const comment: Comment = {
        id: "comment-1",
        content: "Hello",
        quote_comment: null,
        mentions: null,
      };

      const event = createItemCreatedEvent("comment", comment, {
        source: "custom-source",
        subject: "issue-1",
        actor: "admin",
      });

      expect(event.source).toBe("custom-source");
      expect(event.subject).toBe("issue-1");
      expect(event.data?.actor).toBe("admin");
    });
  });

  describe("createItemUpdatedEvent", () => {
    it("creates a valid json.commit CloudEvent with a partial Task schema", () => {
      const patch: Partial<Task> = {
        completed: true,
      };

      const event = createItemUpdatedEvent<Task>("task", "task-1", patch, {
        actor: "test-actor",
      });

      expect(event).toEqual({
        specversion: "1.0",
        id: "test-uuid-123",
        source: "frontend-edit",
        subject: "task-1",
        type: "json.commit",
        time: "2025-01-01T00:00:00.000Z",
        datacontenttype: "application/json",
        dataschema: "http://localhost:8000/schemas/JSONCommit",
        data: {
          schema: "http://localhost:8000/schemas/Task",
          resource_id: "task-1",
          actor: "test-actor",
          patch: {
            completed: true,
          },
        },
      });
    });
  });

  describe("createItemDeletedEvent", () => {
    it("creates a valid json.commit CloudEvent with deleted flag", () => {
      const event = createItemDeletedEvent("task", "task-1", {
        actor: "test-actor",
      });

      expect(event).toEqual({
        specversion: "1.0",
        id: "test-uuid-123",
        source: "frontend-delete",
        subject: "task-1",
        type: "json.commit",
        time: "2025-01-01T00:00:00.000Z",
        datacontenttype: "application/json",
        dataschema: "http://localhost:8000/schemas/JSONCommit",
        data: {
          schema: "http://localhost:8000/schemas/Task",
          resource_id: "task-1",
          actor: "test-actor",
          deleted: true,
        },
      });
    });
  });

  describe("createIssueCreatedEvent", () => {
    it("creates a valid issue creation event with Issue schema", () => {
      const issue: Issue = {
        id: "issue-1",
        title: "New Issue",
        status: "open",
        created_at: "2025-01-01T00:00:00.000Z",
        description: null,
        assignee: null,
        resolution: null,
      };

      const event = createIssueCreatedEvent(issue, { actor: "test-actor" });

      expect(event.type).toBe("json.commit");
      expect(event.data?.schema).toBe("http://localhost:8000/schemas/Issue");
      expect(event.data?.resource_data).toEqual(issue);
      expect(event.subject).toBe("issue-1");
    });
  });

  describe("createCommentCreatedEvent", () => {
    it("creates a valid comment creation event with Comment schema", () => {
      const comment: Comment = {
        id: "comment-test-uuid-123",
        content: "Test comment",
        quote_comment: null,
        mentions: null,
      };

      const event = createCommentCreatedEvent(comment, "zaak-1", {
        actor: "test-actor",
      });

      expect(event.type).toBe("json.commit");
      expect(event.source).toBe("frontend-demo-event");
      expect(event.subject).toBe("zaak-1");
      expect(event.data?.schema).toBe("http://localhost:8000/schemas/Comment");
      expect(event.data?.resource_data).toEqual(comment);
    });

    it("accepts custom options", () => {
      const comment: Comment = {
        id: "comment-123",
        content: "Test with options",
        quote_comment: "parent-comment-123",
        mentions: ["alice@gemeente.nl"],
      };

      const event = createCommentCreatedEvent(comment, "zaak-1", {
        actor: "admin",
      });

      expect(event.data?.actor).toBe("admin");
      expect(event.data?.resource_data).toEqual(comment);
    });
  });

  describe("createTaskCreatedEvent", () => {
    it("creates a valid task creation event with Task schema", () => {
      const task: Task = {
        id: "task-1",
        cta: "Complete onboarding",
        description: "Finish the onboarding process",
        url: "http://example.com/task",
        completed: false,
        deadline: "2025-02-01",
      };

      const event = createTaskCreatedEvent(task, "zaak-1", {
        actor: "test-actor",
      });

      expect(event.type).toBe("json.commit");
      expect(event.data?.schema).toBe("http://localhost:8000/schemas/Task");
      expect(event.data?.resource_data).toEqual(task);
      expect(event.subject).toBe("zaak-1");
    });
  });
});
