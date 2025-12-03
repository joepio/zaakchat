import type React from "react";
import type { Issue } from "../types";
import Card from "./Card";
import { Button } from "./ActionButton";

interface IssueHeaderProps {
  issue: Issue;
  onEdit?: () => void;
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
}) => {
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

      <p className="text-sm leading-relaxed text-text-primary mb-3">
        {String(issue.description) || "Geen beschrijving beschikbaar."}
      </p>

      <div className="text-xs text-text-tertiary" data-testid="issue-assignee">
        <strong className="text-text-primary">Toegewezen aan:</strong>{" "}
        {issue.assignee || "Niet toegewezen"}
      </div>

      <div className="text-xs text-text-tertiary mt-2" data-testid="issue-involved">
        <div className="flex items-center gap-2">
          <strong className="text-text-primary">Betrokkenen:</strong>
          {onEdit && (
            <button
              onClick={onEdit}
              className="inline-flex items-center justify-center w-4 h-4 rounded-full hover:bg-bg-tertiary transition-colors"
              title="Betrokkenen bewerken"
              style={{ color: "var(--text-secondary)" }}
            >
              <i className="fa-solid fa-plus text-xs" aria-hidden="true"></i>
            </button>
          )}
        </div>
        {issue.involved && issue.involved.length > 0 ? (
          <div className="flex flex-wrap gap-1 mt-1">
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
          <span className="text-text-tertiary ml-1">Geen betrokkenen</span>
        )}
      </div>
    </Card>
  );
};

export default IssueHeader;
