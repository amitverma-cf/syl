import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { useShellStore } from "../store/shellStore";
import OnboardingOverlay from "./OnboardingOverlay";

function resetStore() {
  localStorage.clear();
  useShellStore.setState({
    onboardingOpen: false,
    onboardingDismissed: false,
    settingsOpen: false,
    settingsPane: "models",
  });
}

describe("OnboardingOverlay", () => {
  beforeEach(resetStore);

  it("renders nothing when onboardingOpen is false", () => {
    const { container } = render(<OnboardingOverlay />);
    expect(container).toBeEmptyDOMElement();
  });

  it("renders the welcome card when onboardingOpen is true", () => {
    useShellStore.setState({ onboardingOpen: true });
    render(<OnboardingOverlay />);
    expect(screen.getByText("Welcome to syl")).toBeInTheDocument();
  });

  it("clicking the backdrop dismisses onboarding and persists the dismissal", () => {
    useShellStore.setState({ onboardingOpen: true });
    const { container } = render(<OnboardingOverlay />);

    fireEvent.click(container.querySelector(".onboarding-overlay")!);

    expect(useShellStore.getState().onboardingOpen).toBe(false);
    expect(useShellStore.getState().onboardingDismissed).toBe(true);
    expect(localStorage.getItem("syl:onboarded")).toBe("1");
  });

  it("clicking inside the card does not dismiss (only the backdrop itself should)", () => {
    useShellStore.setState({ onboardingOpen: true });
    render(<OnboardingOverlay />);

    fireEvent.click(screen.getByText("Welcome to syl"));

    expect(useShellStore.getState().onboardingOpen).toBe(true);
  });

  it("'Download a local model' dismisses onboarding and opens Settings on the models pane", () => {
    useShellStore.setState({ onboardingOpen: true });
    render(<OnboardingOverlay />);

    fireEvent.click(screen.getByText("Download a local model"));

    const s = useShellStore.getState();
    expect(s.onboardingOpen).toBe(false);
    expect(s.settingsOpen).toBe(true);
    expect(s.settingsPane).toBe("models");
  });

  it("'Connect a cloud provider' dismisses onboarding and opens Settings on the providers pane", () => {
    useShellStore.setState({ onboardingOpen: true });
    render(<OnboardingOverlay />);

    fireEvent.click(screen.getByText("Connect a cloud provider"));

    const s = useShellStore.getState();
    expect(s.onboardingOpen).toBe(false);
    expect(s.settingsPane).toBe("providers");
  });

  it("'Skip for now' dismisses onboarding without opening Settings", () => {
    useShellStore.setState({ onboardingOpen: true });
    render(<OnboardingOverlay />);

    fireEvent.click(screen.getByText("Skip for now"));

    const s = useShellStore.getState();
    expect(s.onboardingOpen).toBe(false);
    expect(s.settingsOpen).toBe(false);
  });
});
