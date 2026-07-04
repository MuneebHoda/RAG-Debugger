import { m } from "motion/react";

import type { EvidenceReactorState } from "./evidenceReactorState";
import { motionTransition } from "./motion";

type EvidenceReactorDeckProps = {
  state: EvidenceReactorState;
  gateColor: string;
  reducedMotion: boolean;
};

const pathTransition = { ...motionTransition, duration: 0.7 };

export function EvidenceReactorDeck({
  state,
  gateColor,
  reducedMotion,
}: EvidenceReactorDeckProps) {
  const supportedPathLength = state.coverage / 100;
  const nodes = [
    { x: 236, y: 82, label: "Source", active: true },
    { x: 468, y: 112, label: "Chunks", active: true },
    { x: 700, y: 114, label: "Rank", active: true },
    { x: 926, y: 94, label: "Support", active: state.coverage > 0 },
  ];

  return (
    <svg fill="none" height="100%" viewBox="0 0 1400 160" width="100%">
      <defs>
        <linearGradient id="reactor-deck-flow" x1="0" x2="1">
          <stop stopColor="#70b6b3" stopOpacity="0.24" />
          <stop offset="0.58" stopColor="#31daca" stopOpacity="0.8" />
          <stop offset="1" stopColor={gateColor} />
        </linearGradient>
      </defs>
      <path
        d="M 118 54 C 360 128, 716 138, 1004 92 S 1198 48, 1290 28"
        stroke="#70b6b3"
        strokeDasharray="5 12"
        strokeOpacity="0.58"
        strokeWidth="2"
      />
      <m.path
        animate={{ pathLength: supportedPathLength }}
        d="M 118 54 C 360 128, 716 138, 1004 92 S 1198 48, 1290 28"
        initial={false}
        stroke="url(#reactor-deck-flow)"
        strokeLinecap="round"
        strokeWidth="4"
        transition={reducedMotion ? { duration: 0 } : pathTransition}
      />
      {nodes.map((node, index) => {
        const tone = index < 3 ? "#31daca" : gateColor;
        return (
          <m.g
            animate={{
              opacity: node.active ? 1 : 0.58,
              scale: node.active ? 1 : 0.92,
            }}
            initial={false}
            key={node.x}
            style={{ transformOrigin: `${node.x}px ${node.y}px` }}
            transition={reducedMotion ? { duration: 0 } : motionTransition}
          >
            <polygon
              fill={node.active ? "#153f3f" : "#0a1d21"}
              points={`${node.x - 34},${node.y} ${node.x},${node.y - 14} ${node.x + 34},${node.y} ${node.x},${node.y + 15}`}
              stroke={node.active ? tone : "#70b6b3"}
              strokeOpacity={node.active ? 0.84 : 0.5}
            />
            <circle
              cx={node.x}
              cy={node.y}
              fill={node.active ? tone : "#70b6b3"}
              r="3"
            />
            <text
              fill="#70b6b3"
              fontSize="10"
              fontWeight="700"
              letterSpacing="1.4"
              textAnchor="middle"
              x={node.x}
              y={node.y + 38}
            >
              {node.label.toUpperCase()}
            </text>
            {index === 2 && state.hasDuplicateBranch ? (
              <path
                d={`M ${node.x} ${node.y + 15} l 34 22`}
                stroke="#f4bd75"
                strokeDasharray="3 5"
              />
            ) : null}
          </m.g>
        );
      })}
      <polygon
        fill="#0d272c"
        points="1080,62 1142,36 1196,62 1134,89"
        stroke={gateColor}
        strokeOpacity="0.82"
      />
      <circle cx="1138" cy="62" fill={gateColor} r="6" />
      <text
        fill={gateColor}
        fontSize="10"
        fontWeight="700"
        letterSpacing="1.4"
        textAnchor="middle"
        x="1138"
        y="112"
      >
        AUDIT
      </text>
    </svg>
  );
}
