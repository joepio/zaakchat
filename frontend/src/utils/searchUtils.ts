
export interface Suggestion {
  type: "key" | "value";
  text: string;
  description?: string;
}

/**
 * Transforms user-friendly query to Tantivy-compatible query.
 *
 * Transformations:
 * - `is:issue` -> `type:issue`
 * - `assignee:me` -> `json_payload.assignee:"alice@example.com"`
 * - `status:open` -> `json_payload.status:open`
 * - `key:value` -> `json_payload.key:value`
 * - `*` remains `*`
 *
 * @param input The raw user input
 * @param currentUserEmail The current user's email for 'assignee:me' substitution
 */
export function transformQuery(input: string, currentUserEmail?: string): string {
  if (!input.trim()) return "*";

  // Split by space but respect quotes roughly (simple regex for now)
  // This is a simplified split, might not handle complex quoting perfectly but sufficient for 80/20
  const parts = input.match(/(?:[^\s"]+|"[^"]*")+/g) || [];

  const transformedParts = parts.map(part => {
    // Handle "is:type"
    if (part.startsWith("is:")) {
      const type = part.substring(3);
      return `type:${type}`;
    }

    // Handle generic key:value
    if (part.includes(":")) {
      const [key, ...valParts] = part.split(":");
      let value = valParts.join(":");

      // Special substitution for currentUser
      if (key === "assignee" && value === "me" && currentUserEmail) {
        value = `"${currentUserEmail}"`;
      }

      // Don't prefix reserved fields or if it's already structured
      if (["type", "id", "json_payload"].includes(key)) {
        return `${key}:${value}`;
      }

      return `json_payload.${key}:${value}`;
    }

    // Free text
    return part;
  });

  return transformedParts.join(" ");
}

/**
 * Generates suggestions based on current input and schema.
 */
export function getSuggestions(
  input: string,
  cursorPos: number,
  schema: any
): Suggestion[] {
  // 1. Determine what we are typing (key or value)
  // Simplified: look at the word under cursor
  const textBeforeCursor = input.substring(0, cursorPos);
  const words = textBeforeCursor.split(/\s+/);
  const currentWord = words[words.length - 1] || "";

  // If typing a value (contains :)
  if (currentWord.includes(":")) {
    const [key, valuePrefix] = currentWord.split(":");

    // Hardcoded suggestions for "is:"
    if (key === "is") {
      const types = ["issue", "task", "comment", "document", "planning"];
      return types
        .filter(t => t.startsWith(valuePrefix))
        .map(t => ({ type: "value", text: t, description: `Filter by ${t} type` }));
    }

    // Schema based value suggestions
    if (schema?.properties?.[key]?.enum) {
      return schema.properties[key].enum
        .filter((val: string) => val.startsWith(valuePrefix))
        .map((val: string) => ({ type: "value", text: val, description: schema.properties[key].description }));
    }

    // Suggest "me" for assignee
    if (key === "assignee" && "me".startsWith(valuePrefix)) {
        return [{ type: "value", text: "me", description: "Current user" }];
    }

    return [];
  }

  // Else suggestions keys
  const keys: Suggestion[] = [];

  // Always suggest "is:"
  if ("is".startsWith(currentWord)) {
    keys.push({ type: "key", text: "is", description: "Filter by document type" });
  }

  // Schema properties
  if (schema?.properties) {
    Object.entries(schema.properties).forEach(([propKey, propVal]: [string, any]) => {
      // filtering
      if (propKey.startsWith(currentWord)) {
        keys.push({
          type: "key",
          text: propKey,
          description: propVal.description || propVal.title
        });
      }
    });
  }

  return keys;
}
