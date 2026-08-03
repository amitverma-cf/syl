import type { SelectHTMLAttributes } from "react";

export type SelectProps = SelectHTMLAttributes<HTMLSelectElement>;

function Select({ className = "", ...rest }: SelectProps) {
  return <select className={`ui-select${className ? ` ${className}` : ""}`} {...rest} />;
}

export default Select;
