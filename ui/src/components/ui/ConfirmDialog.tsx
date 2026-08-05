import { IconCheck, IconX } from "@tabler/icons-react";

export interface ConfirmDialogProps {
  confirming: boolean;
  onRequestConfirm: () => void;
  onConfirm: () => void;
  onCancel: () => void;
  triggerIcon: React.ReactNode;
  label: string;
  className?: string;
  triggerTestId?: string;
  confirmTestId?: string;
  cancelTestId?: string;
}

/// Inline confirm-in-place interaction: a trigger icon that, once clicked,
/// swaps to a check/✕ pair — the same pattern already used independently for
/// conversation delete, folder-tree delete, and flow delete, unified here.
function ConfirmDialog({
  confirming,
  onRequestConfirm,
  onConfirm,
  onCancel,
  triggerIcon,
  label,
  className = "",
  triggerTestId,
  confirmTestId,
  cancelTestId,
}: ConfirmDialogProps) {
  if (confirming) {
    return (
      <span className="ui-confirm-pair">
        <button
          type="button"
          className={className}
          data-testid={confirmTestId}
          aria-label={`Confirm ${label}`}
          onClick={(e) => {
            e.stopPropagation();
            onConfirm();
          }}
        >
          <IconCheck size={12} aria-hidden />
        </button>
        <button
          type="button"
          className={className}
          data-testid={cancelTestId}
          aria-label={`Cancel ${label}`}
          onClick={(e) => {
            e.stopPropagation();
            onCancel();
          }}
        >
          <IconX size={12} aria-hidden />
        </button>
      </span>
    );
  }

  return (
    <button
      type="button"
      className={className}
      data-testid={triggerTestId}
      aria-label={label}
      onClick={(e) => {
        e.stopPropagation();
        onRequestConfirm();
      }}
    >
      {triggerIcon}
    </button>
  );
}

export default ConfirmDialog;
