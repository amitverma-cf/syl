import type { TextareaHTMLAttributes } from "react";

export type TextareaProps = TextareaHTMLAttributes<HTMLTextAreaElement>;

function Textarea({ className = "", ...rest }: TextareaProps) {
  return <textarea className={`ui-textarea${className ? ` ${className}` : ""}`} {...rest} />;
}

export default Textarea;
