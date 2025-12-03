import type {
  CloudEvent,
  PlanningMoment,
  ExtendedPlanning,
  Planning,
} from "../types";

/**
 * Extract planning items from the items store
 */
export const getPlanningForIssue = (
  events: CloudEvent[],
  issueId: string,
  items: Record<string, Record<string, unknown>>
): Map<string, ExtendedPlanning> => {
  const planningMap = new Map<string, ExtendedPlanning>();

  // Get planning IDs from events for this specific issue
  const planningIds = new Set<string>();
  for (const event of events) {
    if (
      event.subject === issueId &&
      event.type === "json.commit" &&
      event.data &&
      typeof event.data === "object" &&
      event.data !== null
    ) {
      const data = event.data as Record<string, unknown>;
      const planningId = String(data.resource_id || data.item_id);
      const isDeletion = data.deleted === true;
      const isPlanning = (data.schema as string)?.endsWith("/Planning");

      if (planningId && isPlanning && !isDeletion) {
        planningIds.add(planningId);
      }
    }
  }

  // Get planning items from items store using the found IDs
  for (const planningId of planningIds) {
    const itemData = items[planningId];
    if (itemData && itemData.moments && Array.isArray(itemData.moments)) {
      const planning = itemData as unknown as Planning;

      // Find the most recent event for this planning to get actor and timestamp
      const recentEvent = events
        .filter((event) => {
          const data = event.data as Record<string, unknown>;
          return (
            String(data.resource_id || data.item_id) === planningId &&
            event.type === "json.commit" &&
            (data.schema as string)?.endsWith("/Planning")
          );
        })
        .sort(
          (a, b) =>
            new Date(b.time || 0).getTime() - new Date(a.time || 0).getTime()
        )[0];

      const extendedPlanning: ExtendedPlanning = {
        ...planning,
        moments: planning.moments || [],
        actor:
          ((recentEvent?.data as Record<string, unknown>)?.actor as string) ||
          "onbekend",
        timestamp: recentEvent?.time || new Date().toISOString(),
      };

      planningMap.set(planningId, extendedPlanning);
    }
  }

  return planningMap;
};

/**
 * Get the latest active planning for an issue (one that has current or planned items)
 */
export const getLatestPlanningForIssue = (
  events: CloudEvent[],
  issueId: string,
  items: Record<string, Record<string, unknown>>
): ExtendedPlanning | null => {
  const planningItems = getPlanningForIssue(events, issueId, items);
  const planningArray = Array.from(planningItems.values());

  // Find planning with current or planned items (not all completed)
  const activePlanning = planningArray
    .filter(
      (planning) =>
        planning.moments &&
        Array.isArray(planning.moments) &&
        planning.moments.some((moment) => moment.status !== "completed")
    )
    .sort(
      (a, b) =>
        new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
    );

  return activePlanning.length > 0 ? activePlanning[0] : null;
};

/**
 * Get progress information for a planning
 */
export const getPlanningProgress = (
  planning: ExtendedPlanning
): {
  completed: number;
  current: number;
  planned: number;
  total: number;
  currentMoment: PlanningMoment | null;
  nextMoment: PlanningMoment | null;
} => {
  const moments =
    planning.moments && Array.isArray(planning.moments) ? planning.moments : [];

  const completed = moments.filter((m) => m.status === "completed").length;
  const current = moments.filter((m) => m.status === "current").length;
  const planned = moments.filter((m) => m.status === "planned").length;
  const total = moments.length;

  const currentMoment = moments.find((m) => m.status === "current") || null;
  const nextMoment = moments.find((m) => m.status === "planned") || null;

  return {
    completed,
    current,
    planned,
    total,
    currentMoment,
    nextMoment,
  };
};

/**
 * Check if a planning has any active (current or planned) moments
 */
export const isPlanningActive = (planning: ExtendedPlanning): boolean => {
  const moments =
    planning.moments && Array.isArray(planning.moments) ? planning.moments : [];
  return moments.some((moment) => moment.status !== "completed");
};

/**
 * Sort planning moments by date
 */
export const sortPlanningMoments = (
  moments: PlanningMoment[]
): PlanningMoment[] => {
  return [...moments]
    .filter((moment) => moment.date != null) // Filter out moments without dates
    .sort(
      (a, b) => new Date(a.date!).getTime() - new Date(b.date!).getTime()
    );
};

/**
 * Get formatted status text for a planning moment
 */
export const getPlanningMomentStatusText = (
  status: "completed" | "current" | "planned"
): string => {
  switch (status) {
    case "completed":
      return "Afgerond";
    case "current":
      return "Huidig";
    case "planned":
      return "Gepland";
    default:
      return "Onbekend";
  }
};

/**
 * Calculate planning completion percentage
 */
export const getPlanningCompletionPercentage = (
  planning: ExtendedPlanning
): number => {
  const moments =
    planning.moments && Array.isArray(planning.moments) ? planning.moments : [];
  if (moments.length === 0) return 0;

  const completedCount = moments.filter((m) => m.status === "completed").length;
  return Math.round((completedCount / moments.length) * 100);
};

/**
 * Check if planning status should be shown for an issue
 * Returns true if there's active planning (not fully completed) with at least one moment
 */
export const shouldShowPlanningStatus = (
  events: CloudEvent[],
  issueId: string,
  items: Record<string, Record<string, unknown>>
): boolean => {
  const latestPlanning = getLatestPlanningForIssue(events, issueId, items);

  if (!latestPlanning) {
    return false;
  }

  const moments =
    latestPlanning.moments && Array.isArray(latestPlanning.moments)
      ? latestPlanning.moments
      : [];
  if (moments.length === 0) {
    return false;
  }

  // Show if there are any current or planned moments (not all completed)
  return moments.some((moment) => moment.status !== "completed");
};
