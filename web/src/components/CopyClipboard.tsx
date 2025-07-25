import { useState } from "react";
import { MdContentCopy, MdDone } from "react-icons/md";

function getPlainText(children) {
  return children.filter((child) => typeof child === "string").join("\n");
}

async function copyToClipboard(textToCopy: string) {
  if (navigator.clipboard && window.isSecureContext) {
    await navigator.clipboard.writeText(textToCopy);
  } else {
    const textArea = document.createElement("textarea");
    textArea.value = textToCopy;

    textArea.style.position = "absolute";
    textArea.style.left = "-999999px";

    document.body.prepend(textArea);
    textArea.select();

    try {
      document.execCommand("copy");
    } catch (error) {
      console.error(error);
    } finally {
      textArea.remove();
    }
  }
}

const CopyClipboard = ({ children, className }) => {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    try {
      await copyToClipboard(
        typeof children === "string" ? children : getPlainText(children),
      );
      setCopied(true);
      setTimeout(() => setCopied(false), 2000); // Reset after 2 seconds
    } catch (err) {
      console.error("Failed to copy!", err);
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
