import React from "react";

interface CodeBlockProps {
  code: string;
  language?: string;
}

const CodeBlock: React.FC<CodeBlockProps> = ({ code, language = "json" }) => {
  return (
    <pre
      className="p-4 rounded-lg overflow-x-auto text-sm"
      style={{
        backgroundColor: "var(--bg-tertiary)",
        color: "var(--text-primary)",
      }}
    >
      <code className={`language-${language}`}>{code}</code>
    </pre>
  );
};

export default CodeBlock;
