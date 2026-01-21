import React from "react";

interface ActionButtonProps {
  children: React.ReactNode;
  onClick?: () => void;
  variant?: "primary" | "secondary" | "danger" | "ghost" | "link" | "icon";
  size?: "xs" | "sm" | "md" | "lg" | "xl";
  disabled?: boolean;
  type?: "button" | "submit" | "reset";
  className?: string;
  loading?: boolean;
  fullWidth?: boolean;
  icon?: boolean;
  title?: string;
  style?: React.CSSProperties;
}

const ActionButton: React.FC<ActionButtonProps> = ({
  children,
  onClick,
  variant = "secondary",
  size = "md",
  disabled = false,
  type = "button",
  className = "",
  loading = false,
  fullWidth = false,
  icon = false,
  title,
  style,
}) => {
  const getSizeClasses = () => {
    if (icon) {
      // Icon button sizes
      switch (size) {
        case "xs":
          return "w-6 h-6 text-xs";
        case "sm":
          return "w-8 h-8 text-sm";
        case "md":
          return "w-10 h-10 text-base";
        case "lg":
          return "w-12 h-12 text-lg";
        case "xl":
          return "w-14 h-14 text-xl";
        default:
          return "w-10 h-10 text-base";
      }
    } else {
      // Regular button sizes with responsive scaling
      switch (size) {
        case "xs":
          return "px-2 py-1 text-xs";
        case "sm":
          return "px-3 py-1.5 text-xs sm:text-sm";
        case "md":
          return "px-4 py-2 lg:px-5 lg:py-3 xl:px-6 xl:py-3 text-sm lg:text-sm xl:text-base";
        case "lg":
          return "px-6 py-3 lg:px-8 lg:py-4 xl:px-10 xl:py-5 text-base lg:text-lg xl:text-xl";
        case "xl":
          return "px-8 py-4 lg:px-10 lg:py-5 xl:px-12 xl:py-6 text-lg lg:text-xl xl:text-2xl";
        default:
          return "px-4 py-2 lg:px-5 lg:py-3 xl:px-6 xl:py-3 text-sm lg:text-sm xl:text-base";
      }
    }
  };

  const getVariantStyles = () => {
    switch (variant) {
      case "primary":
        return {
          backgroundColor: "var(--button-primary-bg)",
          color: "var(--text-primary)",
          borderColor: "var(--button-primary-bg)",
          border: "1px solid",
          borderRadius: !icon ? "var(--border-radius)" : "9999px",
        };
      case "secondary":
        return {
          backgroundColor: "var(--button-secondary-bg)",
          color: "var(--text-primary)",
          borderColor: "var(--border-primary)",
          border: "1px solid",
          borderRadius: !icon ? "var(--border-radius)" : "9999px",
        };
      case "danger":
        return {
          backgroundColor: "var(--button-danger-bg)",
          color: "var(--text-primary)",
          borderColor: "var(--button-danger-bg)",
          border: "1px solid",
          borderRadius: !icon ? "var(--border-radius)" : "9999px",
        };
      case "ghost":
        return {
          backgroundColor: "transparent",
          color: "var(--text-secondary)",
          border: "none",
          borderRadius: !icon ? "var(--border-radius)" : "9999px",
        };
      case "link":
        return {
          backgroundColor: "transparent",
          color: "var(--text-tertiary)",
          border: "none",
          textDecoration: "none",
          borderRadius: "0",
        };
      case "icon":
        return {
          backgroundColor: "transparent",
          color: "var(--text-secondary)",
          border: "none",
          borderRadius: "9999px",
        };
      default:
        return {
          backgroundColor: "var(--button-secondary-bg)",
          color: "var(--text-primary)",
          borderColor: "var(--border-primary)",
          border: "1px solid",
          borderRadius: !icon ? "var(--border-radius)" : "9999px",
        };
    }
  };

  const getHoverStyles = () => {
    switch (variant) {
      case "primary":
        return {
          backgroundColor: "var(--button-primary-hover)",
        };
      case "secondary":
        return {
          backgroundColor: "var(--button-secondary-hover)",
        };
      case "danger":
        return {
          backgroundColor: "var(--button-danger-hover)",
          borderColor: "var(--button-danger-hover)",
        };
      case "ghost":
        return {
          backgroundColor: "var(--bg-hover)",
          color: "var(--text-primary)",
        };
      case "link":
        return {
          textDecoration: "underline",
        };
      case "icon":
        return {
          backgroundColor: "var(--bg-hover)",
          color: "var(--text-primary)",
        };
      default:
        return {
          backgroundColor: "var(--button-secondary-hover)",
        };
    }
  };

  const baseClasses = [
    // Base styles
    "inline-flex items-center justify-center font-medium cursor-pointer transition-all duration-150 select-none",

    // Conditional classes
    icon ? "rounded-full flex-shrink-0" : "gap-2",
    fullWidth ? "w-full" : "",
    disabled || loading ? "opacity-60 cursor-not-allowed" : "",
    variant === "link" ? "hover:underline p-0 bg-transparent border-none" : "",

    // Size classes
    getSizeClasses(),
  ]
    .filter(Boolean)
    .join(" ");

  const handleMouseEnter = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (disabled || loading) return;
    const hoverStyles = getHoverStyles();
    Object.assign(e.currentTarget.style, hoverStyles);
  };

  const handleMouseLeave = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (disabled || loading) return;
    const originalStyles = getVariantStyles();
    Object.assign(e.currentTarget.style, originalStyles);
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (disabled || loading) return;
    e.currentTarget.style.transform = "scale(0.95)";
    e.currentTarget.style.opacity = "0.8";
  };

  const handleMouseUp = (e: React.MouseEvent<HTMLButtonElement>) => {
    if (disabled || loading) return;
    e.currentTarget.style.transform = "scale(1)";
    e.currentTarget.style.opacity = "1";
  };

  const handleClick = () => {
    if (!disabled && !loading && onClick) {
      onClick();
    }
  };

  return (
    <button
      type={type}
      className={`${baseClasses} ${className}`.trim()}
      style={{
        ...getVariantStyles(),
        ...(!icon && { borderRadius: "var(--border-radius)" }),
        ...style
      }}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      disabled={disabled || loading}
      {...(title && { title })}
    >
      {loading && (
        <svg
          className="animate-spin -ml-1 mr-2 h-4 w-4"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          ></circle>
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          ></path>
        </svg>
      )}
      {children}
    </button>
  );
};

// Create Button alias for consistency
export const Button = ActionButton;

export default ActionButton;
