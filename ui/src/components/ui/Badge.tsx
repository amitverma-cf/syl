import type { HTMLAttributes } from "react";

export type BadgeProps = HTMLAttributes<HTMLSpanElement>;

function Badge({ className = "", ...rest }: BadgeProps) {
  return <span className={`ui-badge${className ? ` ${className}` : ""}`} {...rest} />;
}

export default Badge;
