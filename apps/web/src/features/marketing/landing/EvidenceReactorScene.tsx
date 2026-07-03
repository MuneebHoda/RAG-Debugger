import { m } from "motion/react";

import type { CommandCenterScenario } from "./commandCenterData";
import type { EvidenceReactorState } from "./evidenceReactorState";
import { motionTransition } from "./motion";
import styles from "./EvidenceReactor.module.css";

type EvidenceReactorSceneProps = {
  scenario: CommandCenterScenario;
  state: EvidenceReactorState;
  gateColor: string;
  reducedMotion: boolean;
};

const pathTransition = { ...motionTransition, duration: 0.7 };

export function EvidenceReactorScene({
  scenario,
  state,
  gateColor,
  reducedMotion,
}: EvidenceReactorSceneProps) {
  const supportedPathLength = state.coverage / 100;

  return (
    <>
      <svg
        className={styles.desktopReactor}
        display="block"
        fill="none"
        height="100%"
        viewBox="0 0 1400 720"
        width="100%"
      >
        <defs>
          <linearGradient id="reactor-plane" x1="0" x2="1" y1="0" y2="1">
            <stop stopColor="#17383d" />
            <stop offset="1" stopColor="#071216" />
          </linearGradient>
          <linearGradient id="reactor-flow" x1="0" x2="1">
            <stop stopColor="#70b6b3" stopOpacity="0.18" />
            <stop offset="0.58" stopColor="#31daca" stopOpacity="0.72" />
            <stop offset="1" stopColor={gateColor} stopOpacity="0.94" />
          </linearGradient>
          <linearGradient id="reactor-report" x1="0" x2="1" y1="0" y2="1">
            <stop stopColor={gateColor} stopOpacity="0.24" />
            <stop offset="1" stopColor="#0b2024" stopOpacity="0.9" />
          </linearGradient>
        </defs>

        <g>
          <polygon
            fill="#0a1d21"
            points="18,268 144,218 226,256 98,310"
            stroke="#70b6b3"
            strokeOpacity="0.26"
          />
          <polygon
            fill="#0d272c"
            points="32,240 158,190 240,228 112,282"
            stroke="#70b6b3"
            strokeOpacity="0.34"
          />
          <m.polygon
            animate={{ opacity: 1, x: 0, y: 0 }}
            fill="url(#reactor-plane)"
            initial={{ opacity: reducedMotion ? 1 : 0, x: -12, y: 8 }}
            key={`intake-${state.outcome}`}
            points="48,210 174,160 256,198 128,252"
            stroke="#31daca"
            strokeOpacity="0.58"
            transition={reducedMotion ? { duration: 0 } : pathTransition}
          />
          {[0, 1, 2].map((index) => (
            <line
              key={index}
              stroke="#70b6b3"
              strokeOpacity="0.25"
              x1={86 + index * 25}
              x2={173 + index * 25}
              y1={211 - index * 10}
              y2={177 - index * 10}
            />
          ))}
        </g>

        <g>
          {scenario.evidence.map((evidence, index) => {
            const supported = evidence.support === "supported";
            const x = 255 + index * 92;
            const y = 118 + index * 28;
            return (
              <m.g
                animate={{ opacity: 1, y: 0 }}
                initial={{ opacity: reducedMotion ? 1 : 0, y: -8 }}
                key={`${scenario.id}-${evidence.id}`}
                transition={
                  reducedMotion
                    ? { duration: 0 }
                    : { ...motionTransition, delay: index * 0.08 }
                }
              >
                <polygon
                  fill={supported ? "#153f3f" : "#17282c"}
                  points={`${x},${y} ${x + 76},${y - 26} ${x + 112},${y - 8} ${x + 34},${y + 20}`}
                  stroke={supported ? "#d5ff5f" : "#70b6b3"}
                  strokeOpacity={supported ? 0.74 : 0.34}
                />
                <line
                  stroke={supported ? "#d5ff5f" : "#70b6b3"}
                  strokeOpacity={supported ? 0.58 : 0.22}
                  x1={x + 20}
                  x2={x + 76}
                  y1={y - 2}
                  y2={y - 20}
                />
              </m.g>
            );
          })}
        </g>

        <g>
          {[228, 274, 320].map((startY, index) => (
            <m.path
              animate={{ opacity: 1, pathLength: 1 }}
              d={`M 188 ${startY} C 410 ${startY - 170}, 800 ${70 + index * 34}, 1232 ${276 + index * 18}`}
              initial={{
                opacity: reducedMotion ? 1 : 0.2,
                pathLength: reducedMotion ? 1 : 0,
              }}
              key={`${scenario.id}-candidate-${startY}`}
              stroke="#70b6b3"
              strokeDasharray="4 11"
              strokeOpacity="0.26"
              transition={
                reducedMotion
                  ? { duration: 0 }
                  : { ...pathTransition, delay: index * 0.08 }
              }
            />
          ))}
          <m.path
            animate={{ pathLength: supportedPathLength }}
            d="M 190 266 C 420 86, 850 100, 1280 330"
            initial={false}
            stroke="url(#reactor-flow)"
            strokeLinecap="round"
            strokeWidth="3"
            transition={reducedMotion ? { duration: 0 } : pathTransition}
          />
          {state.hasDuplicateBranch ? (
            <m.path
              animate={{ opacity: 0.82, pathLength: 1 }}
              d="M 770 166 C 880 206, 908 272, 972 390"
              initial={{
                opacity: reducedMotion ? 0.82 : 0,
                pathLength: reducedMotion ? 1 : 0,
              }}
              stroke="#f4bd75"
              strokeDasharray="5 8"
              transition={reducedMotion ? { duration: 0 } : pathTransition}
            />
          ) : null}
        </g>

        <g>
          <ellipse
            cx="1290"
            cy="344"
            rx="74"
            ry="126"
            stroke="#70b6b3"
            strokeOpacity="0.18"
          />
          <m.ellipse
            animate={{ opacity: 1, pathLength: 1, scale: 1 }}
            cx="1290"
            cy="344"
            initial={{
              opacity: reducedMotion ? 1 : 0.36,
              pathLength: reducedMotion ? 1 : 0,
              scale: reducedMotion ? 1 : 0.96,
            }}
            key={`gate-${state.gate}`}
            rx="60"
            ry="106"
            stroke={gateColor}
            strokeDasharray={state.gate === "failed" ? "8 8" : undefined}
            strokeWidth="3"
            style={{ transformOrigin: "1290px 344px" }}
            transition={reducedMotion ? { duration: 0 } : pathTransition}
          />
          <circle cx="1290" cy="344" fill={gateColor} r="8" />
          {state.hasUnsupportedBranch ? (
            <path
              d="M 1256 305 L 1324 383"
              stroke="#ff8f84"
              strokeLinecap="round"
              strokeWidth="3"
            />
          ) : null}
        </g>

        <g>
          <polygon
            fill="#091b1f"
            points="1218,500 1332,464 1384,493 1268,532"
            stroke="#70b6b3"
            strokeOpacity="0.2"
          />
          <polygon
            fill="#0c2529"
            points="1206,478 1320,442 1372,471 1256,510"
            stroke="#70b6b3"
            strokeOpacity="0.3"
          />
          <m.polygon
            animate={{ opacity: 1, x: 0, y: 0 }}
            fill="url(#reactor-report)"
            initial={{ opacity: reducedMotion ? 1 : 0, x: 10, y: 8 }}
            key={`output-${state.outcome}`}
            points="1194,456 1308,420 1360,449 1244,488"
            stroke={gateColor}
            strokeOpacity="0.72"
            transition={reducedMotion ? { duration: 0 } : pathTransition}
          />
          <line
            stroke={gateColor}
            strokeOpacity="0.64"
            x1="1232"
            x2="1302"
            y1="461"
            y2="439"
          />
          <circle cx="1330" cy="451" fill={gateColor} r="5" />
        </g>
      </svg>

      <svg
        className={styles.mobileReactor}
        display="block"
        fill="none"
        height="100%"
        viewBox="0 0 720 96"
        width="100%"
      >
        <path
          d="M 44 48 C 182 12, 280 84, 402 48 S 564 18, 676 48"
          stroke="#70b6b3"
          strokeDasharray="4 10"
          strokeOpacity="0.35"
        />
        <m.path
          animate={{ pathLength: supportedPathLength }}
          d="M 44 48 C 182 12, 280 84, 402 48 S 564 18, 676 48"
          initial={false}
          stroke={gateColor}
          strokeLinecap="round"
          strokeWidth="3"
          transition={reducedMotion ? { duration: 0 } : pathTransition}
        />
        {[44, 180, 316, 452, 572, 676].map((x, index) => (
          <circle
            cx={x}
            cy={index % 2 === 0 ? 48 : 34}
            fill={index === 4 ? gateColor : "#0d272c"}
            key={x}
            r={index === 4 ? 9 : 6}
            stroke={index >= 4 ? gateColor : "#70b6b3"}
          />
        ))}
      </svg>
    </>
  );
}
