import { useState } from 'react';
import { MdContentCopy, MdDone } from "react-icons/md";

function getPlainText(children) {
  return children.filter(child => typeof child === 'string').join("\n");
}

const CopyClipboard = ({ children, className }) => {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(typeof children === "string" ? children : getPlainText(children));
      setCopied(true);
      setTimeout(() => setCopied(false), 2000); // Reset after 2 seconds
    } catch (err) {
      console.error('Failed to copy!', err);
    }
  };

  return (
    <div className={`flex justify-between items-center ${className}`}>
      <span>{children}</span>
      <button
        onClick={handleCopy}
        className="px-4 py-2 text-slate-950 rounded-lg transition"
      >
        {copied ? <MdDone /> : <MdContentCopy />}
      </button>
    </div>
  );
};

export default CopyClipboard;
