/**
 * Configuration for the ZaakChat frontend
 */

/**
 * Returns the base URL for JSON schemas.
 * On localhost, this might be http://localhost:8000/schemas (via proxy or direct)
 * In production, it should be https://zaakchat.nl/schemas
 */
export function getSchemaBaseUrl(): string {
  // Use the current origin to make schema URLs portable
  // For standard CloudEvents compliant with NL-GOV, we want full URLs
  return `${window.location.origin}/schemas`;
}

/**
 * Returns the schema URL for a specific type
 */
export function getSchemaUrl(type: string): string {
  // Capitalize first letter to match backend schema naming convention
  const capitalizedType =
    type.charAt(0).toUpperCase() + type.slice(1).toLowerCase();
  return `${getSchemaBaseUrl()}/${capitalizedType}`;
}

/**
 * Returns the schema URL for JSONCommit (the wrapper)
 */
export function getJSONCommitSchemaUrl(): string {
  return `${getSchemaBaseUrl()}/JSONCommit`;
}
