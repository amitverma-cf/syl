import type { HTMLAttributes, ReactNode } from "react";

export interface OverlayProps extends Omit<HTMLAttributes<HTMLDivElement>, "onClose"> {
  onClose: () => void;
  children: ReactNode;
}

function Overlay({ onClose, className = "", onKeyDown, children, ...rest }: OverlayProps) {
  return (
    <div
      className={`ui-overlay${className ? ` ${className}` : ""}`}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
        onKeyDown?.(e);
      }}
      {...rest}
    >
      {children}
    </div>
  );
}

export default Overlay;
