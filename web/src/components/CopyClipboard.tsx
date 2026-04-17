import type { ReactNode } from "react";

const CopyClipboard = ({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) => {
  return (
    <div className={`flex justify-between items-center ${className}`}>
      <span>{children}</span>
    </div>
  );
};

export default CopyClipboard;
