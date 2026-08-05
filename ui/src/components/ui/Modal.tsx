import type { ReactNode } from "react";
import Overlay from "./Overlay";

export interface ModalProps {
  onClose: () => void;
  title?: string;
  children: ReactNode;
}

function Modal({ onClose, title, children }: ModalProps) {
  return (
    <Overlay onClose={onClose} className="ui-modal-overlay">
      <div className="ui-modal" role="dialog" aria-modal="true" aria-label={title}>
        {title && <div className="ui-modal-title">{title}</div>}
        <div className="ui-modal-body">{children}</div>
      </div>
    </Overlay>
  );
}

export default Modal;
