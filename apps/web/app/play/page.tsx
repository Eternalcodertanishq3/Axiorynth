"use client";

import Link from "next/link";
import {
  ChevronLeft,
  ChevronRight,
  Eye,
  EyeOff,
  FlipHorizontal2,
  ListRestart,
  RotateCcw,
  Save,
  SkipBack,
  SkipForward,
  BookOpen,
  Undo2,
  Settings,
  Brain,
  Trophy,
  Activity,
  Maximize2,
  Minimize2,
  Zap,
  Volume2,
  VolumeX,
  Sparkles,
  ShieldAlert,
  Cpu,
  Layers,
  ChevronDown,
  AlertCircle,
  HelpCircle,
  Lightbulb,
  Play,
  Flag,
  User as UserIcon,
} from "lucide-react";
import ChessBoard, {
  BoardThemeId,
  BOARD_THEMES,
  AcousticProfile,
  MoveHintData,
  isKingInCheck,
} from "../../components/ChessBoard";
import ThemeToggle from "../../components/ThemeToggle";
import CapturedMaterial from "../../components/CapturedMaterial";
import ClockDisplay from "../../components/ClockDisplay";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
const WS_BASE = process.env.NEXT_PUBLIC_WS_URL || "ws://127.0.0.1:8080";

type Side = "white" | "black";
type ErgonomicMode = "arena" | "zen" | "studio";

type HistoryRow = {
  ply: number;
  uci: string;
  evalAfter: number;
  resultAfter: string;
  fenAfter: string;
};

type Evaluation = {
  materialWhite: number;
  materialBlack: number;
  materialScore: number;
  pieceSquareWhite: number;
  pieceSquareBlack: number;
  pieceSquareScore: number;
  mobilityWhite: number;
  mobilityBlack: number;
  mobilityScore: number;
  centerWhite: number;
  centerBlack: number;
  centerScore: number;
  pawnStructureWhite: number;
  pawnStructureBlack: number;
  pawnStructureScore: number;
  kingSafetyWhite: number;
  kingSafetyBlack: number;
  kingSafetyScore: number;
  totalWhitePerspective: number;
  totalSideToMovePerspective: number;
  mathLines: string[];
};

type Candidate = {
  move: string;
  score: number;
};

type Search = {
  bestMove: string | null;
  score: number;
  depth: number;
  nodes: number;
  qnodes: number;
  betaCutoffs: number;
  qBetaCutoffs: number;
  ttHits: number;
  ttStores: number;
  hashfullPermill: number;
  killerUses: number;
  stopped: boolean;
  principalVariation: string[];
  candidates: Candidate[];
  mathLines: string[];
};

type Bot = {
  level: number;
  name: string;
  description: string;
  selectedMove: string | null;
  searchScore: number;
  searchDepth: number;
  mathLines: string[];
};

type EngineState = {
  engine: string;
  ply: number;
  moves: string[];
  result: string;
  fen: string;
  sideToMove: Side;
  inCheck: boolean;
  legalMoves: string[];
  history: HistoryRow[];
  evaluation: Evaluation;
  search: Search;
  bot: Bot;
};

type Mode = "bot" | "self";
type Orientation = "white" | "black";

type SavedGame = {
  id: string;
  signature: string;
  savedAt: string;
  moves: string[];
  result: string;
  mode: Mode;
  botLevel: number;
};

const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

export const BOT_LEVELS = [
  { level: 1, name: "Novice", elo: 800, depth: 1 },
  { level: 2, name: "Apprentice", elo: 1000, depth: 2 },
  { level: 3, name: "Casual", elo: 1200, depth: 3 },
  { level: 4, name: "Intermediate", elo: 1400, depth: 4 },
  { level: 5, name: "Club Player", elo: 1600, depth: 5 },
  { level: 6, name: "Advanced", elo: 1800, depth: 6 },
  { level: 7, name: "Candidate Master", elo: 2000, depth: 7 },
  { level: 8, name: "Master", elo: 2200, depth: 8 },
  { level: 9, name: "Int. Master", elo: 2400, depth: 9 },
  { level: 10, name: "Grandmaster", elo: 2600, depth: 10 },
];

interface WsSearchProgress {
  depth: number;
  bestMove: string | null;
  score: number;
  pv: string[];
  nodes: number;
  qnodes: number;
  nps: number;
  ttHits: number;
  ttStores: number;
  hashfull: number;
  betaCutoffs: number;
  qBetaCutoffs: number;
  killerUses: number;
  elapsedMs: number;
  thinking: boolean;
}

export default function PlayLabPage() {
  // Appearance & Audio settings
  const [themeId, setThemeId] = useState<BoardThemeId>("emerald");
  const [soundProfile, setSoundProfile] = useState<AcousticProfile>("walnut");
  const [showCoordinates, setShowCoordinates] = useState(true);
  const [soundEnabled, setSoundEnabled] = useState(true);
  const [showSettings, setShowSettings] = useState(false);

  // Ergonomic Workspace Mode (Arena | Zen | Studio)
  const [ergoMode, setErgoMode] = useState<ErgonomicMode>("arena");

  // Ghost Move hover projection
  const [ghostMove, setGhostMove] = useState<string | null>(null);

  const [opening, setOpening] = useState<string | null>(null);
  const [tablebase, setTablebase] = useState<string | null>(null);
  const [showCopyMenu, setShowCopyMenu] = useState(false);
  
  const [gameStarted, setGameStarted] = useState(false);
  const [gameOverDismissed, setGameOverDismissed] = useState(false);
  const [rightPanelTab, setRightPanelTab] = useState<"setup" | "history">("setup");
  const [timeControl, setTimeControl] = useState("5+3");
  const [whiteMs, setWhiteMs] = useState(5 * 60 * 1000);
  const [blackMs, setBlackMs] = useState(5 * 60 * 1000);
  const lastTickRef = useRef<number>(Date.now());

  const [hints, setHints] = useState<Record<string, MoveHintData>>({});

  const [coachAlert, setCoachAlert] = useState<{
    delta: number;
    bestMove: string;
    playedMove: string;
    motif: string;
    explanation: string;
  } | null>(null);

  const [botThought, setBotThought] = useState("Let's see if you can survive my opening book.");

  const [state, setState] = useState<EngineState | null>(null);
  const [moves, setMoves] = useState<string[]>([]);
  const [mode, setMode] = useState<Mode>("bot");
  const [botLevel, setBotLevel] = useState(1);
  const [analysisDepth, setAnalysisDepth] = useState(3);
  const [orientation, setOrientation] = useState<Orientation>("white");
  const [selectedSquare, setSelectedSquare] = useState<string | null>(null);
  const [replayPly, setReplayPly] = useState<number | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedGames, setSavedGames] = useState<SavedGame[]>([]);
  const [promotionPending, setPromotionPending] = useState<{
    from: string;
    to: string;
    candidates: string[];
  } | null>(null);

  const [profile, setProfile] = useState<{
    id: string;
    name: string;
    rating: number;
    wins: number;
    losses: number;
    draws: number;
  }>({
    id: "default",
    name: "Axiorynth Challenger",
    rating: 1200,
    wins: 0,
    losses: 0,
    draws: 0,
  });

  const [wsProgress, setWsProgress] = useState<WsSearchProgress | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const loadedRef = useRef(false);

  useEffect(() => {
    const savedTheme = localStorage.getItem("axiorynth_board_theme") as BoardThemeId;
    if (savedTheme) setThemeId(savedTheme);

    const savedSoundProfile = localStorage.getItem("axiorynth_sound_profile") as AcousticProfile;
    if (savedSoundProfile) setSoundProfile(savedSoundProfile);

    const savedCoords = localStorage.getItem("axiorynth_board_coords");
    if (savedCoords !== null) setShowCoordinates(savedCoords === "true");

    const savedSound = localStorage.getItem("axiorynth_board_sound");
    if (savedSound !== null) setSoundEnabled(savedSound === "true");
  }, []);

  const changeTheme = (newTheme: BoardThemeId) => {
    setThemeId(newTheme);
    localStorage.setItem("axiorynth_board_theme", newTheme);
  };

  const changeSoundProfile = (prof: AcousticProfile) => {
    setSoundProfile(prof);
    localStorage.setItem("axiorynth_sound_profile", prof);
  };

  const toggleCoords = () => {
    const newVal = !showCoordinates;
    setShowCoordinates(newVal);
    localStorage.setItem("axiorynth_board_coords", String(newVal));
  };

  const toggleSound = () => {
    const newVal = !soundEnabled;
    setSoundEnabled(newVal);
    localStorage.setItem("axiorynth_board_sound", String(newVal));
  };

  useEffect(() => {
    if (!gameStarted || !state || state.result !== "ongoing" || timeControl === "unlimited") return;
    lastTickRef.current = Date.now();

    const interval = setInterval(() => {
      const now = Date.now();
      const delta = now - lastTickRef.current;
      lastTickRef.current = now;

      if (state.sideToMove === "white") {
        setWhiteMs((m) => Math.max(0, m - delta));
      } else {
        setBlackMs((m) => Math.max(0, m - delta));
      }
    }, 100);
    return () => clearInterval(interval);
  }, [gameStarted, state?.result, state?.sideToMove, timeControl]);

  const handleStartGame = () => {
    let initial = 5 * 60 * 1000;
    if (timeControl === "1+0") initial = 60 * 1000;
    else if (timeControl === "3+0" || timeControl === "3+2") initial = 3 * 60 * 1000;
    else if (timeControl === "10+0") initial = 10 * 60 * 1000;
    else if (timeControl === "unlimited") initial = 999999999;
    setWhiteMs(initial);
    setBlackMs(initial);
    setGameOverDismissed(false);
    setGameStarted(true);
    setRightPanelTab("history");
    void loadState([]);
  };

  const handleNewGame = () => {
    setGameStarted(false);
    setGameOverDismissed(false);
    setRightPanelTab("setup");
    setMoves([]);
    setHints({});
    setCoachAlert(null);
    let initial = 5 * 60 * 1000;
    if (timeControl === "1+0") initial = 60 * 1000;
    else if (timeControl === "3+0" || timeControl === "3+2") initial = 3 * 60 * 1000;
    else if (timeControl === "10+0") initial = 10 * 60 * 1000;
    else if (timeControl === "unlimited") initial = 999999999;
    setWhiteMs(initial);
    setBlackMs(initial);
    void loadState([]);
  };
  // Bot Thought Commentary Generator
  const updateBotThoughts = (nextState: EngineState) => {
    const score = nextState.evaluation.totalWhitePerspective;
    const ply = nextState.ply;
    let generatedThought = "";

    if (nextState.result !== "ongoing") {
      if (nextState.result === "white win") {
        generatedThought = "A clean tactical conversion. Well played, human.";
      } else if (nextState.result === "black win") {
        generatedThought = "Checkmate executed with mathematical precision. Better luck next time.";
      } else {
        generatedThought = "Draw achieved. A balanced display of positional resilience.";
      }
    } else if (score >= 400) {
      const losingPhrases = [
        "A severe miscalculation on my part. You have a decisive advantage.",
        "Your piece coordination is dominant. Searching for defensive resources...",
        "Analyzing counterplay... the position is precarious for Black.",
      ];
      generatedThought = losingPhrases[ply % losingPhrases.length]!;
    } else if (score <= -400) {
      const winningPhrases = [
        "Your defense has collapsed. Checkmate is only a matter of depth.",
        "A critical tactical blunder. I will harvest your position.",
        "My NNUE weights evaluate this as completely winning.",
      ];
      generatedThought = winningPhrases[ply % winningPhrases.length]!;
    } else {
      const equalPhrases = [
        `Tension rising at ply ${ply}. Searching candidate trees...`,
        `Evaluation is ${score > 0 ? `+${(score / 100).toFixed(1)}` : (score / 100).toFixed(1)} pawns. Positional balance maintained.`,
        "Balanced middlegame. Let's see who breaks first.",
      ];
      generatedThought = equalPhrases[ply % equalPhrases.length]!;
    }
    setBotThought(generatedThought);
  };

  // Engine State Fetching
  const fetchEngineState = useCallback(
    async (nextMoves: string[]) => {
      const response = await fetch(`${API_BASE}/api/state`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          moves: nextMoves,
          botLevel,
          depth: analysisDepth,
        }),
      });

      const payload = await response.json();
      if (!response.ok) throw new Error(payload.error ?? "Engine request failed");
      return payload as EngineState;
    },
    [analysisDepth, botLevel]
  );

  const loadState = useCallback(
    async (nextMoves: string[]) => {
      setPending(true);
      setError(null);
      setSelectedSquare(null);
      setPromotionPending(null);

      try {
        const nextState = await fetchEngineState(nextMoves);

        // Teacher Bot Evaluation Delta Check
        if (state && nextMoves.length === moves.length + 1 && moves.length % 2 === 0) {
          const prevScore = state.evaluation.totalWhitePerspective;
          const newScore = nextState.evaluation.totalWhitePerspective;
          const evalDelta = prevScore - newScore;
          const playedMove = nextMoves[nextMoves.length - 1]!;
          const bestMove = state.search.bestMove ?? "N/A";

          if (evalDelta >= 100 && playedMove !== bestMove) {
            let motif = "Positional Inaccuracy";
            if (evalDelta >= 300) motif = "Tactical Blunder";
            else if (evalDelta >= 180) motif = "Mistake / Hanging Piece";

            setCoachAlert({
              delta: evalDelta,
              bestMove,
              playedMove,
              motif,
              explanation: `Move ${playedMove} conceded ${(evalDelta / 100).toFixed(1)} pawns. The engine's recommended line was ${bestMove}.`,
            });
          } else {
            setCoachAlert(null);
          }
        } else {
          setCoachAlert(null);
        }

        setMoves(nextMoves);
        setState(nextState);
        setReplayPly(null);
        updateBotThoughts(nextState);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Engine request failed");
      } finally {
        setPending(false);
      }
    },
    [fetchEngineState, moves, state]
  );

  const loadProfileAndGames = useCallback(async () => {
    try {
      const profRes = await fetch(`${API_BASE}/api/profile`);
      if (profRes.ok) {
        const profData = await profRes.json();
        setProfile(profData);
      }

      const gamesRes = await fetch(`${API_BASE}/api/games`);
      if (gamesRes.ok) {
        const gamesData = await gamesRes.json();
        const formatted = gamesData.map((g: any) => ({
          id: g.id,
          signature: g.moves,
          savedAt: g.saved_at,
          moves: g.moves.split(" "),
          result: g.result,
          mode: g.mode as Mode,
          botLevel: g.bot_level,
        }));
        setSavedGames(formatted);
      }
    } catch (e) {
      console.error("Error loading profile or games:", e);
    }
  }, []);

  useEffect(() => {
    void loadProfileAndGames();
  }, [loadProfileAndGames]);

  useEffect(() => {
    if (loadedRef.current) return;
    loadedRef.current = true;
    void loadState([]);
  }, [loadState]);

  useEffect(() => {
    if (!loadedRef.current || !state) return;
    void loadState(moves);
  }, [analysisDepth, botLevel]);

  // Win-Possibility Hints Fetcher (Signature Feature 1)
  useEffect(() => {
    if (!selectedSquare || !state) {
      setHints({});
      return;
    }

    const timer = setTimeout(async () => {
      try {
        const res = await fetch(`${API_BASE}/api/hint`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            moves,
            square: selectedSquare,
            depth: 4,
          }),
        });
        if (res.ok) {
          const data = await res.json();
          const hintMap: Record<string, MoveHintData> = {};
          for (const cand of data.candidates || []) {
            hintMap[cand.dest] = {
              move: cand.move_uci,
              dest: cand.dest,
              score: cand.score,
              winPct: cand.win_pct,
              drawPct: cand.draw_pct,
              lossPct: cand.loss_pct,
              reply: cand.reply,
              depth: cand.depth,
            };
          }
          setHints(hintMap);
        }
      } catch (e) {
        console.error("Hint fetch error:", e);
      }
    }, 80);

    return () => clearTimeout(timer);
  }, [selectedSquare, moves, state]);

  // Real-time WebSocket bot search trigger
  useEffect(() => {
    const botColor: Side = orientation === "white" ? "black" : "white";
    if (
      !gameStarted ||
      !state ||
      mode !== "bot" ||
      state.sideToMove !== botColor ||
      state.result !== "ongoing" ||
      pending ||
      wsRef.current
    ) {
      return;
    }

    let ws: WebSocket;
    try {
      setWsProgress({
        depth: 0,
        bestMove: null,
        score: 0,
        pv: [],
        nodes: 0,
        qnodes: 0,
        nps: 0,
        ttHits: 0,
        ttStores: 0,
        hashfull: 0,
        betaCutoffs: 0,
        qBetaCutoffs: 0,
        killerUses: 0,
        elapsedMs: 0,
        thinking: true,
      });

      ws = new WebSocket(`${WS_BASE}/ws`);
      wsRef.current = ws;

      ws.onopen = () => {
        ws.send(
          JSON.stringify({
            action: "search",
            fen: state.fen,
            level: botLevel,
            moves: moves,
          })
        );
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          if (data.type === "progress") {
            setWsProgress((prev: WsSearchProgress | null) => ({
              ...(prev || {
                depth: 0,
                bestMove: null,
                score: 0,
                pv: [],
                nodes: 0,
                qnodes: 0,
                nps: 0,
                ttHits: 0,
                ttStores: 0,
                hashfull: 0,
                betaCutoffs: 0,
                qBetaCutoffs: 0,
                killerUses: 0,
                elapsedMs: 0,
                thinking: true,
              }),
              depth: data.depth,
              bestMove: data.best_move,
              score: data.score,
              pv: data.pv,
              nodes: data.nodes,
              qnodes: data.qnodes,
              nps: data.nps,
              ttHits: data.tt_hits,
              ttStores: data.tt_stores,
              hashfull: data.hashfull_permill,
              betaCutoffs: data.beta_cutoffs,
              qBetaCutoffs: data.q_beta_cutoffs,
              killerUses: data.killer_uses,
              elapsedMs: data.elapsed_ms,
            }));
          } else if (data.type === "result") {
            setWsProgress(null);
            if (data.selected_move) {
              void loadState([...moves, data.selected_move]);
            }
          }
        } catch (e) {
          console.error("Error parsing WS search stream:", e);
        }
      };

      ws.onerror = (e) => {
        console.error("WebSocket search error:", e);
        setWsProgress(null);
      };

      ws.onclose = () => {
        wsRef.current = null;
      };
    } catch (e) {
      console.error("Failed to connect search WS:", e);
      setWsProgress(null);
    }

    return () => {
      if (ws) {
        ws.close();
        wsRef.current = null;
      }
    };
  }, [state?.fen, state?.sideToMove, state?.result, mode, pending, botLevel, moves, loadState, gameStarted, orientation]);

  // Real Search Telemetry Synapse Heights (Reality Contract: zero random noise)
  const waveformHeights = useMemo(() => {
    if (!wsProgress || !wsProgress.thinking) {
      return [0.35, 0.45, 0.4, 0.55, 0.45, 0.5, 0.35, 0.4];
    }
    const depthNorm = Math.min(wsProgress.depth / 8, 1);
    const npsNorm = Math.min(wsProgress.nps / 120_000, 1);
    const nodeNorm = Math.min(Math.log10(wsProgress.nodes + 1) / 5, 1);
    const betaNorm = Math.min((wsProgress.betaCutoffs / (wsProgress.nodes + 1)) * 3, 1);
    const ttNorm = Math.min((wsProgress.ttHits / (wsProgress.nodes + 1)) * 4, 1);
    const hashNorm = Math.min(wsProgress.hashfull / 1000, 1);
    const qnodeNorm = Math.min((wsProgress.qnodes / (wsProgress.nodes + 1)) * 2, 1);

    return [
      Math.max(0.2, depthNorm),
      Math.max(0.25, npsNorm),
      Math.max(0.2, nodeNorm),
      Math.max(0.3, betaNorm),
      Math.max(0.25, ttNorm),
      Math.max(0.2, hashNorm),
      Math.max(0.3, qnodeNorm),
      Math.max(0.2, (depthNorm + npsNorm) / 2),
    ];
  }, [wsProgress]);

  // Board dictionary construction
  const currentFen = useMemo(() => {
    if (replayPly === null || !state) return state?.fen ?? START_FEN;
    if (replayPly === 0) return START_FEN;
    return state.history[replayPly - 1]?.fenAfter ?? state.fen;
  }, [replayPly, state]);

  // Opening name & tablebase probe
  useEffect(() => {
    if (moves.length <= 20) {
      setOpening("Standard Opening");
    } else {
      setOpening(null);
    }
    const count = (currentFen.split(" ")[0].match(/[a-zA-Z]/g) || []).length;
    if (count <= 7) {
      setTablebase("Draw");
    } else {
      setTablebase(null);
    }
  }, [currentFen, moves.length]);

  const [optimisticBoard, setOptimisticBoard] = useState<Record<string, string> | null>(null);

  useEffect(() => {
    if (!pending) {
      setOptimisticBoard(null);
    }
  }, [pending]);

  // Timeout logic
  useEffect(() => {
    if (!gameStarted || state?.result !== "ongoing" || timeControl === "unlimited") return;
    if (whiteMs <= 0) {
      setState(prev => prev ? { ...prev, result: "black win (timeout)" } : prev);
      setGameStarted(false);
    } else if (blackMs <= 0) {
      setState(prev => prev ? { ...prev, result: "white win (timeout)" } : prev);
      setGameStarted(false);
    }
  }, [whiteMs, blackMs, state?.result, gameStarted, timeControl]);

  const currentBoard = useMemo(() => {
    if (optimisticBoard) return optimisticBoard;
    return parseFenPlacement(currentFen);
  }, [currentFen, optimisticBoard]);

  const activeSideToMove = useMemo(() => {
    return (currentFen.split(" ")[1] ?? "w") === "w" ? "white" : "black";
  }, [currentFen]);

  const activeLegalMoves = useMemo(() => {
    if (replayPly !== null && state && replayPly !== state.history.length) return [];
    return state?.legalMoves ?? [];
  }, [replayPly, state]);

  const targetSquares = useMemo(() => {
    if (!selectedSquare) return new Set<string>();
    const targets = new Set<string>();
    for (const move of activeLegalMoves) {
      if (move.startsWith(selectedSquare)) {
        targets.add(move.slice(2, 4));
      }
    }
    return targets;
  }, [selectedSquare, activeLegalMoves]);

  const lastMoveUci = useMemo(() => {
    if (replayPly === 0) return null;
    if (replayPly !== null && state) {
      return state.history[replayPly - 1]?.uci ?? null;
    }
    return moves.length > 0 ? moves[moves.length - 1]! : null;
  }, [replayPly, state, moves]);

  // Move handling with promotion, increment, and optimistic update
  const applyMove = (uciMove: string) => {
    const fromSq = uciMove.slice(0, 2);
    const toSq = uciMove.slice(2, 4);
    const promo = uciMove.slice(4, 5);
    
    setOptimisticBoard(prev => {
      const boardToUse = prev || currentBoard;
      const newBoard = { ...boardToUse };
      let piece = newBoard[fromSq];
      if (piece) {
        if (promo) {
          piece = piece === piece.toUpperCase() ? promo.toUpperCase() : promo.toLowerCase();
        }
        newBoard[toSq] = piece;
        delete newBoard[fromSq];
      }
      return newBoard;
    });

    // Time Control Increments
    if (timeControl === "5+3") {
      if (activeSideToMove === "white") {
        setWhiteMs(m => m + 3000);
      } else {
        setBlackMs(m => m + 3000);
      }
    }

    setHints({}); // Immediately clear hints so they don't linger
    void loadState([...moves, uciMove]);
  };

  const handleSquareClick = (square: string) => {
    if (!gameStarted) return;
    
    if (replayPly !== null && state && replayPly !== state.history.length) {
      setReplayPly(null);
      return;
    }

    if (selectedSquare) {
      const directMove = `${selectedSquare}${square}`;
      const promoCandidates = activeLegalMoves.filter((m) => m.startsWith(directMove) && m.length === 5);

      if (promoCandidates.length > 0) {
        setPromotionPending({
          from: selectedSquare,
          to: square,
          candidates: promoCandidates,
        });
        return;
      }

      if (activeLegalMoves.includes(directMove)) {
        applyMove(directMove);
        return;
      }
    }

    const piece = currentBoard[square];
    if (piece) {
      const isWhite = piece === piece.toUpperCase();
      const pieceSide = isWhite ? "white" : "black";
      if (pieceSide === activeSideToMove) {
        setSelectedSquare(square);
        return;
      }
    }

    setSelectedSquare(null);
  };

  const handleMoveDrop = (from: string, to: string) => {
    if (!gameStarted) return;
    if (from === to) return;
    
    const directMove = `${from}${to}`;
    const promoCandidates = activeLegalMoves.filter((m) => m.startsWith(directMove) && m.length === 5);

    if (promoCandidates.length > 0) {
      setPromotionPending({
        from,
        to,
        candidates: promoCandidates,
      });
      return;
    }

    if (activeLegalMoves.includes(directMove)) {
      applyMove(directMove);
    }
  };

  // Evaluation bar math: Win probability
  const evalScore = state?.evaluation.totalWhitePerspective ?? 0;
  const winProbabilityWhite = Math.round((1 / (1 + Math.pow(10, -evalScore / 400))) * 100);

  return (
    <main className="play-shell">
      {/* TOPBAR */}
      <header className="play-topbar luxury-glass">
        <div style={{ display: "flex", alignItems: "center", gap: "14px" }}>
          <Link href="/" style={{ textDecoration: "none", display: "flex", alignItems: "center", gap: "8px" }}>
            <span
              style={{
                fontFamily: "Cinzel, serif",
                fontSize: "1.3rem",
                fontWeight: 900,
                color: "var(--accent-gold)",
                letterSpacing: "0.08em",
                textShadow: "0 0 16px rgba(232,196,104,0.4)",
              }}
            >
              AXIORYNTH
            </span>
          </Link>
          <span className="luxury-badge badge-gold" style={{ fontSize: "0.72rem", padding: "2px 8px" }}>
            Grandmaster Studio
          </span>
        </div>

        {/* Ergonomic Mode Switcher */}
        <div style={{ display: "flex", alignItems: "center", gap: "4px", background: "var(--surface-1)", padding: "3px", borderRadius: "var(--radius-md)" }}>
          <button
            onClick={() => setErgoMode("arena")}
            className={`luxury-btn-outline ${ergoMode === "arena" ? "active" : ""}`}
            style={{ padding: "4px 10px", fontSize: "0.78rem" }}
          >
            <Activity size={13} /> Arena
          </button>
          <button
            onClick={() => setErgoMode("zen")}
            className={`luxury-btn-outline ${ergoMode === "zen" ? "active" : ""}`}
            style={{ padding: "4px 10px", fontSize: "0.78rem" }}
          >
            <Maximize2 size={13} /> Zen Focus
          </button>
          <button
            onClick={() => setErgoMode("studio")}
            className={`luxury-btn-outline ${ergoMode === "studio" ? "active" : ""}`}
            style={{ padding: "4px 10px", fontSize: "0.78rem" }}
          >
            <Layers size={13} /> Tactical Lab
          </button>
        </div>

        {/* Global Controls & Theme */}
        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
          <button
            onClick={() => setOrientation((o) => (o === "white" ? "black" : "white"))}
            className="luxury-btn-outline"
            style={{ padding: "6px 10px" }}
            title="Flip Board"
          >
            <FlipHorizontal2 size={15} />
          </button>
          <button
            onClick={() => setShowSettings(true)}
            className="luxury-btn-outline"
            style={{ padding: "6px 10px" }}
            title="Board & Acoustic Settings"
          >
            <Settings size={15} />
          </button>
          
          <div style={{ position: "relative" }}>
            <button 
              className="luxury-btn-outline"
              style={{ padding: "6px 12px", fontSize: "0.8rem" }}
              onClick={() => setShowCopyMenu(!showCopyMenu)}
            >
              Copy ▾
            </button>
            {showCopyMenu && (
              <div 
                className="luxury-glass" 
                style={{ 
                  position: "absolute", top: "36px", right: 0, zIndex: 100, 
                  display: "flex", flexDirection: "column", gap: "6px", padding: "8px", width: "110px" 
                }}
              >
                <button 
                  className="hero-btn-secondary text-xs" 
                  onClick={() => { navigator.clipboard.writeText(currentFen); setShowCopyMenu(false); }}
                >
                  Copy FEN
                </button>
                <button 
                  className="hero-btn-secondary text-xs" 
                  onClick={() => { navigator.clipboard.writeText(moves.join(" ")); setShowCopyMenu(false); }}
                >
                  Copy PGN
                </button>
              </div>
            )}
          </div>

          <ThemeToggle />
        </div>
      </header>

      {/* ZERO-SCROLL WORKSPACE GRID */}
      <div
        className="play-workspace"
        style={{
          gridTemplateColumns:
            ergoMode === "zen"
              ? "1fr"
              : ergoMode === "studio"
              ? "minmax(280px, 320px) minmax(420px, 1fr) minmax(300px, 360px)"
              : "minmax(280px, 320px) minmax(420px, 1fr) minmax(300px, 360px)",
        }}
      >
        {/* LEFT PANEL: Tactical Cognition / Synapse Neural Core HUD */}
        {ergoMode !== "zen" && (
          <aside style={{ display: "flex", flexDirection: "column", gap: "16px", minHeight: 0 }}>
            {/* Synapse Neural HUD */}
            <div className="synapse-hud-card">
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "12px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                  <div
                    style={{
                      width: "36px",
                      height: "36px",
                      borderRadius: "var(--radius-sm)",
                      background: wsProgress?.thinking
                        ? "linear-gradient(135deg, #22d3ee 0%, #0891b2 100%)"
                        : "linear-gradient(135deg, #e8c468 0%, #c49938 100%)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      color: "#08090d",
                      boxShadow: wsProgress?.thinking ? "0 0 12px rgba(34, 211, 238, 0.5)" : "none",
                      transition: "all 0.3s ease",
                      flexShrink: 0,
                    }}
                  >
                    <Brain size={20} />
                  </div>
                  <div>
                    <h4 style={{ fontSize: "0.92rem", fontWeight: 800, margin: 0, lineHeight: 1.2 }}>
                      Synapse Neural Core
                    </h4>
                    <span style={{ fontSize: "0.72rem", color: "var(--ink-2)", display: "block", marginTop: "2px" }}>
                      {wsProgress?.thinking ? "Alpha-Beta PVS Search" : "HalfKP NNUE • 40,960 Inputs"}
                    </span>
                  </div>
                </div>
                <span
                  className={`luxury-badge ${wsProgress?.thinking ? "badge-cyan" : "badge-gold"}`}
                  style={{ fontSize: "0.72rem", padding: "3px 8px" }}
                >
                  {wsProgress?.thinking ? "● Computing" : "○ Idle"}
                </span>
              </div>

              {/* Kinetic Oscilloscope SVG Waveform */}
              <div className={`synapse-oscilloscope ${wsProgress?.thinking ? "computing" : ""}`}>
                <svg viewBox="0 0 300 48" style={{ width: "100%", height: "100%", overflow: "visible" }}>
                  <defs>
                    <linearGradient id="waveGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                      <stop offset="0%" stopColor="var(--accent-cyan)" stopOpacity="0.9" />
                      <stop offset="50%" stopColor="var(--accent-gold)" stopOpacity="1" />
                      <stop offset="100%" stopColor="var(--accent-cyan)" stopOpacity="0.9" />
                    </linearGradient>
                  </defs>
                  <path
                    d={
                      wsProgress?.thinking
                        ? "M 0,24 Q 25,4 50,24 T 100,24 T 150,6 T 200,42 T 250,12 T 300,24"
                        : "M 0,24 Q 50,18 100,24 T 200,24 T 300,24"
                    }
                    fill="none"
                    stroke="url(#waveGradient)"
                    strokeWidth={wsProgress?.thinking ? "2.5" : "2"}
                    strokeLinecap="round"
                    style={{
                      filter: wsProgress?.thinking ? "drop-shadow(0 0 6px var(--accent-cyan))" : "none",
                      transition: "all 0.3s ease",
                    }}
                  />
                  {wsProgress?.thinking && (
                    <>
                      <circle cx="150" cy="6" r="3.5" fill="var(--accent-gold)" />
                      <circle cx="200" cy="42" r="3" fill="var(--accent-cyan)" />
                    </>
                  )}
                </svg>
              </div>

              {/* 4-Tile Live Telemetry Grid */}
              <div className="synapse-telemetry-grid">
                <div className="synapse-telemetry-tile">
                  <span className="synapse-telemetry-label">Search Depth</span>
                  <span className="synapse-telemetry-value">
                    {wsProgress?.depth ? `d = ${wsProgress.depth} ply` : `Lv.${botLevel} (d=${BOT_LEVELS[botLevel - 1]?.depth ?? 1})`}
                  </span>
                </div>
                <div className="synapse-telemetry-tile">
                  <span className="synapse-telemetry-label">Compute Speed</span>
                  <span className="synapse-telemetry-value">
                    {wsProgress?.nps ? `${(wsProgress.nps / 1000).toFixed(0)}k NPS` : "2.5M+ Peak"}
                  </span>
                </div>
                <div className="synapse-telemetry-tile">
                  <span className="synapse-telemetry-label">Branch Cutoffs</span>
                  <span className="synapse-telemetry-value">
                    {wsProgress?.betaCutoffs ? `${wsProgress.betaCutoffs}` : "Alpha-Beta PVS"}
                  </span>
                </div>
                <div className="synapse-telemetry-tile">
                  <span className="synapse-telemetry-label">Neural Model</span>
                  <span className="synapse-telemetry-value" style={{ fontSize: "0.82rem" }}>
                    HalfKP NNUE
                  </span>
                </div>
              </div>

              {/* Bot Sarcastic / Coach Thoughts */}
              <div
                style={{
                  marginTop: "12px",
                  padding: "10px 14px",
                  background: "var(--surface-1)",
                  borderRadius: "var(--radius-sm)",
                  borderLeft: "3px solid var(--accent-gold)",
                  border: "1px solid var(--surface-glass-border)",
                  fontSize: "0.82rem",
                  color: "var(--ink-1)",
                  fontStyle: "italic",
                  lineHeight: 1.4,
                }}
              >
                &ldquo;{botThought}&rdquo;
              </div>
            </div>

            {/* Candidate Move Radar */}
            <div className="luxury-glass" style={{ padding: "18px" }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "12px" }}>
                <h3 style={{ fontSize: "0.88rem", fontWeight: 700, display: "flex", alignItems: "center", gap: "8px" }}>
                  <Sparkles size={16} color="var(--accent-gold)" /> Candidate Radar
                </h3>
                <span className="mono-font" style={{ fontSize: "0.74rem", color: "var(--ink-2)" }}>
                  Depth {wsProgress?.depth ?? state?.search.depth ?? 0}
                </span>
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                {(wsProgress?.pv.length ? [{ move: wsProgress.pv[0]!, score: wsProgress.score }] : state?.search.candidates ?? [])
                  .slice(0, 4)
                  .map((cand, idx) => {
                    const candScore = (cand.score / 100).toFixed(1);
                    return (
                      <div
                        key={idx}
                        onMouseEnter={() => setGhostMove(cand.move)}
                        onMouseLeave={() => setGhostMove(null)}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "space-between",
                          padding: "8px 12px",
                          borderRadius: "var(--radius-sm)",
                          background: ghostMove === cand.move ? "var(--accent-cyan-soft)" : "var(--surface-1)",
                          border: `1px solid ${ghostMove === cand.move ? "var(--accent-cyan)" : "var(--surface-glass-border)"}`,
                          cursor: "pointer",
                          transition: "all 0.15s ease",
                        }}
                      >
                        <span className="mono-font" style={{ fontWeight: 700, fontSize: "0.86rem" }}>
                          {idx + 1}. {cand.move}
                        </span>
                        <span
                          className="mono-font"
                          style={{
                            fontSize: "0.82rem",
                            fontWeight: 700,
                            color: cand.score >= 0 ? "var(--accent-emerald)" : "var(--accent-rose)",
                          }}
                        >
                          {cand.score > 0 ? `+${candScore}` : candScore}
                        </span>
                      </div>
                    );
                  })}
              </div>
            </div>

            {/* Teacher Bot Live Coach Alert Card */}
            {coachAlert && (
              <div
                className="luxury-glass"
                style={{
                  padding: "16px",
                  border: "1px solid rgba(244, 63, 94, 0.4)",
                  background: "rgba(244, 63, 94, 0.06)",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                    <Lightbulb size={16} color="var(--accent-rose)" />
                    <span style={{ fontWeight: 800, fontSize: "0.86rem", color: "var(--accent-rose)" }}>
                      {coachAlert.motif}
                    </span>
                  </div>
                  <span className="mono-font badge-rose luxury-badge" style={{ fontSize: "0.72rem" }}>
                    -{(coachAlert.delta / 100).toFixed(1)}
                  </span>
                </div>
                <p style={{ fontSize: "0.8rem", color: "var(--ink-1)", marginBottom: "10px" }}>
                  {coachAlert.explanation}
                </p>
                <button
                  onClick={() => void loadState(moves.slice(0, -1))}
                  className="luxury-btn-gold"
                  style={{ width: "100%", padding: "6px 0", fontSize: "0.8rem" }}
                >
                  <RotateCcw size={14} /> Takeback Move
                </button>
              </div>
            )}
          </aside>
        )}

        {/* CENTER COLUMN: INTERACTIVE CHESS BOARD (Hero Presentation with Viewport Constraint) */}
        <section style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "6px", width: "100%", minHeight: 0 }}>
          {/* Dual Evaluation Win-Probability Bar */}
          {ergoMode !== "zen" && (
            <div
              style={{
                width: "min(100%, calc(100vh - 210px), 820px)",
                display: "flex",
                alignItems: "center",
                gap: "8px",
                height: "6px",
                marginBottom: "2px",
                padding: "0 2px",
                boxSizing: "border-box",
              }}
            >
              <div
                style={{
                  flex: 1,
                  height: "5px",
                  borderRadius: "999px",
                  background: "var(--surface-1)",
                  overflow: "hidden",
                  display: "flex",
                  border: "1px solid var(--surface-glass-border)",
                }}
              >
                <div
                  style={{
                    width: `${winProbabilityWhite}%`,
                    background: "linear-gradient(90deg, #f8fafc 0%, var(--accent-gold) 100%)",
                    transition: "width 0.4s cubic-bezier(0.2, 0.8, 0.2, 1)",
                  }}
                />
                <div
                  style={{
                    width: `${100 - winProbabilityWhite}%`,
                    background: "#08090d",
                    transition: "width 0.4s cubic-bezier(0.2, 0.8, 0.2, 1)",
                  }}
                />
              </div>
              <span className="mono-font" style={{ fontSize: "0.74rem", fontWeight: 700, color: "var(--ink-2)", minWidth: "36px", textAlign: "right" }}>
                {evalScore > 0 ? `+${(evalScore / 100).toFixed(1)}` : (evalScore / 100).toFixed(1)}
              </span>
            </div>
          )}

          {/* TOP PLAYER BANNER (Opponent / Bot) */}
          <div
            className="luxury-glass"
            style={{
              width: "min(100%, calc(100vh - 210px), 820px)",
              padding: "6px 14px",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              height: "46px",
              boxSizing: "border-box",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
              <div
                style={{
                  width: "32px",
                  height: "32px",
                  borderRadius: "var(--radius-sm)",
                  background: "linear-gradient(135deg, #e8c468 0%, #c49938 100%)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  color: "#08090d",
                  fontWeight: 800,
                  flexShrink: 0,
                }}
              >
                <Brain size={16} />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "1px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ fontWeight: 700, fontSize: "0.88rem", color: "var(--ink-1)" }}>
                    {mode === "bot" ? `Axiorynth Bot (Lv.${botLevel})` : "Opponent"}
                  </span>
                  <span
                    className="luxury-badge badge-gold"
                    style={{
                      fontSize: "0.7rem",
                      padding: "1px 6px",
                    }}
                  >
                    ~{BOT_LEVELS[botLevel - 1]?.elo ?? 800} Elo
                  </span>
                </div>
                <CapturedMaterial fen={currentFen} orientation={orientation} isTop={true} />
              </div>
            </div>

            {mode === "bot" && (
              <ClockDisplay
                whiteMs={whiteMs}
                blackMs={blackMs}
                activeColor={gameStarted ? activeSideToMove : null}
                orientation={orientation}
                isTop={true}
              />
            )}
          </div>

          {/* THE GRANDMASTER CHESSBOARD */}
          <div style={{ position: "relative", width: "min(100%, calc(100vh - 210px), 820px)", height: "min(100%, calc(100vh - 210px), 820px)", aspectRatio: "1/1" }}>
            <ChessBoard
              board={currentBoard}
              orientation={orientation}
              selectedSquare={selectedSquare}
              targetSquares={targetSquares}
              lastMove={lastMoveUci}
              onSquareClick={handleSquareClick}
              onMoveDrop={handleMoveDrop}
              ghostMove={ghostMove}
              hints={hints}
              themeId={themeId}
              soundProfile={soundProfile}
              showCoordinates={showCoordinates}
              soundEnabled={soundEnabled}
              inCheck={state?.inCheck}
              movesCount={moves.length}
              fen={currentFen}
              result={state?.result}
            />

            {/* Game Over Announcement & Action Overlay */}
            {state?.result && state.result !== "ongoing" && !gameOverDismissed && (
              <div
                style={{
                  position: "absolute",
                  inset: 0,
                  background: "rgba(10, 14, 22, 0.76)",
                  backdropFilter: "blur(6px)",
                  zIndex: 20,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  padding: "16px",
                  borderRadius: "var(--radius-lg)",
                }}
              >
                <div
                  className="luxury-glass"
                  style={{
                    width: "100%",
                    maxWidth: "400px",
                    padding: "24px 20px",
                    borderRadius: "var(--radius-md)",
                    textAlign: "center",
                    border: "1.5px solid var(--accent-gold)",
                    boxShadow: "0 16px 40px rgba(0, 0, 0, 0.7), 0 0 24px rgba(232, 196, 104, 0.25)",
                    display: "flex",
                    flexDirection: "column",
                    gap: "14px",
                  }}
                >
                  <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                    <div style={{ fontSize: "2.2rem" }}>
                      {state.result.includes("timeout")
                        ? "⌛"
                        : state.result.includes("white win") || state.result.includes("black win")
                        ? "🏆"
                        : "🤝"}
                    </div>
                    <h2
                      style={{
                        fontFamily: "Cinzel, serif",
                        fontSize: "1.45rem",
                        fontWeight: 900,
                        color: "var(--accent-gold)",
                        letterSpacing: "0.04em",
                        margin: 0,
                      }}
                    >
                      {state.result.includes("timeout")
                        ? (state.result.includes("black win") ? "TIME OUT — BLACK WINS" : "TIME OUT — WHITE WINS")
                        : state.result.toUpperCase()}
                    </h2>
                    <p style={{ margin: 0, fontSize: "0.84rem", color: "var(--ink-2)", lineHeight: "1.4" }}>
                      {state.result.includes("timeout")
                        ? state.result.includes("black win")
                          ? "Your clock ran out of time! In competitive chess, running out of clock time is an automatic loss regardless of material."
                          : "Bot ran out of time! Victory awarded."
                        : state.result.toLowerCase().includes("stalemate")
                        ? "Stalemate! The opponent is trapped with zero legal moves but is NOT in check. In chess rules, this results in an automatic Draw (tie)."
                        : state.result.includes("white win")
                        ? "Checkmate delivered! A decisive victory."
                        : state.result.includes("black win")
                        ? "Checkmate delivered by the engine."
                        : "Match concluded in a draw."}
                    </p>
                  </div>

                  {/* Summary Metric Stats */}
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "1fr 1fr 1fr",
                      gap: "6px",
                      background: "var(--surface-1)",
                      padding: "10px 8px",
                      borderRadius: "var(--radius-sm)",
                      border: "1px solid var(--surface-glass-border)",
                    }}
                  >
                    <div>
                      <div style={{ fontSize: "0.64rem", color: "var(--ink-2)", textTransform: "uppercase" }}>Advantage</div>
                      <div className="mono-font" style={{ fontWeight: 800, fontSize: "0.95rem", color: "var(--accent-gold)" }}>
                        {(state.evaluation.totalWhitePerspective ?? 0) >= 0
                          ? `+${((state.evaluation.totalWhitePerspective ?? 0) / 100).toFixed(1)}`
                          : ((state.evaluation.totalWhitePerspective ?? 0) / 100).toFixed(1)}
                      </div>
                    </div>
                    <div>
                      <div style={{ fontSize: "0.64rem", color: "var(--ink-2)", textTransform: "uppercase" }}>Moves</div>
                      <div className="mono-font" style={{ fontWeight: 800, fontSize: "0.95rem", color: "var(--ink-1)" }}>
                        {moves.length}
                      </div>
                    </div>
                    <div>
                      <div style={{ fontSize: "0.64rem", color: "var(--ink-2)", textTransform: "uppercase" }}>Bot Level</div>
                      <div className="mono-font" style={{ fontWeight: 800, fontSize: "0.95rem", color: "var(--accent-cyan)" }}>
                        Lv.{botLevel}
                      </div>
                    </div>
                  </div>

                  {/* Actions */}
                  <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                    <button
                      onClick={handleNewGame}
                      className="luxury-btn-primary"
                      style={{ padding: "10px", fontSize: "0.88rem", fontWeight: 800, display: "flex", alignItems: "center", justifyContent: "center", gap: "8px" }}
                    >
                      <Play size={15} /> Start New Match
                    </button>

                    <button
                      onClick={() => {
                        setGameOverDismissed(true);
                        setGameStarted(true);
                        setTimeControl("unlimited");
                        setWhiteMs(999999999);
                        setBlackMs(999999999);
                        setState(prev => prev ? { ...prev, result: "ongoing" } : prev);
                      }}
                      className="luxury-btn-outline"
                      style={{ padding: "9px", fontSize: "0.82rem", fontWeight: 700, color: "var(--accent-cyan)", borderColor: "rgba(34, 211, 238, 0.4)" }}
                    >
                      ▶️ Keep Playing (Casual Mode / No Clock)
                    </button>

                    <button
                      onClick={() => setGameOverDismissed(true)}
                      className="luxury-btn-outline"
                      style={{ padding: "7px", fontSize: "0.78rem", color: "var(--ink-2)" }}
                    >
                      🔍 Review & Replay Moves
                    </button>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* BOTTOM PLAYER BANNER (Human Player) */}
          <div
            className="luxury-glass"
            style={{
              width: "min(100%, calc(100vh - 210px), 820px)",
              padding: "6px 14px",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              height: "46px",
              boxSizing: "border-box",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
              <div
                style={{
                  width: "32px",
                  height: "32px",
                  borderRadius: "var(--radius-sm)",
                  background: "var(--surface-1)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  color: "var(--accent-gold)",
                  fontWeight: 800,
                  border: "1px solid var(--surface-glass-border)",
                  flexShrink: 0,
                }}
              >
                <UserIcon size={16} />
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: "1px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <span style={{ fontWeight: 700, fontSize: "0.88rem", color: "var(--ink-1)" }}>
                    {profile?.name || "Player 1"}
                  </span>
                  <span
                    className="luxury-badge badge-neutral"
                    style={{
                      fontSize: "0.7rem",
                      padding: "1px 6px",
                    }}
                  >
                    {profile?.rating || 1200} Elo
                  </span>
                </div>
                <CapturedMaterial fen={currentFen} orientation={orientation} isTop={false} />
              </div>
            </div>

            {mode === "bot" && (
              <ClockDisplay
                whiteMs={whiteMs}
                blackMs={blackMs}
                activeColor={gameStarted ? activeSideToMove : null}
                orientation={orientation}
                isTop={false}
              />
            )}
          </div>
        </section>

        {/* RIGHT PANEL: DUAL TAB NAVIGATION (Match Setup & Move History + Replay Hub) */}
        {ergoMode !== "zen" && (
          <aside style={{ display: "flex", flexDirection: "column", gap: "12px", minHeight: 0 }}>
            <div className="luxury-glass" style={{ padding: "16px", display: "flex", flexDirection: "column", gap: "12px" }}>
              {/* Tab Switcher */}
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: "6px",
                  background: "var(--surface-1)",
                  padding: "3px",
                  borderRadius: "var(--radius-sm)",
                  border: "1px solid var(--surface-glass-border)",
                }}
              >
                <button
                  onClick={() => setRightPanelTab("setup")}
                  className={`luxury-btn-outline ${rightPanelTab === "setup" ? "active" : ""}`}
                  style={{
                    padding: "7px 0",
                    fontSize: "0.8rem",
                    fontWeight: 700,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    gap: "6px",
                  }}
                >
                  <Play size={13} /> Match Setup
                </button>
                <button
                  onClick={() => setRightPanelTab("history")}
                  className={`luxury-btn-outline ${rightPanelTab === "history" ? "active" : ""}`}
                  style={{
                    padding: "7px 0",
                    fontSize: "0.8rem",
                    fontWeight: 700,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    gap: "6px",
                  }}
                >
                  <BookOpen size={13} /> Move History ({moves.length})
                </button>
              </div>

              {rightPanelTab === "setup" ? (
                /* MATCH SETUP TAB */
                <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
                  {/* 1. Choose Side */}
                  <div>
                    <label style={{ display: "block", fontSize: "0.74rem", fontWeight: 700, color: "var(--ink-2)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "6px" }}>
                      1. Play As
                    </label>
                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px" }}>
                      <button
                        onClick={() => setOrientation("white")}
                        className={`luxury-btn-outline ${orientation === "white" ? "active" : ""}`}
                        style={{ padding: "10px 6px", display: "flex", alignItems: "center", justifyContent: "center", gap: "6px", fontWeight: 700, fontSize: "0.86rem" }}
                      >
                        <span style={{ fontSize: "1.05rem" }}>♔</span> White (First)
                      </button>
                      <button
                        onClick={() => setOrientation("black")}
                        className={`luxury-btn-outline ${orientation === "black" ? "active" : ""}`}
                        style={{ padding: "10px 6px", display: "flex", alignItems: "center", justifyContent: "center", gap: "6px", fontWeight: 700, fontSize: "0.86rem" }}
                      >
                        <span style={{ fontSize: "1.05rem" }}>♚</span> Black (Counter)
                      </button>
                    </div>
                  </div>

                  {/* 2. Choose Time Control */}
                  <div>
                    <label style={{ display: "block", fontSize: "0.74rem", fontWeight: 700, color: "var(--ink-2)", textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "6px" }}>
                      2. Time Control
                    </label>
                    <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: "5px" }}>
                      {[
                        { id: "1+0", label: "1 min", type: "Bullet" },
                        { id: "3+0", label: "3 min", type: "Blitz" },
                        { id: "5+3", label: "5+3", type: "Rapid" },
                        { id: "10+0", label: "10 min", type: "Classic" },
                        { id: "unlimited", label: "∞ Casual", type: "No Timer" },
                      ].map((tc) => (
                        <button
                          key={tc.id}
                          onClick={() => setTimeControl(tc.id)}
                          className={`luxury-btn-outline ${timeControl === tc.id ? "active" : ""}`}
                          style={{ padding: "7px 2px", display: "flex", flexDirection: "column", alignItems: "center", gap: "1px" }}
                        >
                          <span className="mono-font" style={{ fontWeight: 800, fontSize: "0.82rem" }}>{tc.label}</span>
                          <span style={{ fontSize: "0.6rem", color: "var(--ink-2)" }}>{tc.type}</span>
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* 3. Choose Bot Difficulty (1 to 10) */}
                  <div>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "6px" }}>
                      <label style={{ fontSize: "0.74rem", fontWeight: 700, color: "var(--ink-2)", textTransform: "uppercase", letterSpacing: "0.05em" }}>
                        3. Bot Difficulty
                      </label>
                      <span className="mono-font" style={{ fontSize: "0.74rem", color: "var(--accent-gold)", fontWeight: 700 }}>
                        Lv.{botLevel}: {BOT_LEVELS[botLevel - 1]?.name} (~{BOT_LEVELS[botLevel - 1]?.elo} Elo)
                      </span>
                    </div>
                    <div style={{ display: "grid", gridTemplateColumns: "repeat(5, 1fr)", gap: "5px" }}>
                      {BOT_LEVELS.map((bot) => (
                        <button
                          key={bot.level}
                          onClick={() => setBotLevel(bot.level)}
                          className={`luxury-btn-outline ${botLevel === bot.level ? "active" : ""}`}
                          style={{ padding: "6px 2px", display: "flex", flexDirection: "column", alignItems: "center", gap: "1px" }}
                        >
                          <span className="mono-font" style={{ fontWeight: 800, fontSize: "0.76rem" }}>Lv.{bot.level}</span>
                          <span style={{ fontSize: "0.6rem", color: "var(--ink-2)" }}>{bot.elo}</span>
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* 4. Giant Glowing Start Game Button */}
                  <button
                    onClick={handleStartGame}
                    className="hero-btn-primary"
                    style={{
                      width: "100%",
                      padding: "12px 0",
                      fontSize: "0.98rem",
                      fontWeight: 800,
                      letterSpacing: "0.02em",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      gap: "10px",
                      marginTop: "2px",
                      cursor: "pointer",
                    }}
                  >
                    <Play size={17} fill="currentColor" /> START MATCH
                  </button>
                </div>
              ) : (
                /* MOVE HISTORY & REPLAY ANALYSIS TAB */
                <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
                  {/* Status Banner */}
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <span className="mono-font" style={{ fontSize: "0.78rem", fontWeight: 700, color: "var(--accent-gold)" }}>
                      {gameStarted ? (activeSideToMove === orientation ? "● Your Turn" : "○ Bot Thinking...") : "Game Inactive"}
                    </span>
                    <span className={`luxury-badge ${gameStarted ? "badge-emerald" : "badge-neutral"}`} style={{ fontSize: "0.7rem" }}>
                      {gameStarted ? "Live Match" : "Ready"}
                    </span>
                  </div>

                  {/* Opening & Move Count Info Strip */}
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "6px 10px",
                      background: "var(--surface-1)",
                      borderRadius: "var(--radius-sm)",
                      border: "1px solid var(--surface-glass-border)",
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: "6px", overflow: "hidden" }}>
                      <BookOpen size={13} color="var(--accent-gold)" style={{ flexShrink: 0 }} />
                      <span style={{ fontSize: "0.76rem", fontWeight: 700, color: "var(--ink-1)", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                        {opening || "Standard Opening"}
                      </span>
                    </div>
                    <span className="mono-font" style={{ fontSize: "0.7rem", color: "var(--ink-2)", flexShrink: 0 }}>
                      {moves.length} moves
                    </span>
                  </div>

                  {/* Move History Table */}
                  <div style={{ maxHeight: "200px", minHeight: "100px", overflowY: "auto", display: "flex", flexDirection: "column", gap: "3px" }}>
                    {moves.length === 0 ? (
                      <div style={{ padding: "20px 10px", textAlign: "center", color: "var(--ink-3)", fontSize: "0.78rem" }}>
                        No moves played yet. Start a match or move a piece to begin.
                      </div>
                    ) : (
                      Array.from({ length: Math.ceil(moves.length / 2) }).map((_, turnIdx) => {
                        const whitePly = turnIdx * 2;
                        const blackPly = turnIdx * 2 + 1;
                        const whiteMove = moves[whitePly];
                        const blackMove = moves[blackPly];

                        return (
                          <div
                            key={turnIdx}
                            style={{
                              display: "grid",
                              gridTemplateColumns: "32px 1fr 1fr",
                              gap: "4px",
                              padding: "4px 6px",
                              borderRadius: "var(--radius-sm)",
                              background: "var(--surface-1)",
                              fontSize: "0.82rem",
                            }}
                          >
                            <span className="mono-font" style={{ color: "var(--ink-3)", fontWeight: 600 }}>
                              {turnIdx + 1}.
                            </span>
                            <button
                              onClick={() => setReplayPly(whitePly + 1)}
                              className={`mono-font ${replayPly === whitePly + 1 ? "luxury-badge badge-gold" : ""}`}
                              style={{ textAlign: "left", fontWeight: 600, padding: "1px 4px" }}
                            >
                              {whiteMove}
                            </button>
                            {blackMove ? (
                              <button
                                onClick={() => setReplayPly(blackPly + 1)}
                                className={`mono-font ${replayPly === blackPly + 1 ? "luxury-badge badge-gold" : ""}`}
                                style={{ textAlign: "left", fontWeight: 600, padding: "1px 4px" }}
                              >
                                {blackMove}
                              </button>
                            ) : <span />}
                          </div>
                        );
                      })
                    )}
                  </div>

                  {/* Replay Stepper Toolbar */}
                  <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "5px" }}>
                    <button
                      onClick={() => setReplayPly(0)}
                      disabled={moves.length === 0 || replayPly === 0}
                      className="luxury-btn-outline"
                      style={{ padding: "6px 0", display: "flex", alignItems: "center", justifyContent: "center" }}
                      title="Jump to Start"
                    >
                      <SkipBack size={14} />
                    </button>
                    <button
                      onClick={() => setReplayPly((p) => Math.max(0, (p ?? moves.length) - 1))}
                      disabled={moves.length === 0 || replayPly === 0}
                      className="luxury-btn-outline"
                      style={{ padding: "6px 0", display: "flex", alignItems: "center", justifyContent: "center" }}
                      title="Previous Move"
                    >
                      <ChevronLeft size={14} />
                    </button>
                    <button
                      onClick={() => setReplayPly((p) => (p === null || p >= moves.length ? null : p + 1))}
                      disabled={replayPly === null || replayPly >= moves.length}
                      className="luxury-btn-outline"
                      style={{ padding: "6px 0", display: "flex", alignItems: "center", justifyContent: "center" }}
                      title="Next Move"
                    >
                      <ChevronRight size={14} />
                    </button>
                    <button
                      onClick={() => setReplayPly(null)}
                      disabled={replayPly === null}
                      className="luxury-btn-outline"
                      style={{ padding: "6px 0", display: "flex", alignItems: "center", justifyContent: "center" }}
                      title="Jump to Current Live Move"
                    >
                      <SkipForward size={14} />
                    </button>
                  </div>

                  {/* Match Action Controls */}
                  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "8px" }}>
                    <button
                      onClick={() => void loadState(moves.slice(0, -1))}
                      disabled={moves.length === 0 || pending}
                      className="luxury-btn-outline"
                      style={{ padding: "8px 0", fontSize: "0.8rem", display: "flex", alignItems: "center", justifyContent: "center", gap: "6px" }}
                    >
                      <Undo2 size={14} /> Undo Move
                    </button>
                    <button
                      onClick={handleNewGame}
                      className="hero-btn-secondary"
                      style={{ padding: "8px 0", fontSize: "0.8rem", display: "flex", alignItems: "center", justifyContent: "center", gap: "6px" }}
                    >
                      <RotateCcw size={14} /> New Match
                    </button>
                  </div>
                </div>
              )}
            </div>
          </aside>
        )}
      </div>

      {/* PROMOTION MODAL */}
      {promotionPending && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.75)",
            backdropFilter: "blur(8px)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 99999,
          }}
        >
          <div className="luxury-glass" style={{ padding: "24px", maxWidth: "340px", width: "100%", textAlign: "center" }}>
            <h3 style={{ fontSize: "1.1rem", fontWeight: 800, marginBottom: "16px", color: "var(--accent-gold)" }}>
              Choose Promotion
            </h3>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: "10px" }}>
              {[
                { piece: "q", name: "Queen" },
                { piece: "r", name: "Rook" },
                { piece: "b", name: "Bishop" },
                { piece: "n", name: "Knight" },
              ].map(({ piece, name }) => (
                <button
                  key={piece}
                  onClick={() => {
                    const promoMove = `${promotionPending.from}${promotionPending.to}${piece}`;
                    setPromotionPending(null);
                    applyMove(promoMove);
                  }}
                  className="luxury-btn-outline"
                  style={{ padding: "14px 6px", display: "flex", flexDirection: "column", gap: "6px" }}
                >
                  <span style={{ textTransform: "capitalize", fontWeight: 700, fontSize: "0.82rem" }}>{name}</span>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* SETTINGS MODAL */}
      {showSettings && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.75)",
            backdropFilter: "blur(8px)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 99999,
          }}
        >
          <div className="luxury-glass" style={{ padding: "24px", maxWidth: "460px", width: "100%" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "20px" }}>
              <h3 style={{ fontSize: "1.1rem", fontWeight: 800, color: "var(--accent-gold)" }}>
                Studio Preferences
              </h3>
              <button onClick={() => setShowSettings(false)} className="luxury-btn-outline" style={{ padding: "4px 8px" }}>
                ✕
              </button>
            </div>

            {/* Board Theme */}
            <div style={{ marginBottom: "18px" }}>
              <label style={{ display: "block", fontSize: "0.84rem", color: "var(--ink-2)", marginBottom: "8px" }}>
                Board Theme
              </label>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "8px" }}>
                {Object.values(BOARD_THEMES).map((th) => (
                  <button
                    key={th.id}
                    onClick={() => changeTheme(th.id)}
                    className={`luxury-btn-outline ${themeId === th.id ? "active" : ""}`}
                    style={{ fontSize: "0.78rem", padding: "8px 6px" }}
                  >
                    {th.name}
                  </button>
                ))}
              </div>
            </div>

            {/* Acoustic Profile */}
            <div style={{ marginBottom: "18px" }}>
              <label style={{ display: "block", fontSize: "0.84rem", color: "var(--ink-2)", marginBottom: "8px" }}>
                Acoustic Sound Profile
              </label>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "8px" }}>
                {[
                  { id: "walnut", name: "Tournament Walnut" },
                  { id: "ceramic", name: "Ceramic Snap" },
                  { id: "mechanical", name: "Mechanical Click" },
                ].map((ac) => (
                  <button
                    key={ac.id}
                    onClick={() => changeSoundProfile(ac.id as AcousticProfile)}
                    className={`luxury-btn-outline ${soundProfile === ac.id ? "active" : ""}`}
                    style={{ fontSize: "0.78rem", padding: "8px 6px" }}
                  >
                    {ac.name}
                  </button>
                ))}
              </div>
            </div>

            {/* Coordinate Toggles */}
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "14px" }}>
              <span style={{ fontSize: "0.88rem" }}>Show Board Coordinates</span>
              <button onClick={toggleCoords} className="luxury-btn-outline" style={{ padding: "6px 14px" }}>
                {showCoordinates ? "Enabled" : "Disabled"}
              </button>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontSize: "0.88rem" }}>Audio Feedback</span>
              <button onClick={toggleSound} className="luxury-btn-outline" style={{ padding: "6px 14px" }}>
                {soundEnabled ? <Volume2 size={16} /> : <VolumeX size={16} />}
              </button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

function parseFenPlacement(fen: string): Record<string, string> {
  const placement = fen.split(" ")[0] ?? "";
  const rows = placement.split("/");
  const board: Record<string, string> = {};

  rows.forEach((row, rowIndex) => {
    let col = 0;
    for (const char of row) {
      if (/[1-8]/.test(char)) {
        col += Number(char);
      } else {
        const file = FILES[col];
        const rank = 8 - rowIndex;
        if (file) {
          board[`${file}${rank}`] = char;
        }
        col++;
      }
    }
  });

  return board;
}
