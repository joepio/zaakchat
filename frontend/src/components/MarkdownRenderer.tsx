import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface MarkdownRendererProps {
  content: string;
  truncated?: boolean;
  className?: string;
}

const MarkdownRenderer: React.FC<MarkdownRendererProps> = ({
  content,
  truncated = false,
  className = "",
}) => {
  if (!content) return null;

  // Use prose class for typography defaults
  // prose-sm for smaller text, prose-stone specifically for gray scales matching the theme
  const baseClasses = "prose prose-sm max-w-none dark:prose-invert prose-stone";

  // Custom styles to match the application theme more closely if needed
  // This uses Tailwind Typography but we might want to override some specific colors
  // to use our CSS variables if the default prose colors don't match.
  // For now, standard prose should be good enough.

  return (
    <div className={`${baseClasses} ${className} ${truncated ? "line-clamp-3" : ""}`}>
      <ReactMarkdown remarkPlugins={[remarkGfm]}>
        {content}
      </ReactMarkdown>
    </div>
  );
};

export default MarkdownRenderer;
