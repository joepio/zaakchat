import type React from "react";
import { useEffect, useRef, useState } from "react";
import { BrowserRouter as Router, Routes, Route, Link } from "react-router-dom";
import SearchPlayground from "./components/SearchPlayground";
import ApiDocumentationView from "./components/documentation/ApiDocumentationView";
import PageHeader from "./components/PageHeader";
import InstallPrompt from "./components/InstallPrompt";
import ScrollToTop from "./components/ScrollToTop";
import { SSEProvider, useSSE } from "./contexts/SSEContext";
import { SearchProvider } from "./contexts/SearchContext";
import { ActorProvider } from "./contexts/ActorContext";
import CreateIssueForm from "./components/CreateIssueForm";
import IssueTimeline from "./components/IssueTimeline";
import Modal from "./components/Modal";
import ActionButton, { Button } from "./components/ActionButton";
import { formatRelativeTime } from "./utils/time";
import { getLatestTaskForIssue } from "./utils/taskUtils";
import DeadlineBadge from "./components/DeadlineBadge";
import PlanningStatusBadge from "./components/PlanningStatusBadge";
import { shouldShowPlanningStatus } from "./utils/planningUtils";
import Card from "./components/Card";
import { ConnectionErrorBanner } from "./components/ConnectionErrorBanner";

import type { CloudEvent, Issue } from "./types";
import "./App.css";

const ZakenDashboard: React.FC = () => {
  const { issues, events, items, sendEvent, completeTask } = useSSE();
  const [animatingIssues, setAnimatingIssues] = useState<Set<string>>(
    new Set(),
  );
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const prevIssuesRef = useRef<Record<string, Issue>>({});

  // Track which issues are new for animation
  useEffect(() => {
    const currentIssueIds = new Set(Object.keys(issues));
    const prevIssueIds = new Set(Object.keys(prevIssuesRef.current));

    const newIssues = new Set(
      [...currentIssueIds].filter((id) => !prevIssueIds.has(id)),
    );

    if (newIssues.size > 0) {
      setAnimatingIssues((prev) => new Set([...prev, ...newIssues]));

      // Remove animation class after animation completes
      setTimeout(() => {
        setAnimatingIssues((prev) => {
          const updated = new Set(prev);
          newIssues.forEach((id) => {
            updated.delete(id);
          });
          return updated;
        });
      }, 500);
    }

    prevIssuesRef.current = issues;
  }, [issues]);

  const handleCreateIssue = async (event: CloudEvent) => {
    await sendEvent(event);
    setIsCreateModalOpen(false);
  };

  const issueEntries = Object.entries(issues).sort((a, b) => {
    const [, issueA] = a;
    const [, issueB] = b;

    // Sort by lastActivity (newest first), fallback to created_at
    const timeA = issueA.lastActivity || "1970-01-01";
    const timeB = issueB.lastActivity || "1970-01-01";

    return new Date(timeB).getTime() - new Date(timeA).getTime();
  });

  return (
    <div
      className="min-h-screen"
      style={{
        backgroundColor: "var(--bg-primary)",
        color: "var(--text-primary)",
      }}
    >
      <PageHeader />

      <div
        className="text-center py-12 lg:py-16 xl:py-20"
        style={{
          backgroundColor: "var(--bg-primary)",
        }}
      >
        <h1
          className="text-3xl lg:text-4xl xl:text-5xl font-bold mb-4 lg:mb-6 xl:mb-8"
          style={{ color: "var(--logo-primary)" }}
          data-testid="main-heading"
        >
          ZaakChat
        </h1>
        <p
          className="text-base lg:text-lg xl:text-xl max-w-2xl lg:max-w-3xl xl:max-w-4xl mx-auto"
          style={{ color: "var(--text-secondary)" }}
        >
          Een simpel zaaksysteem met realtime updates.
        </p>
        <div className="mt-4">
          <Link to="/search-test" className="text-blue-400 hover:text-blue-300 underline">
            Search Playground
          </Link>
        </div>
      </div>

      <main className="max-w-3xl lg:max-w-4xl xl:max-w-5xl mx-auto p-4 md:p-8 lg:p-12 xl:p-16 pt-8 lg:pt-12 xl:pt-16">
        <div className="flex flex-col gap-6 md:gap-4 lg:gap-6 xl:gap-8 mb-8 lg:mb-12 xl:mb-16">
          {issueEntries.length === 0 ? (
            <p
              className="text-center text-base lg:text-lg xl:text-xl py-8 lg:py-12 xl:py-16"
              style={{ color: "var(--text-secondary)" }}
              data-testid="no-issues"
            >
              Geen zaken gevonden.
            </p>
          ) : (
            issueEntries.map(([id, issue]) => {
              const latestTask = getLatestTaskForIssue(events, id, items);
              const showPlanning = shouldShowPlanningStatus(events, id, items);

              return (
                <Card
                  key={id}
                  className={`zaak-item-hover transition-all duration-150 block no-underline ${
                    animatingIssues.has(id) ? "animate-timeline-appear" : ""
                  }`}
                  padding="sm"
                  data-issue-id={id}
                  data-testid="zaak-item"
                >
                  <Link
                    to={`/zaak/${id}`}
                    className="flex flex-col md:flex-row md:justify-between md:items-center gap-4 md:gap-2 no-underline group"
                  >
                    <div
                      className="zaak-link font-semibold text-lg lg:text-xl xl:text-2xl leading-tight flex-1 min-w-0 no-underline"
                      style={{ color: "var(--text-primary)" }}
                    >
                      <h2>{issue.title || "Zaak zonder titel"}</h2>
                    </div>
                    <div
                      className="text-sm lg:text-base xl:text-lg font-normal whitespace-nowrap flex-shrink-0 opacity-80 group-hover:opacity-100 md:self-end"
                      style={{ color: "var(--text-tertiary)" }}
                      data-testid="last-activity"
                    >
                      {formatRelativeTime(
                        issue.lastActivity ||
                          issue.created_at ||
                          new Date().toISOString(),
                      )}
                    </div>
                  </Link>

                  {(showPlanning || (latestTask && !latestTask.completed)) && (
                    <div className="mt-4 md:mt-3 space-y-3">
                      {/* Planning Status */}
                      {showPlanning && (
                        <div
                          onClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                          }}
                        >
                          <PlanningStatusBadge
                            events={events}
                            issueId={id}
                            variant="compact"
                          />
                        </div>
                      )}

                      {/* Active Task */}
                      {latestTask && !latestTask.completed && (
                        <div
                          onClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                          }}
                        >
                          <div className="flex items-center gap-2 text-sm lg:text-base xl:text-lg">
                            <div className="text-sm lg:text-base xl:text-lg">
                              <ActionButton
                                variant="secondary"
                                onClick={() => {
                                  completeTask(latestTask.id, id);
                                }}
                                data-testid="complete-task-button"
                              >
                                {latestTask.cta}
                              </ActionButton>
                            </div>
                            {latestTask.deadline && (
                              <DeadlineBadge
                                deadline={latestTask.deadline}
                                variant="full"
                                showLabel={false}
                              />
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  )}
                </Card>
              );
            })
          )}
        </div>

        <Button
          variant="primary"
          size="md"
          onClick={() => setIsCreateModalOpen(true)}
          data-testid="create-issue-button"
        >
          + Nieuwe Zaak Aanmaken
        </Button>
      </main>

      {/* Create Zaak Modal */}
      <Modal
        isOpen={isCreateModalOpen}
        onClose={() => setIsCreateModalOpen(false)}
        title="Nieuwe Zaak Aanmaken"
        maxWidth="600px"
      >
        <div style={{ backgroundColor: "var(--bg-secondary)" }}>
          <CreateIssueForm onCreateIssue={handleCreateIssue} />
        </div>
      </Modal>
      <InstallPrompt />
    </div>
  );
};

import { AuthProvider, useAuth } from "./contexts/AuthContext";
import Login from "./components/Login";

const AuthenticatedApp: React.FC = () => {
  const { isAuthenticated } = useAuth();

  if (!isAuthenticated) {
    return <Login />;
  }

  return (
    <ActorProvider>
      <SSEProvider>
        <SearchProvider>
          <ConnectionErrorBanner />
          <Router>
            <ScrollToTop />
            <Routes>
              <Route path="/" element={<ZakenDashboard />} />
              <Route path="/zaak/:zaakId" element={<IssueTimeline />} />
              <Route path="/search-test" element={<SearchPlayground />} />
              <Route path="/api-docs" element={<ApiDocumentationView />} />
            </Routes>
          </Router>
        </SearchProvider>
      </SSEProvider>
    </ActorProvider>
  );
};

const App: React.FC = () => {
  return (
    <AuthProvider>
      <AuthenticatedApp />
    </AuthProvider>
  );
};

export default App;
