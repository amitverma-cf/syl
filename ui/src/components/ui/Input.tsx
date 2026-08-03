import type { InputHTMLAttributes } from "react";

export type InputProps = InputHTMLAttributes<HTMLInputElement>;

function Input({ className = "", ...rest }: InputProps) {
  return <input className={`ui-input${className ? ` ${className}` : ""}`} {...rest} />;
}

export default Input;
