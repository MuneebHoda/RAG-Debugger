import { useReducedMotion } from "motion/react";
import { useCallback, useEffect, useState } from "react";

import {
  commandCenterScenarios,
  getCommandCenterScenario,
  type CommandCenterOutcome,
} from "./commandCenterData";

export const COMMAND_CENTER_INTERVAL_MS = 7_000;

export function useCommandCenterSimulation() {
  const reducedMotion = useReducedMotion();
  const [activeId, setActiveId] = useState<CommandCenterOutcome>("failing");
  const [isPlaying, setIsPlaying] = useState(true);
  const playbackActive = isPlaying && !reducedMotion;

  useEffect(() => {
    if (!playbackActive) return undefined;

    const interval = window.setInterval(() => {
      setActiveId((currentId) => {
        const currentIndex = commandCenterScenarios.findIndex(
          (scenario) => scenario.id === currentId,
        );
        const nextIndex = (currentIndex + 1) % commandCenterScenarios.length;
        return commandCenterScenarios[nextIndex].id;
      });
    }, COMMAND_CENTER_INTERVAL_MS);

    return () => window.clearInterval(interval);
  }, [playbackActive]);

  const selectScenario = useCallback((id: CommandCenterOutcome) => {
    setActiveId(id);
    setIsPlaying(false);
  }, []);

  const togglePlayback = useCallback(() => {
    if (!reducedMotion) setIsPlaying((current) => !current);
  }, [reducedMotion]);

  return {
    activeScenario: getCommandCenterScenario(activeId),
    isPlaying: playbackActive,
    reducedMotion: Boolean(reducedMotion),
    selectScenario,
    togglePlayback,
  };
}
