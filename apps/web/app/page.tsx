"use client";

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
  Undo2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type Side = "white" | "black";

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

const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
const STORAGE_KEY = "axiorynth.savedGames.v1";

const PIECES: Record<string, string> = {
  K: "♔",
  Q: "♕",
  R: "♖",
  B: "♗",
  N: "♘",
  P: "♙",
  k: "♚",
  q: "♛",
  r: "♜",
  b: "♝",
  n: "♞",
  p: "♟",
};

export default function AxiorynthApp() {
  const [state, setState] = useState<EngineState | null>(null);
  const [moves, setMoves] = useState<string[]>([]);
  const [mode, setMode] = useState<Mode>("bot");
  const [botLevel, setBotLevel] = useState(3);
  const [analysisDepth, setAnalysisDepth] = useState(2);
  const [showMath, setShowMath] = useState(true);
  const [orientation, setOrientation] = useState<Orientation>("white");
  const [selectedSquare, setSelectedSquare] = useState<string | null>(null);
  const [replayPly, setReplayPly] = useState<number | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [savedGames, setSavedGames] = useState<SavedGame[]>([]);
  const [archivedSignature, setArchivedSignature] = useState("");
  const loadedRef = useRef(false);

  const fetchEngineState = useCallback(
    async (nextMoves: string[]) => {
      const response = await fetch("/api/state", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          moves: nextMoves,
          botLevel,
          depth: analysisDepth,
        }),
      });

      const payload = await response.json();
      if (!response.ok) {
        throw new Error(payload.error ?? "Engine request failed");
      }
      return payload as EngineState;
    },
    [analysisDepth, botLevel],
  );

  const loadState = useCallback(
    async (nextMoves: string[], options: { withBot?: boolean } = {}) => {
      setPending(true);
      setError(null);
      setSelectedSquare(null);

      try {
        const firstState = await fetchEngineState(nextMoves);
        if (
          options.withBot &&
          mode === "bot" &&
          firstState.result === "ongoing" &&
          firstState.sideToMove === "black" &&
          firstState.bot.selectedMove
        ) {
          const botMoves = [...nextMoves, firstState.bot.selectedMove];
          const secondState = await fetchEngineState(botMoves);
          setMoves(botMoves);
          setState(secondState);
        } else {
          setMoves(nextMoves);
          setState(firstState);
        }
        setReplayPly(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Engine request failed");
      } finally {
        setPending(false);
      }
    },
    [fetchEngineState, mode],
  );

  useEffect(() => {
    if (loadedRef.current) {
      return;
    }
    loadedRef.current = true;
    void loadState([]);
  }, [loadState]);

  useEffect(() => {
    if (!loadedRef.current || !state) {
      return;
    }
    void loadState(moves);
  }, [analysisDepth, botLevel]);

  useEffect(() => {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return;
    }

    try {
      const parsed = JSON.parse(raw) as SavedGame[];
      if (Array.isArray(parsed)) {
        setSavedGames(parsed);
      }
    } catch {
      setSavedGames([]);
    }
  }, []);

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(savedGames));
  }, [savedGames]);

  const archiveCurrentGame = useCallback(() => {
    if (!state || moves.length === 0) {
      return;
    }

    const signature = moves.join(" ");
    if (archivedSignature === signature || savedGames.some((game) => game.signature === signature)) {
      return;
    }

    const savedGame: SavedGame = {
      id: `${Date.now()}-${signature.replaceAll(" ", "-")}`,
      signature,
      savedAt: new Date().toISOString(),
      moves,
      result: state.result,
      mode,
      botLevel,
    };

    setSavedGames((current) => [savedGame, ...current].slice(0, 30));
    setArchivedSignature(signature);
  }, [archivedSignature, botLevel, mode, moves, savedGames, state]);

  useEffect(() => {
    if (state?.result && state.result !== "ongoing") {
      archiveCurrentGame();
    }
  }, [archiveCurrentGame, state?.result]);

  const activeFen = useMemo(() => {
    if (!state) {
      return START_FEN;
    }
    if (replayPly === null) {
      return state.fen;
    }
    if (replayPly <= 0) {
      return START_FEN;
    }
    return state.history[replayPly - 1]?.fenAfter ?? state.fen;
  }, [replayPly, state]);

  const board = useMemo(() => parseFen(activeFen), [activeFen]);
  const squareOrder = useMemo(() => buildSquareOrder(orientation), [orientation]);
  const live = replayPly === null;
  const targetMoves = useMemo(() => {
    if (!state || !selectedSquare || !live) {
      return [];
    }
    return state.legalMoves.filter((move) => move.startsWith(selectedSquare));
  }, [live, selectedSquare, state]);
  const targetSquares = useMemo(() => new Set(targetMoves.map((move) => move.slice(2, 4))), [targetMoves]);
  const replayValue = replayPly ?? moves.length;
  const movePairs = useMemo(() => pairMoves(moves), [moves]);
  const savedStats = useMemo(() => buildSavedStats(savedGames), [savedGames]);
  const evalPercent = useMemo(() => {
    const score = state?.evaluation.totalWhitePerspective ?? 0;
    return Math.max(5, Math.min(95, 50 + score / 12));
  }, [state]);

  async function playMove(move: string) {
    const nextMoves = [...moves, move];
    await loadState(nextMoves, { withBot: mode === "bot" });
  }

  function handleSquareClick(square: string) {
    if (!state || !live || pending) {
      return;
    }

    if (mode === "bot" && state.sideToMove === "black") {
      return;
    }

    if (selectedSquare) {
      const move =
        targetMoves.find((candidate) => candidate.slice(2, 4) === square && candidate.endsWith("q")) ??
        targetMoves.find((candidate) => candidate.slice(2, 4) === square);
      if (move) {
        void playMove(move);
        return;
      }
    }

    const piece = board[square];
    if (piece && pieceSide(piece) === state.sideToMove) {
      setSelectedSquare(square === selectedSquare ? null : square);
    } else {
      setSelectedSquare(null);
    }
  }

  function startNewGame() {
    archiveCurrentGame();
    setArchivedSignature("");
    void loadState([]);
  }

  function undoMove() {
    const undoCount = mode === "bot" ? Math.min(2, moves.length) : 1;
    const nextMoves = moves.slice(0, Math.max(0, moves.length - undoCount));
    setArchivedSignature("");
    void loadState(nextMoves);
  }

  function loadSavedGame(game: SavedGame) {
    setMode(game.mode);
    setBotLevel(game.botLevel);
    setArchivedSignature(game.signature);
    void loadState(game.moves);
  }

  function setReplay(value: number) {
    setSelectedSquare(null);
    setReplayPly(value === moves.length ? null : value);
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">Axiorynth</p>
          <h1>Engine Play Lab</h1>
        </div>
        <div className="status-strip" aria-live="polite">
          <span className={`status-dot ${pending ? "thinking" : ""}`} />
          <span>{pending ? "Calculating" : state?.result ?? "loading"}</span>
          <span>{state ? `${state.sideToMove} to move` : "engine booting"}</span>
        </div>
      </header>

      <section className="workspace">
        <div className="board-column">
          <div className="toolbar" aria-label="Game controls">
            <div className="segmented" aria-label="Mode">
              <button className={mode === "bot" ? "active" : ""} onClick={() => setMode("bot")} type="button">
                Bot
              </button>
              <button className={mode === "self" ? "active" : ""} onClick={() => setMode("self")} type="button">
                Self
              </button>
            </div>

            <button className="tool-button" onClick={startNewGame} title="New game" type="button">
              <ListRestart size={18} />
              New
            </button>
            <button className="tool-button" disabled={moves.length === 0 || pending} onClick={undoMove} title="Undo" type="button">
              <Undo2 size={18} />
              Undo
            </button>
            <button
              className="icon-button"
              onClick={() => setOrientation((value) => (value === "white" ? "black" : "white"))}
              title="Flip board"
              type="button"
            >
              <FlipHorizontal2 size={18} />
            </button>
            <button
              className="icon-button"
              onClick={() => setShowMath((value) => !value)}
              title={showMath ? "Hide math" : "Show math"}
              type="button"
            >
              {showMath ? <Eye size={18} /> : <EyeOff size={18} />}
            </button>
            <button className="icon-button" disabled={moves.length === 0} onClick={archiveCurrentGame} title="Save game" type="button">
              <Save size={18} />
            </button>
          </div>

          <div className="board-frame">
            <div className="board" aria-label="Chessboard">
              {squareOrder.map((square) => {
                const piece = board[square];
                const isDark = isDarkSquare(square);
                const selected = selectedSquare === square;
                const target = targetSquares.has(square);
                const fromMove = state?.search.bestMove?.startsWith(square);
                const toMove = state?.search.bestMove?.slice(2, 4) === square;

                return (
                  <button
                    className={[
                      "square",
                      isDark ? "dark" : "light",
                      selected ? "selected" : "",
                      target ? "target" : "",
                      fromMove || toMove ? "best" : "",
                    ].join(" ")}
                    key={square}
                    onClick={() => handleSquareClick(square)}
                    type="button"
                  >
                    <span className="square-name">{square}</span>
                    {piece ? <span className={`piece ${pieceSide(piece)}`}>{PIECES[piece]}</span> : null}
                  </button>
                );
              })}
            </div>
          </div>

          <div className="replay-bar">
            <button className="icon-button" disabled={moves.length === 0} onClick={() => setReplay(0)} title="Start" type="button">
              <SkipBack size={18} />
            </button>
            <button className="icon-button" disabled={replayValue === 0} onClick={() => setReplay(Math.max(0, replayValue - 1))} title="Back" type="button">
              <ChevronLeft size={18} />
            </button>
            <input
              aria-label="Replay ply"
              max={moves.length}
              min={0}
              onChange={(event) => setReplay(Number(event.target.value))}
              type="range"
              value={replayValue}
            />
            <button
              className="icon-button"
              disabled={replayValue === moves.length}
              onClick={() => setReplay(Math.min(moves.length, replayValue + 1))}
              title="Forward"
              type="button"
            >
              <ChevronRight size={18} />
            </button>
            <button className="tool-button" onClick={() => setReplay(moves.length)} title="Live board" type="button">
              <RotateCcw size={18} />
              Live
            </button>
          </div>

          {error ? <div className="error-banner">{error}</div> : null}
        </div>

        <aside className="side-panel">
          <section className="panel">
            <div className="panel-head">
              <div>
                <p className="eyebrow">Position</p>
                <h2>{state?.inCheck ? "Check on board" : "Current state"}</h2>
              </div>
              <span className="pill">{state?.ply ?? 0} ply</span>
            </div>

            <div className="score-band" aria-label="Evaluation">
              <span>Black</span>
              <div className="score-track">
                <div style={{ width: `${evalPercent}%` }} />
              </div>
              <span>White</span>
            </div>

            <div className="metric-grid">
              <Metric label="Eval" value={formatScore(state?.evaluation.totalWhitePerspective ?? 0)} />
              <Metric label="Best" value={state?.search.bestMove ?? "-"} />
              <Metric label="Nodes" value={compactNumber((state?.search.nodes ?? 0) + (state?.search.qnodes ?? 0))} />
              <Metric label="PV" value={state?.search.principalVariation.join(" ") || "-"} />
            </div>

            <div className="selectors">
              <label>
                <span>Bot level</span>
                <input min={1} max={10} onChange={(event) => setBotLevel(Number(event.target.value))} type="range" value={botLevel} />
                <strong>{botLevel}</strong>
              </label>
              <label>
                <span>Depth</span>
                <input min={1} max={4} onChange={(event) => setAnalysisDepth(Number(event.target.value))} type="range" value={analysisDepth} />
                <strong>{analysisDepth}</strong>
              </label>
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <div>
                <p className="eyebrow">Moves</p>
                <h2>Game record</h2>
              </div>
              <span className="pill">{movePairs.length} turns</span>
            </div>
            <div className="move-list">
              {movePairs.length === 0 ? <p className="empty-text">No moves yet.</p> : null}
              {movePairs.map((pair) => (
                <div className="move-row" key={pair.turn}>
                  <span>{pair.turn}.</span>
                  <button onClick={() => setReplay(pair.whitePly)} type="button">
                    {pair.white}
                  </button>
                  {pair.black ? (
                    <button onClick={() => setReplay(pair.blackPly)} type="button">
                      {pair.black}
                    </button>
                  ) : (
                    <span />
                  )}
                </div>
              ))}
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <div>
                <p className="eyebrow">Bot</p>
                <h2>{state?.bot.name ?? "Loading"}</h2>
              </div>
              <span className="pill">{state?.bot.selectedMove ?? "none"}</span>
            </div>
            <p className="panel-copy">{state?.bot.description ?? "Engine profile loading."}</p>
            <div className="candidate-list">
              {state?.search.candidates.map((candidate) => (
                <div key={candidate.move}>
                  <span>{candidate.move}</span>
                  <strong>{formatScore(candidate.score)}</strong>
                </div>
              ))}
            </div>
          </section>
        </aside>

        <aside className="side-panel right-panel">
          <section className="panel">
            <div className="panel-head">
              <div>
                <p className="eyebrow">Possibilities</p>
                <h2>Legal moves</h2>
              </div>
              <span className="pill">{state?.legalMoves.length ?? 0}</span>
            </div>
            <div className="legal-grid">
              {state?.legalMoves.map((move) => (
                <button key={move} onClick={() => live && void playMove(move)} type="button">
                  {move}
                </button>
              ))}
            </div>
          </section>

          {showMath ? (
            <section className="panel math-panel">
              <div className="panel-head">
                <div>
                  <p className="eyebrow">Numbers</p>
                  <h2>Actual math</h2>
                </div>
                <span className="pill">depth {state?.search.depth ?? analysisDepth}</span>
              </div>

              <div className="math-block">
                <h3>Evaluation</h3>
                {state?.evaluation.mathLines.map((line) => (
                  <code key={line}>{line}</code>
                ))}
              </div>

              <div className="math-block">
                <h3>Search</h3>
                {state?.search.mathLines.map((line) => (
                  <code key={line}>{line}</code>
                ))}
              </div>
            </section>
          ) : null}

          <section className="panel">
            <div className="panel-head">
              <div>
                <p className="eyebrow">Archive</p>
                <h2>Saved games</h2>
              </div>
              <span className="pill">{savedGames.length}</span>
            </div>
            <div className="archive-stats">
              <Metric label="White" value={String(savedStats.whiteWins)} />
              <Metric label="Black" value={String(savedStats.blackWins)} />
              <Metric label="Draws" value={String(savedStats.draws)} />
            </div>
            <div className="saved-list">
              {savedGames.length === 0 ? <p className="empty-text">No saved games yet.</p> : null}
              {savedGames.map((game) => (
                <button key={game.id} onClick={() => loadSavedGame(game)} type="button">
                  <span>{new Date(game.savedAt).toLocaleString()}</span>
                  <strong>{game.result}</strong>
                  <small>{game.moves.join(" ")}</small>
                </button>
              ))}
            </div>
          </section>
        </aside>
      </section>
    </main>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function parseFen(fen: string) {
  const placement = fen.split(" ")[0] ?? "";
  const rows = placement.split("/");
  const board: Record<string, string> = {};

  rows.forEach((row, rankIndex) => {
    const rank = 8 - rankIndex;
    let fileIndex = 0;
    for (const char of row) {
      const emptyCount = Number(char);
      if (Number.isInteger(emptyCount) && emptyCount > 0) {
        fileIndex += emptyCount;
      } else {
        board[`${FILES[fileIndex]}${rank}`] = char;
        fileIndex += 1;
      }
    }
  });

  return board;
}

function buildSquareOrder(orientation: Orientation) {
  return Array.from({ length: 64 }, (_, index) => {
    const rankOffset = Math.floor(index / 8);
    const fileOffset = index % 8;
    const rank = orientation === "white" ? 8 - rankOffset : 1 + rankOffset;
    const file = orientation === "white" ? FILES[fileOffset] : FILES[7 - fileOffset];
    return `${file}${rank}`;
  });
}

function isDarkSquare(square: string) {
  const file = FILES.indexOf(square[0]);
  const rank = Number(square[1]) - 1;
  return (file + rank) % 2 === 1;
}

function pieceSide(piece: string): Side {
  return piece === piece.toUpperCase() ? "white" : "black";
}

function formatScore(score: number) {
  return `${score >= 0 ? "+" : ""}${score}`;
}

function compactNumber(value: number) {
  return new Intl.NumberFormat("en", { notation: "compact" }).format(value);
}

function pairMoves(moves: string[]) {
  const pairs: { turn: number; white: string; black?: string; whitePly: number; blackPly: number }[] = [];
  for (let index = 0; index < moves.length; index += 2) {
    pairs.push({
      turn: index / 2 + 1,
      white: moves[index],
      black: moves[index + 1],
      whitePly: index + 1,
      blackPly: index + 2,
    });
  }
  return pairs;
}

function buildSavedStats(games: SavedGame[]) {
  return games.reduce(
    (stats, game) => {
      if (game.result === "white win") {
        stats.whiteWins += 1;
      } else if (game.result === "black win") {
        stats.blackWins += 1;
      } else if (game.result.startsWith("draw")) {
        stats.draws += 1;
      }
      return stats;
    },
    { whiteWins: 0, blackWins: 0, draws: 0 },
  );
}
