import React from "react";

interface UserAvatarProps {
  name: string;
  size?: "sm" | "md" | "lg" | "xl";
  className?: string;
}

const UserAvatar: React.FC<UserAvatarProps> = ({
  name,
  size = "md",
  className = "",
}) => {
  const initial = name ? name.charAt(0).toUpperCase() : "?";

  const sizeClasses = {
    sm: "w-8 h-8 text-xs",
    md: "w-10 h-10 text-sm",
    lg: "w-12 h-12 text-base",
    xl: "w-16 h-16 text-xl",
  };

  return (
    <div
      className={`rounded-full flex items-center justify-center font-semibold border-2 ${sizeClasses[size]} ${className}`}
      style={{
        backgroundColor: "var(--text-primary)",
        color: "var(--text-inverse)",
        borderColor: "var(--bg-primary)",
      }}
    >
      {initial}
    </div>
  );
};

export default UserAvatar;
