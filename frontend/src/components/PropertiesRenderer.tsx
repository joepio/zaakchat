import React, { useEffect, useState } from "react";
import { fetchSchema } from "../types/interfaces";

interface PropertiesRendererProps {
  data: Record<string, unknown>;
  schema?: Record<string, unknown>;
  schemaUrl?: string; // e.g. "Issue" or full URL
  ignoredProperties?: string[];
  className?: string;
}

const PropertyItem: React.FC<{
  label: string;
  description?: string;
  children: React.ReactNode;
}> = ({ label, description, children }) => (
  <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-2">
    <div className="flex-shrink-0 min-w-[120px] max-w-[200px] flex items-center">
      <strong
        className={`text-text-primary ${description ? "cursor-help border-b border-dotted border-gray-400" : ""}`}
        title={description}
      >
        {label}:
      </strong>
    </div>
    <div className="flex-grow break-words text-text-primary">
      {children}
    </div>
  </div>
);

const PropertiesRenderer: React.FC<PropertiesRendererProps> = ({
  data,
  schema: initialSchema,
  schemaUrl,
  ignoredProperties = [],
  className = "",
}) => {
  const [schema, setSchema] = useState<Record<string, unknown> | null>(
    initialSchema || null
  );

  useEffect(() => {
    if (initialSchema) {
      setSchema(initialSchema);
      return;
    }

    if (schemaUrl) {
      const loadSchema = async () => {
        try {
          // Extract schema name if it's a URL
          const schemaName = schemaUrl.includes("/")
            ? schemaUrl.split("/").pop()
            : schemaUrl;

          if (schemaName) {
            const fetchedSchema = await fetchSchema(schemaName);
            setSchema(fetchedSchema);
          }
        } catch (error) {
          console.error(`Failed to load schema for ${schemaUrl}:`, error);
        }
      };

      loadSchema();
    }
  }, [schemaUrl, initialSchema]);

  const renderValue = (value: unknown): React.ReactNode => {
    if (value === null || value === undefined) {
      return <span className="text-gray-400 italic">null</span>;
    }

    if (Array.isArray(value)) {
      return (
        <div className="flex flex-wrap gap-1">
          {value.map((item, idx) => (
            <span
              key={idx}
              className="px-1.5 py-0.5 rounded text-xs border"
              style={{
                borderColor: "var(--border-primary)",
                backgroundColor: "var(--bg-tertiary)"
              }}
            >
              {typeof item === "object" ? JSON.stringify(item) : String(item)}
            </span>
          ))}
        </div>
      );
    }

    if (typeof value === "boolean") {
      return value ? "Ja" : "Nee";
    }

    if (typeof value === "object") {
      return (
        <pre className="text-xs m-0 font-mono whitespace-pre-wrap">
          {JSON.stringify(value, null, 2)}
        </pre>
      );
    }

    // Check if string looks like a date
    if (typeof value === "string" && /^\d{4}-\d{2}-\d{2}T/.test(value)) {
       try {
         return new Date(value).toLocaleDateString("nl-NL", {
            day: "numeric",
            month: "long",
            year: "numeric",
            hour: "2-digit",
            minute: "2-digit"
         });
       } catch (e) {
         return value;
       }
    }

    return String(value);
  };

  const keys = Object.keys(data).filter(
    (key) => !ignoredProperties.includes(key)
  );

  if (keys.length === 0) {
    return null;
  }

  return (
    <div className={`space-y-2 text-xs ${className}`}>
      {keys.map((key) => {
        const properties = schema?.properties as Record<string, any>;
        const fieldSchema = properties?.[key];
        const description = fieldSchema?.description;
        const title = fieldSchema?.title || key.charAt(0).toUpperCase() + key.slice(1).replace(/_/g, " ");

        return (
          <PropertyItem key={key} label={title} description={description}>
            {renderValue(data[key])}
          </PropertyItem>
        );
      })}
    </div>
  );
};

export default PropertiesRenderer;
