import React from "react";

interface CardProps {
  children: React.ReactNode;
  className?: string;
  padding?: "sm" | "md" | "lg";
  id?: string;
}

const Card: React.FC<CardProps> = ({
  children,
  className = "",
  padding = "md",
  id,
}) => {
  const paddingClasses = {
    sm: "p-3 sm:p-4 lg:p-4 xl:p-5",
    md: "p-4 sm:p-5 lg:p-6 xl:p-8",
    lg: "p-5 sm:p-6 lg:p-8 xl:p-10",
  };

  const baseClasses = `${paddingClasses[padding]} border border-opacity-10`;

  return (
    <div
      id={id}
      className={`${baseClasses} ${className}`.trim()}
      style={{
        backgroundColor: "var(--bg-secondary)",
        borderColor: "var(--border-primary)",
        borderRadius: "var(--border-radius)",
      }}
    >
      {children}
    </div>
  );
};

export const CardHeader: React.FC<{
  children: React.ReactNode;
  className?: string;
}> = ({ children, className = "" }) => (
  <div
    className={`p-4 sm:p-5 lg:p-6 xl:p-8 ${className}`.trim()}
    style={{
      backgroundColor: "var(--bg-secondary)",
      color: "var(--text-primary)",
      borderColor: "var(--border-primary)",
      borderTopLeftRadius: "var(--border-radius)",
      borderTopRightRadius: "var(--border-radius)",
    }}
  >
    {children}
  </div>
);

export const CardContent: React.FC<{
  children: React.ReactNode;
  className?: string;
  padding?: "sm" | "md" | "lg";
}> = ({ children, className = "", padding = "md" }) => {
  const paddingClasses = {
    sm: "p-3 sm:p-4 lg:p-4 xl:p-5",
    md: "p-4 sm:p-5 lg:p-6 xl:p-8",
    lg: "p-5 sm:p-6 lg:p-8 xl:p-10",
  };

  return (
    <div
      className={`${paddingClasses[padding]} ${className}`.trim()}
      style={{ color: "var(--text-primary)" }}
    >
      {children}
    </div>
  );
};

export default Card;
