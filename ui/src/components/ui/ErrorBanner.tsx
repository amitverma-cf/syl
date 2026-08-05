import type { ReactNode } from "react";

export interface ErrorBannerProps {
  children: ReactNode;
}

function ErrorBanner({ children }: ErrorBannerProps) {
  return (
    <div className="ui-error-banner" role="alert">
      {children}
    </div>
  );
}

export default ErrorBanner;
