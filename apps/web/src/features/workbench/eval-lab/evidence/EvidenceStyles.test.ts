import { describe, expect, it } from "vitest";

import styles from "./EvidencePicker.module.css";
import picker from "./EvidencePicker.tsx?raw";
import review from "./EvidenceSelectionReview.tsx?raw";
import savePanel from "./SaveEvidenceToEvalPanel.tsx?raw";
import stateList from "./EvidenceStateList.tsx?raw";

const componentSources = [picker, review, stateList, savePanel] as const;

describe("evidence CSS module", () => {
  it("defines every statically referenced class and evidence severity", () => {
    const referencedClasses = new Set(
      componentSources.flatMap((source) =>
        [...source.matchAll(/styles\.([A-Za-z_]\w*)/g)].map(
          (match) => match[1],
        ),
      ),
    );

    for (const className of referencedClasses) {
      expect(styles[className], `missing .${className}`).toBeTypeOf("string");
    }
    for (const severity of ["success", "warning", "danger", "neutral"]) {
      expect(styles[severity], `missing dynamic .${severity}`).toBeTypeOf(
        "string",
      );
    }
  });
});
