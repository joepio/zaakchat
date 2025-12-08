import React, { useState } from "react";
import type { Task as TaskType } from "../types";
import DeadlineBadge from "./DeadlineBadge";
import { Button } from "./ActionButton";
import MarkdownRenderer from "./MarkdownRenderer";

interface TaskProps {
  task: TaskType;
  onComplete: (taskId: string) => void;
  variant?: "full" | "compact";
  showActor?: boolean;
}

const Task: React.FC<TaskProps> = ({
  task,
  onComplete,
  variant = "full",
  showActor = true,
}) => {
  const [isCompleting, setIsCompleting] = useState(false);

  const handleComplete = async () => {
    setIsCompleting(true);
    try {
      await onComplete(task.id);
    } finally {
      setIsCompleting(false);
    }
  };

  const isCompact = variant === "compact";

  if (task.completed) {
    return (
      <div
        className={`border border-l-4 rounded-lg transition-all duration-200 ${
          isCompact ? "mb-2" : "mb-4"
        } opacity-80 border-border-primary bg-bg-success`}
        style={{ borderLeftColor: "var(--text-success)" }}
      >
        <div className={`${isCompact ? "p-3" : "p-4"}`}>
          <div className="flex justify-between items-center mb-3">
            <span className="font-semibold text-text-success flex items-center gap-2">
              ✓ Voltooid
            </span>
            {showActor && (
              <span className="text-text-tertiary text-xs">
                voltooid
              </span>
            )}
          </div>
          <div
            className={`text-text-primary leading-relaxed ${
              isCompact ? "text-sm" : "text-base"
            }`}
          >
            <MarkdownRenderer content={task.description} />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`border border-l-4 rounded-lg bg-bg-primary hover:border-border-hover hover:shadow-md transition-all duration-200 ${
        isCompact ? "mb-2" : "mb-4"
      } border-border-primary`}
      style={{ borderLeftColor: "var(--link-primary)" }}
    >
      <div className={`${isCompact ? "p-3" : "p-4"}`}>
        {task.deadline && (
          <div className="mb-2">
            <DeadlineBadge
              deadline={task.deadline}
              variant="compact"
              showLabel={false}
            />
          </div>
        )}

        <div className={`${isCompact ? "mb-3" : "mb-4"}`}>
          <div
            className={`text-text-primary leading-relaxed m-0 ${
              isCompact ? "text-sm" : "text-base"
            }`}
          >
            <MarkdownRenderer content={task.description} />
          </div>
        </div>

        <div className="flex gap-3 items-center md:flex-col md:items-stretch">
          <Button
            onClick={handleComplete}
            variant="secondary"
            size={isCompact ? "xs" : "sm"}
            disabled={isCompleting}
            loading={isCompleting}
            title="Markeer als voltooid"
          >
            {task.cta}
          </Button>
        </div>
      </div>
    </div>
  );
};

export default Task;
