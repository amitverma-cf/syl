import { useState, type CSSProperties, type ReactNode } from "react";
import { IconChevronRight } from "@tabler/icons-react";

export interface DropdownMenuItem {
  key: string;
  label: string;
  sublabel?: string;
  selected?: boolean;
  onSelect?: () => void;
  children?: DropdownMenuItem[];
  /** Extra DOM attributes (e.g. data-testid) spread onto the row for targeting in tests. */
  attrs?: Record<string, string>;
}

export interface DropdownMenuGroup {
  label?: string;
  items: DropdownMenuItem[];
}

export interface DropdownMenuProps {
  /** The trigger element. Wire its own onClick to `onOpenChange(!open)`. */
  trigger: ReactNode;
  groups: DropdownMenuGroup[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  emptyLabel?: string;
  menuClassName?: string;
  menuStyle?: CSSProperties;
}

function MenuItemRow({ item, depth, onSelect }: { item: DropdownMenuItem; depth: number; onSelect: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const hasChildren = !!item.children && item.children.length > 0;

  return (
    <>
      <div
        className={`option-item${item.selected ? " sel" : ""}`}
        style={{ paddingLeft: 9 + depth * 14 }}
        {...item.attrs}
        onClick={() => {
          if (hasChildren) {
            setExpanded((v) => !v);
          } else {
            item.onSelect?.();
            onSelect();
          }
        }}
      >
        <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
          {hasChildren && (
            <IconChevronRight
              size={11}
              aria-hidden
              style={{ transform: expanded ? "rotate(90deg)" : undefined, transition: "transform 0.1s" }}
            />
          )}
          {item.label}
        </span>
        {item.sublabel && <span className="option-sub">{item.sublabel}</span>}
      </div>
      {hasChildren &&
        expanded &&
        item.children!.map((child) => (
          <MenuItemRow key={child.key} item={child} depth={depth + 1} onSelect={onSelect} />
        ))}
    </>
  );
}

function DropdownMenu({
  trigger,
  groups,
  open,
  onOpenChange,
  emptyLabel,
  menuClassName = "",
  menuStyle,
}: DropdownMenuProps) {
  const isEmpty = groups.every((g) => g.items.length === 0);

  return (
    <div className="dropdown-wrap">
      {trigger}
      {open && (
        <div
          className={`option-menu open${menuClassName ? ` ${menuClassName}` : ""}`}
          style={menuStyle}
          onMouseLeave={() => onOpenChange(false)}
        >
          {isEmpty && emptyLabel && <div className="cmdk-empty">{emptyLabel}</div>}
          {groups.map((group, gi) => (
            <div key={group.label ?? gi}>
              {group.label && (
                <div className="cmdk-group-label" style={{ padding: "4px 8px" }}>
                  {group.label}
                </div>
              )}
              {group.items.map((item) => (
                <MenuItemRow key={item.key} item={item} depth={0} onSelect={() => onOpenChange(false)} />
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default DropdownMenu;
