import React, { useState } from "react";
import type { ExtendedPlanning } from "../types";
import { planningMomentKey } from "../types";
import SchemaEditForm from "./SchemaEditForm";
import { Button } from "./ActionButton";
import { useSSE } from "../contexts/SSEContext";

interface PlanningCardProps {
  planning: ExtendedPlanning;
  zaakId?: string;
}

const PlanningCard: React.FC<PlanningCardProps> = ({ planning, zaakId }) => {
  const { sendEvent } = useSSE();
  const [showEditModal, setShowEditModal] = useState(false);
  const { title, description, moments = [] } = planning;

  const getStatusColor = (status: "completed" | "current" | "planned") => {
    switch (status) {
      case "completed":
        return "var(--status-open)"; // Green
      case "current":
        return "var(--status-progress)"; // Yellow/Orange
      case "planned":
        return "var(--border-primary)"; // Gray
      default:
        return "var(--border-primary)";
    }
  };

  const getStatusIcon = (status: "completed" | "current" | "planned") => {
    switch (status) {
      case "completed":
        return <i className="fa-solid fa-check" aria-hidden="true"></i>;
      case "current":
        return <i className="fa-solid fa-circle" aria-hidden="true"></i>;
      case "planned":
        return <i className="fa-regular fa-circle" aria-hidden="true"></i>;
      default:
        return <i className="fa-regular fa-circle" aria-hidden="true"></i>;
    }
  };

  return (
    <div className="p-0" id={planning.id}>
      {/* Planning header */}
      <div className="mb-3">
        <div className="flex items-center justify-between gap-2 mb-2">
          <div className="flex items-center gap-2">
            <span className="text-lg">
              <i className="fa-regular fa-calendar" aria-hidden="true"></i>
            </span>
            <h4 className="text-base sm:text-lg lg:text-xl xl:text-2xl font-medium text-text-primary m-0">
              {title || "Planning"}
            </h4>
          </div>
          {zaakId && (
            <Button
              variant="icon"
              size="sm"
              onClick={() => setShowEditModal(true)}
              title="Planning bewerken"
            >
              <i className="fa-solid fa-pen" aria-hidden="true"></i>
            </Button>
          )}
        </div>

        {description && (
          <p className="text-sm sm:text-base lg:text-lg xl:text-xl text-text-secondary m-0 mb-3">
            {description}
          </p>
        )}
      </div>

      {/* Mini timeline */}
      <div className="relative">
        {/* Timeline line */}
        <div className="absolute left-3 top-6 bottom-0 w-0.5 bg-border-primary"></div>

        {/* Timeline moments */}
        <div className="space-y-3">
          {moments.map((moment, idx) => (
            <div
              key={planningMomentKey(moment, idx)}
              className="flex items-start gap-3 relative"
            >
              {/* Status indicator */}
              <div
                className="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold border-2 bg-bg-primary relative z-10"
                style={{
                  color:
                    moment.status === "completed"
                      ? "white"
                      : getStatusColor(moment.status),
                  backgroundColor:
                    moment.status === "completed"
                      ? getStatusColor(moment.status)
                      : "var(--bg-primary)",
                  borderColor: getStatusColor(moment.status),
                }}
              >
                {getStatusIcon(moment.status)}
              </div>

              {/* Moment content */}
              <div className="flex-1 min-w-0 pb-1">
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <div
                      className={`text-sm sm:text-base lg:text-lg xl:text-xl font-medium leading-tight ${
                        moment.status === "completed"
                          ? "text-text-secondary line-through"
                          : moment.status === "current"
                            ? "text-text-primary font-semibold"
                            : "text-text-secondary"
                      }`}
                    >
                      {moment.title}
                    </div>
                  </div>
                  <div className="text-xs sm:text-sm lg:text-sm xl:text-base text-text-tertiary whitespace-nowrap">
                    {new Date(moment.date).toLocaleDateString("nl-NL", {
                      day: "numeric",
                      month: "short",
                    })}
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      {moments.length === 0 && (
        <div className="text-text-secondary text-sm sm:text-base lg:text-lg xl:text-xl italic">
          Geen planning momenten beschikbaar
        </div>
      )}

      {zaakId && (
        <SchemaEditForm
          isOpen={showEditModal}
          onClose={() => setShowEditModal(false)}
          itemType="planning"
          itemId={planning.id}
          initialData={{
            title: title,
            description: description,
            moments: moments,
          }}
          onSubmit={sendEvent}
          zaakId={zaakId}
        />
      )}
    </div>
  );
};

export default PlanningCard;
