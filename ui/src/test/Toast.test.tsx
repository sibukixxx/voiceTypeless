import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { ToastContainer } from "../components/Toast";
import { useToastStore } from "../store/toastStore";

describe("ToastContainer", () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
  });

  it("renders nothing when no toasts", () => {
    const { container } = render(<ToastContainer />);
    expect(container.firstChild).toBeNull();
  });

  it("shows toast message", () => {
    render(<ToastContainer />);
    act(() => {
      useToastStore.getState().addToast("error", "Something went wrong");
    });
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
  });

  it("shows success toast", () => {
    render(<ToastContainer />);
    act(() => {
      useToastStore.getState().addToast("success", "Copied!");
    });
    expect(screen.getByText("Copied!")).toBeInTheDocument();
  });
});
