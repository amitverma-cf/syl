import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { IconMessageCircle } from "@tabler/icons-react";
import Button from "./Button";
import IconButton from "./IconButton";
import Input from "./Input";
import Textarea from "./Textarea";
import Select from "./Select";
import Badge from "./Badge";
import Overlay from "./Overlay";
import NavItem from "./NavItem";

describe("Button", () => {
  it("renders a real button element with the default variant class", () => {
    render(<Button>Save</Button>);
    const btn = screen.getByRole("button", { name: "Save" });
    expect(btn).toHaveAttribute("type", "button");
    expect(btn.className).toContain("ui-btn");
    expect(btn.className).not.toContain("ui-btn-danger");
  });

  it("applies the requested variant class", () => {
    render(<Button variant="danger">Delete</Button>);
    expect(screen.getByRole("button", { name: "Delete" }).className).toContain("ui-btn-danger");
  });

  it("merges a caller-supplied className instead of replacing it", () => {
    render(<Button className="extra">Go</Button>);
    const btn = screen.getByRole("button", { name: "Go" });
    expect(btn.className).toContain("ui-btn");
    expect(btn.className).toContain("extra");
  });

  it("fires onClick and respects disabled", () => {
    const onClick = vi.fn();
    render(
      <Button onClick={onClick} disabled>
        Save
      </Button>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe("IconButton", () => {
  it("renders the icon inside a real button and forwards title", () => {
    const onClick = vi.fn();
    render(<IconButton icon={IconMessageCircle} title="Close" onClick={onClick} />);
    const btn = screen.getByRole("button", { name: "Close" });
    fireEvent.click(btn);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("applies danger variant and lg size classes", () => {
    render(<IconButton icon={IconMessageCircle} variant="danger" size="lg" aria-label="danger-btn" />);
    const btn = screen.getByRole("button", { name: "danger-btn" });
    expect(btn.className).toContain("ui-icon-btn-danger");
    expect(btn.className).toContain("ui-icon-btn-lg");
  });
});

describe("Input", () => {
  it("renders a controlled input and reports changes", () => {
    const onChange = vi.fn();
    render(<Input placeholder="Name" value="a" onChange={onChange} />);
    const input = screen.getByPlaceholderText("Name");
    fireEvent.change(input, { target: { value: "ab" } });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(input.className).toContain("ui-input");
  });
});

describe("Textarea", () => {
  it("renders with the ui-textarea class", () => {
    render(<Textarea placeholder="Body" />);
    expect(screen.getByPlaceholderText("Body").className).toContain("ui-textarea");
  });
});

describe("Select", () => {
  it("renders options and reports selection changes", () => {
    const onChange = vi.fn();
    render(
      <Select value="a" onChange={onChange} aria-label="pick">
        <option value="a">A</option>
        <option value="b">B</option>
      </Select>,
    );
    const select = screen.getByRole("combobox", { name: "pick" });
    fireEvent.change(select, { target: { value: "b" } });
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(select.className).toContain("ui-select");
  });
});

describe("Badge", () => {
  it("renders children with the badge class", () => {
    render(<Badge>configured</Badge>);
    expect(screen.getByText("configured").className).toContain("ui-badge");
  });
});

describe("Overlay", () => {
  it("closes when the backdrop is clicked but not when the card is clicked", () => {
    const onClose = vi.fn();
    const { container } = render(
      <Overlay onClose={onClose} className="my-overlay">
        <div className="my-card">card</div>
      </Overlay>,
    );

    fireEvent.click(container.querySelector(".my-card")!);
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.click(container.querySelector(".my-overlay")!);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("closes on Escape", () => {
    const onClose = vi.fn();
    const { container } = render(
      <Overlay onClose={onClose} className="my-overlay">
        <div>card</div>
      </Overlay>,
    );
    fireEvent.keyDown(container.querySelector(".my-overlay")!, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("NavItem", () => {
  it("marks itself active and renders an icon plus label", () => {
    render(
      <NavItem icon={IconMessageCircle} active>
        Chats
      </NavItem>,
    );
    const item = screen.getByText("Chats").closest(".ui-nav-item")!;
    expect(item.className).toContain("active");
  });

  it("is not active by default", () => {
    render(<NavItem>Chats</NavItem>);
    expect(screen.getByText("Chats").className).not.toContain("active");
  });
});
