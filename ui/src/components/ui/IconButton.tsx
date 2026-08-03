import type { ButtonHTMLAttributes, ComponentType } from "react";

export type IconButtonVariant = "default" | "danger";
export type IconButtonSize = "sm" | "lg";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: ComponentType<{ size?: number; "aria-hidden"?: boolean }>;
  iconSize?: number;
  variant?: IconButtonVariant;
  size?: IconButtonSize;
}

function IconButton({
  icon: Icon,
  iconSize = 16,
  variant = "default",
  size = "sm",
  className = "",
  type = "button",
  ...rest
}: IconButtonProps) {
  const variantClass = variant === "danger" ? " ui-icon-btn-danger" : "";
  const sizeClass = size === "lg" ? " ui-icon-btn-lg" : "";
  return (
    <button
      type={type}
      className={`ui-icon-btn${sizeClass}${variantClass}${className ? ` ${className}` : ""}`}
      {...rest}
    >
      <Icon size={iconSize} aria-hidden />
    </button>
  );
}

export default IconButton;
