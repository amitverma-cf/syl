import type { ButtonHTMLAttributes } from "react";

export type ButtonVariant = "default" | "ghost" | "danger";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
}

function Button({ variant = "default", className = "", type = "button", ...rest }: ButtonProps) {
  const variantClass = variant === "default" ? "" : ` ui-btn-${variant}`;
  return (
    <button
      type={type}
      className={`ui-btn${variantClass}${className ? ` ${className}` : ""}`}
      {...rest}
    />
  );
}

export default Button;
