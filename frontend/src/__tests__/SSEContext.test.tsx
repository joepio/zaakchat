import React from "react";
import { render, screen, waitFor, act } from "@testing-library/react";
import { SSEProvider, useSSE } from "../contexts/SSEContext";
import type { CloudEvent } from "../types";
import { vi, beforeEach, afterEach, describe, it, expect } from "vitest";

// Mock contexts
vi.mock("../contexts/ActorContext", () => ({
  useActor: () => ({ actor: "test-actor" }),
}));

vi.mock("../contexts/AuthContext", () => ({
  useAuth: () => ({ token: "test-token", logout: vi.fn() }),
}));

// Mock EventSource
class MockEventSource {
  url: string;
  readyState: number = EventSource.CONNECTING;
  private listeners: Map<string, ((event: MessageEvent) => void)[]> = new Map();

  constructor(url: string) {
    this.url = url;
    // Don't auto-connect in constructor to avoid timing issues
  }

  addEventListener(type: string, listener: (event: MessageEvent) => void) {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, []);
    }
    this.listeners.get(type)!.push(listener);
  }

  removeEventListener(type: string, listener: (event: MessageEvent) => void) {
    const typeListeners = this.listeners.get(type);
    if (typeListeners) {
      const index = typeListeners.indexOf(listener);
      if (index > -1) {
        typeListeners.splice(index, 1);
      }
    }
  }

  dispatchEvent(event: MessageEvent) {
    const typeListeners = this.listeners.get(event.type);
    if (typeListeners) {
      typeListeners.forEach((listener) => listener(event));
    }
  }

  close() {
    this.readyState = EventSource.CLOSED;
  }

  // Test helper methods
  connect() {
    this.readyState = EventSource.OPEN;
    const openEvent = new Event("open");
    this.dispatchEvent(openEvent as any);
  }

  simulateSnapshot(events: CloudEvent[]) {
    const event = new MessageEvent("snapshot", {
      data: JSON.stringify(events),
    });
    this.dispatchEvent(event);
  }

  simulateDelta(event: CloudEvent) {
    const messageEvent = new MessageEvent("delta", {
      data: JSON.stringify(event),
    });
    this.dispatchEvent(messageEvent);
  }
}

// Mock fetch
const mockFetch = vi.fn();

// Mock window.location.reload
const mockReload = vi.fn();

// Test component
const TestComponent: React.FC = () => {
  const { events, issues, connectionStatus } = useSSE();

  return (
    <div>
      <div data-testid="connection-status">{connectionStatus}</div>
      <div data-testid="events-count">{events.length}</div>
      <div data-testid="issues-count">{Object.keys(issues).length}</div>
      {Object.entries(issues).map(([id, issue]) => (
        <div key={id} data-testid={`issue-${id}`}>
          <span data-testid={`issue-${id}-title`}>{issue.title}</span>
          <span data-testid={`issue-${id}-lastActivity`}>
            {issue.lastActivity}
          </span>
        </div>
      ))}
    </div>
  );
};

// Helper to create test events
const createIssueEvent = (
  id: string,
  title: string,
  time: string = new Date().toISOString(),
): CloudEvent => ({
  specversion: "1.0",
  id: `event-${Date.now()}-${Math.random()}`,
  source: "test",
  subject: id,
  type: "json.commit",
  time,
  datacontenttype: "application/json",
  dataschema: "http://localhost:8000/schemas/JSONCommit",
  data: {
    schema: "http://localhost:8000/schemas/Issue",
    resource_id: id,
    resource_data: {
      id,
      title,
      status: "open",
      created_at: time,
    },
  },
});

const createCommentEvent = (
  issueId: string,
  commentId: string,
  content: string,
  time: string = new Date().toISOString(),
): CloudEvent => ({
  specversion: "1.0",
  id: `comment-event-${Date.now()}-${Math.random()}`,
  source: "test",
  subject: issueId,
  type: "json.commit",
  time,
  datacontenttype: "application/json",
  dataschema: "http://localhost:8000/schemas/JSONCommit",
  data: {
    schema: "http://localhost:8000/schemas/Comment",
    resource_id: commentId,
    resource_data: {
      id: commentId,
      content,
      author: "test@zaakchat.nl",
    },
  },
});

// Global test setup
let mockEventSource: MockEventSource;

describe("SSEContext", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Mock fetch
    mockFetch.mockResolvedValue({ ok: true });
    (globalThis as any).fetch = mockFetch as any;

    // Mock window.location.reload
    Object.defineProperty(window, "location", {
      value: { reload: mockReload },
      writable: true,
    });

    // Mock EventSource
    (globalThis as any).EventSource = vi
      .fn()
      .mockImplementation((url: string) => {
        mockEventSource = new MockEventSource(url);
        return mockEventSource;
      }) as any;
  });

  afterEach(() => {
    if (mockEventSource) {
      mockEventSource.close();
    }
  });

  describe("Basic Functionality", () => {
    it("should initialize with connecting status", () => {
      render(
        <SSEProvider>
          <TestComponent />
        </SSEProvider>,
      );

      expect(screen.getByTestId("connection-status")).toHaveTextContent(
        "connecting",
      );
    });

    it("should connect and process snapshot", async () => {
      const testIssue = createIssueEvent("issue-1", "Test Issue");

      render(
        <SSEProvider>
          <TestComponent />
        </SSEProvider>,
      );

      // Simulate connection
      act(() => {
        mockEventSource.connect();
      });

      await waitFor(() => {
        expect(screen.getByTestId("connection-status")).toHaveTextContent(
          "connected",
        );
      });

      // Send snapshot
      act(() => {
        mockEventSource.simulateSnapshot([testIssue]);
      });

      await waitFor(() => {
        expect(screen.getByTestId("issues-count")).toHaveTextContent("1");
      });

      expect(screen.getByTestId("issue-issue-1-title")).toHaveTextContent(
        "Test Issue",
      );
    });

    it("should process delta events", async () => {
      render(
        <SSEProvider>
          <TestComponent />
        </SSEProvider>,
      );

      // Connect and start with empty snapshot
      act(() => {
        mockEventSource.connect();
        mockEventSource.simulateSnapshot([]);
      });

      await waitFor(() => {
        expect(screen.getByTestId("connection-status")).toHaveTextContent(
          "connected",
        );
      });

      // Add issue via delta
      const newIssue = createIssueEvent("issue-2", "New Issue");
      act(() => {
        mockEventSource.simulateDelta(newIssue);
      });

      await waitFor(() => {
        expect(screen.getByTestId("issues-count")).toHaveTextContent("1");
      });

      expect(screen.getByTestId("issue-issue-2-title")).toHaveTextContent(
        "New Issue",
      );
    });

    it("should update lastActivity when comments are added", async () => {
      const initialTime = "2024-01-01T10:00:00.000Z";
      const commentTime = "2024-01-01T11:00:00.000Z";

      const initialIssue = createIssueEvent(
        "issue-1",
        "Test Issue",
        initialTime,
      );

      render(
        <SSEProvider>
          <TestComponent />
        </SSEProvider>,
      );

      // Connect and load initial issue
      act(() => {
        mockEventSource.connect();
        mockEventSource.simulateSnapshot([initialIssue]);
      });

      await waitFor(() => {
        expect(screen.getByTestId("issues-count")).toHaveTextContent("1");
      });

      // Verify initial lastActivity
      expect(
        screen.getByTestId("issue-issue-1-lastActivity"),
      ).toHaveTextContent(initialTime);

      // Add comment
      const comment = createCommentEvent(
        "issue-1",
        "comment-1",
        "Test comment",
        commentTime,
      );

      act(() => {
        mockEventSource.simulateDelta(comment);
      });

      // Should update lastActivity
      await waitFor(() => {
        expect(
          screen.getByTestId("issue-issue-1-lastActivity"),
        ).toHaveTextContent(commentTime);
      });
    });

    it("should correctly calculate lastActivity from multiple events in snapshot", async () => {
      const createTime = "2024-01-01T10:00:00.000Z";
      const commentTime = "2024-01-01T11:00:00.000Z";
      const laterCommentTime = "2024-01-01T12:00:00.000Z";

      const events: CloudEvent[] = [
        createIssueEvent("issue-1", "Test Issue", createTime),
        createCommentEvent(
          "issue-1",
          "comment-1",
          "First comment",
          commentTime,
        ),
        createCommentEvent(
          "issue-1",
          "comment-2",
          "Later comment",
          laterCommentTime,
        ),
      ];

      render(
        <SSEProvider>
          <TestComponent />
        </SSEProvider>,
      );

      act(() => {
        mockEventSource.connect();
        mockEventSource.simulateSnapshot(events);
      });

      await waitFor(() => {
        expect(screen.getByTestId("issues-count")).toHaveTextContent("1");
      });

      // Should show the latest activity time (from the later comment)
      expect(
        screen.getByTestId("issue-issue-1-lastActivity"),
      ).toHaveTextContent(laterCommentTime);
    });
  });

  describe("Event Sending", () => {
    it("should send events to server", async () => {
      const TestSender: React.FC = () => {
        const { sendEvent } = useSSE();

        const handleSend = () => {
          sendEvent(createIssueEvent("new-issue", "Sent Issue"));
        };

        return (
          <button onClick={handleSend} data-testid="send-button">
            Send
          </button>
        );
      };

      render(
        <SSEProvider>
          <TestSender />
        </SSEProvider>,
      );

      const sendButton = screen.getByTestId("send-button");

      act(() => {
        sendButton.click();
      });

      await waitFor(() => {
        expect(mockFetch).toHaveBeenCalledWith(
          "/events",
          expect.objectContaining({
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              Authorization: "Bearer test-token",
            },
          }),
        );
      });
    });
  });

  describe("Error Handling", () => {
    it("should handle malformed event data gracefully", async () => {
      const consoleSpy = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});

      render(
        <SSEProvider>
          <TestComponent />
        </SSEProvider>,
      );

      act(() => {
        mockEventSource.connect();
      });

      await waitFor(() => {
        expect(screen.getByTestId("connection-status")).toHaveTextContent(
          "connected",
        );
      });

      // Send malformed data
      act(() => {
        const malformedEvent = new MessageEvent("delta", {
          data: "invalid json",
        });
        mockEventSource.dispatchEvent(malformedEvent);
      });

      // Should not crash, error should be logged
      expect(consoleSpy).toHaveBeenCalled();
      expect(screen.getByTestId("issues-count")).toHaveTextContent("0");

      consoleSpy.mockRestore();
    });

    it("should handle system reset by calling window.location.reload", async () => {
      render(
        <SSEProvider>
          <TestComponent />
        </SSEProvider>,
      );

      act(() => {
        mockEventSource.connect();
      });

      const resetEvent: CloudEvent = {
        specversion: "1.0",
        id: "reset-event",
        source: "test",
        subject: null,
        type: "system.reset",
        time: new Date().toISOString(),
        datacontenttype: "application/json",
        data: {},
      };

      act(() => {
        mockEventSource.simulateDelta(resetEvent);
      });

      expect(mockReload).toHaveBeenCalled();
    });
  });
});
