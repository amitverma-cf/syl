import { IconSparkles, IconDownload, IconPlug } from "@tabler/icons-react";
import { useShellStore } from "../store/shellStore";
import { Overlay } from "../components/ui";

function OnboardingOverlay() {
  const { onboardingOpen, dismissOnboarding, openSettings } = useShellStore();

  if (!onboardingOpen) return null;

  return (
    <Overlay className="onboarding-overlay open" onClose={dismissOnboarding}>
      <div className="onboarding-card">
        <IconSparkles className="logo" aria-hidden />
        <h2>Welcome to syl</h2>
        <p>
          A local-first agentic assistant. Get set up in a minute — download a model to run fully
          offline, or connect a cloud provider for the frontier models.
        </p>
        <div className="onboarding-actions">
          <div
            className="onboarding-btn primary"
            onClick={() => {
              dismissOnboarding();
              openSettings("models");
            }}
          >
            <IconDownload aria-hidden />
            <div>
              <div className="ob-title">Download a local model</div>
              <div className="ob-sub">Runs fully offline, no API key needed</div>
            </div>
          </div>
          <div
            className="onboarding-btn"
            onClick={() => {
              dismissOnboarding();
              openSettings("providers");
            }}
          >
            <IconPlug aria-hidden />
            <div>
              <div className="ob-title">Connect a cloud provider</div>
              <div className="ob-sub">Use OpenAI, Anthropic, or another API key</div>
            </div>
          </div>
        </div>
        <div className="onboarding-skip" onClick={dismissOnboarding}>
          Skip for now
        </div>
      </div>
    </Overlay>
  );
}

export default OnboardingOverlay;
