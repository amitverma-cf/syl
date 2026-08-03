import type { ComponentType, HTMLAttributes } from "react";

export interface NavItemProps extends HTMLAttributes<HTMLDivElement> {
  icon?: ComponentType<{ size?: number; "aria-hidden"?: boolean }>;
  active?: boolean;
  iconSize?: number;
}

function NavItem({ icon: Icon, active = false, iconSize = 14, className = "", children, ...rest }: NavItemProps) {
  return (
    <div
      className={`ui-nav-item${active ? " active" : ""}${className ? ` ${className}` : ""}`}
      {...rest}
    >
      {Icon && <Icon size={iconSize} aria-hidden />}
      {children}
    </div>
  );
}

export default NavItem;
