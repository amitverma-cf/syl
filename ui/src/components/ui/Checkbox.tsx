import type { InputHTMLAttributes } from "react";

export type CheckboxProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type">;

function Checkbox({ className = "", ...rest }: CheckboxProps) {
  return (
    <input
      type="checkbox"
      className={`ui-checkbox${className ? ` ${className}` : ""}`}
      {...rest}
    />
  );
}

export default Checkbox;
