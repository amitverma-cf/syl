import { useState } from "react";
import { useInvokeResource } from "./useInvokeResource";

export type ContributionKind = "settingsPane" | "sidebarView" | "statusBarItem" | "command";

export interface ExtensionContribution {
  extensionId: string;
  kind: ContributionKind;
  id: string;
  title: string;
}

export function useContributions(): ExtensionContribution[] {
  const [contributions, setContributions] = useState<ExtensionContribution[]>([]);
  useInvokeResource<ExtensionContribution[]>("list_contributions", setContributions, () => {
    // Best-effort: an extension-discovery failure shouldn't block the shell
    // from rendering — contributed UI entries just won't appear.
  });
  return contributions;
}
