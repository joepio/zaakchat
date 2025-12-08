import React from "react";
import type { Issue } from "../types";
import Card from "./Card";
import { Button } from "./ActionButton";

import MarkdownRenderer from "./MarkdownRenderer";

interface IssueHeaderProps {
  issue: Issue;
  onEdit?: () => void;
  onAddInvolved?: (email: string) => void;
}

const getStatusColor = (status: string): string => {
  switch (status) {
    case "open":
      return "#10B981"; // Green
    case "in_progress":
      return "#F59E0B"; // Yellow
    case "closed":
      return "#6B7280"; // Gray
    default:
      return "#6B7280";
  }
};

const IssueHeader: React.FC<IssueHeaderProps> = ({
  issue,
  onEdit,
  onAddInvolved,
}) => {
  const [isAddingInvolved, setIsAddingInvolved] = React.useState(false);
  const [newEmail, setNewEmail] = React.useState("");

  const handleAddSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (newEmail && onAddInvolved) {
      onAddInvolved(newEmail);
      setNewEmail("");
      setIsAddingInvolved(false);
    }
  };

  return (
    <Card padding="md">
      <div className="flex items-start justify-between gap-4 mb-3">
        <div className="flex-1 min-w-0">
          <h1 className="text-lg md:text-xl xl:text-2xl font-semibold text-text-primary mb-2 lg:mb-3 xl:mb-4 leading-tight">
            {String(issue.title) || "Zaak zonder titel"}
          </h1>
          <div className="flex items-center gap-2">
            <span
              className="inline-flex items-center px-2 py-1 lg:px-3 lg:py-2 xl:px-3 xl:py-2 text-xs lg:text-xs xl:text-sm font-semibold text-text-inverse capitalize"
              style={{ backgroundColor: getStatusColor(issue.status) }}
              data-testid="issue-status"
            >
              {issue.status === "in_progress"
                ? "In Behandeling"
                : issue.status === "open"
                  ? "Open"
                  : issue.status === "closed"
                    ? "Gesloten"
                    : issue.status}
            </span>
          </div>
        </div>

        <div className="flex items-center gap-1">
          {onEdit && (
            <Button variant="icon" size="sm" onClick={onEdit} title="Bewerken">
              <i className="fa-solid fa-pen" aria-hidden="true"></i>
            </Button>
          )}
        </div>
      </div>

      <div className="text-sm leading-relaxed text-text-primary mb-3">
        {issue.description ? (
          <MarkdownRenderer content={issue.description} />
        ) : (
          "Geen beschrijving beschikbaar."
        )}
      </div>

      <div className="text-xs text-text-tertiary" data-testid="issue-assignee">
        <strong className="text-text-primary">Toegewezen aan:</strong>{" "}
        {issue.assignee || "Niet toegewezen"}
      </div>

      <div className="text-xs text-text-tertiary mt-2" data-testid="issue-involved">
        <div className="flex items-center gap-2 mb-1">
          <strong className="text-text-primary">Betrokkenen:</strong>
          {onAddInvolved && !isAddingInvolved && (
            <button
              onClick={() => setIsAddingInvolved(true)}
              className="inline-flex items-center justify-center w-5 h-5 rounded-full hover:bg-bg-tertiary transition-colors"
              title="Persoon toevoegen"
              style={{ color: "var(--text-secondary)" }}
            >
              <i className="fa-solid fa-plus text-xs" aria-hidden="true"></i>
            </button>
          )}
        </div>

        {isAddingInvolved && (
          <form onSubmit={handleAddSubmit} className="flex items-center gap-2 mb-2">
            <input
              type="email"
              value={newEmail}
              onChange={(e) => setNewEmail(e.target.value)}
              placeholder="Email adres..."
              className="px-2 py-1 text-sm border rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
              style={{
                backgroundColor: "var(--bg-primary)",
                borderColor: "var(--border-primary)",
                color: "var(--text-primary)"
              }}
              autoFocus
            />
            <button
              type="submit"
              className="px-2 py-1 text-xs font-medium text-white bg-blue-600 rounded hover:bg-blue-700"
            >
              Toevoegen
            </button>
            <button
              type="button"
              onClick={() => setIsAddingInvolved(false)}
              className="px-2 py-1 text-xs font-medium text-gray-600 bg-gray-200 rounded hover:bg-gray-300"
            >
              Annuleren
            </button>
          </form>
        )}

        {issue.involved && issue.involved.length > 0 ? (
          <div className="flex flex-wrap gap-1">
            {issue.involved.map((email, index) => (
              <span
                key={index}
                className="inline-flex items-center px-2 py-0.5 rounded text-xs"
                style={{
                  backgroundColor: "var(--bg-tertiary)",
                  color: "var(--text-primary)",
                }}
              >
                {email}
              </span>
            ))}
          </div>
        ) : (
          <span className="text-text-tertiary">Geen betrokkenen</span>
        )}
      </div>
    </Card>
  );
};

export default IssueHeader;
