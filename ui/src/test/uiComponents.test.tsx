import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { Button } from "../components/ui/Button";
import { Input } from "../components/ui/Input";
import { Card, CardHeader } from "../components/ui/Card";
import { Select } from "../components/ui/Select";
import { Toggle } from "../components/ui/Toggle";

describe("Button", () => {
  it("renders with children", () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText("Click me")).toBeInTheDocument();
  });

  it("applies variant classes", () => {
    render(<Button variant="danger">Delete</Button>);
    const btn = screen.getByText("Delete");
    expect(btn.className).toContain("bg-red-600");
  });

  it("applies size classes", () => {
    render(<Button size="lg">Big</Button>);
    const btn = screen.getByText("Big");
    expect(btn.className).toContain("px-6");
  });

  it("forwards disabled prop", () => {
    render(<Button disabled>Disabled</Button>);
    expect(screen.getByText("Disabled")).toBeDisabled();
  });

  it("applies secondary variant", () => {
    render(<Button variant="secondary">Sec</Button>);
    expect(screen.getByText("Sec").className).toContain("bg-gray-700");
  });

  it("applies ghost variant", () => {
    render(<Button variant="ghost">Ghost</Button>);
    expect(screen.getByText("Ghost").className).toContain("bg-transparent");
  });

  it("applies sm size", () => {
    render(<Button size="sm">Small</Button>);
    expect(screen.getByText("Small").className).toContain("px-3");
  });
});

describe("Input", () => {
  it("renders with placeholder", () => {
    render(<Input placeholder="Type here" />);
    expect(screen.getByPlaceholderText("Type here")).toBeInTheDocument();
  });

  it("renders label", () => {
    render(<Input label="Name" />);
    expect(screen.getByText("Name")).toBeInTheDocument();
  });

  it("renders error message", () => {
    render(<Input error="Required field" />);
    expect(screen.getByText("Required field")).toBeInTheDocument();
  });

  it("applies error styling", () => {
    render(<Input error="Bad" />);
    const input = screen.getByRole("textbox");
    expect(input.className).toContain("border-red-500");
  });

  it("associates label with input via id", () => {
    render(<Input label="Email" />);
    const label = screen.getByText("Email");
    expect(label.getAttribute("for")).toBe("email");
  });
});

describe("Card", () => {
  it("renders children", () => {
    render(<Card>Card content</Card>);
    expect(screen.getByText("Card content")).toBeInTheDocument();
  });

  it("applies padding by default", () => {
    const { container } = render(<Card>Test</Card>);
    expect(container.firstChild).toHaveClass("p-4");
  });

  it("removes padding when disabled", () => {
    const { container } = render(<Card padding={false}>Test</Card>);
    expect(container.firstChild).not.toHaveClass("p-4");
  });
});

describe("CardHeader", () => {
  it("renders title", () => {
    render(<CardHeader title="Test Title" />);
    expect(screen.getByText("Test Title")).toBeInTheDocument();
  });

  it("renders description", () => {
    render(<CardHeader title="T" description="Some description" />);
    expect(screen.getByText("Some description")).toBeInTheDocument();
  });

  it("renders action node", () => {
    render(<CardHeader title="T" action={<button>Act</button>} />);
    expect(screen.getByText("Act")).toBeInTheDocument();
  });
});

describe("Select", () => {
  const options = [
    { value: "a", label: "Alpha" },
    { value: "b", label: "Beta" },
  ];

  it("renders options", () => {
    render(<Select options={options} />);
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
  });

  it("renders label", () => {
    render(<Select label="Choose" options={options} />);
    expect(screen.getByText("Choose")).toBeInTheDocument();
  });

  it("selects value", () => {
    render(<Select options={options} value="b" onChange={() => {}} />);
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.value).toBe("b");
  });
});

describe("Toggle", () => {
  it("renders unchecked", () => {
    render(<Toggle checked={false} onChange={() => {}} />);
    const btn = screen.getByRole("switch");
    expect(btn.getAttribute("aria-checked")).toBe("false");
  });

  it("renders checked", () => {
    render(<Toggle checked={true} onChange={() => {}} />);
    const btn = screen.getByRole("switch");
    expect(btn.getAttribute("aria-checked")).toBe("true");
  });

  it("calls onChange on click", () => {
    let value = false;
    render(<Toggle checked={value} onChange={(v) => (value = v)} />);
    fireEvent.click(screen.getByRole("switch"));
    expect(value).toBe(true);
  });

  it("renders label", () => {
    render(<Toggle checked={false} onChange={() => {}} label="Enable" />);
    expect(screen.getByText("Enable")).toBeInTheDocument();
  });

  it("respects disabled", () => {
    render(<Toggle checked={false} onChange={() => {}} disabled />);
    expect(screen.getByRole("switch")).toBeDisabled();
  });
});
