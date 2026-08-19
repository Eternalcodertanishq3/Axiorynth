"use client";

import {
  ChevronLeft,
  ChevronRight,
  FlipHorizontal2,
  RotateCcw,
  SkipBack,
  LogOut,
  User as UserIcon,
  Trophy,
  Play,
  X,
  Eye,
  Settings,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import ChessBoard, { BoardThemeId, BOARD_THEMES, isKingInCheck, ChessPiece } from "../../components/ChessBoard";
import ThemeToggle from "../../components/ThemeToggle";
import CapturedMaterial from "../../components/CapturedMaterial";
import ClockDisplay from "../../components/ClockDisplay";

type Side = "white" | "black";
type Orientation = "white" | "black";

type UserPublic = {
  id: string;
  username: string;
  rating: number;
};

type LiveGameSummary = {
  id: string;
  white_user_id: string;
  black_user_id: string;
  result: string;
  time_control: string;
  created_at: string;
};

type LiveGameState = {
  id: string;
  white_user_id: string;
  black_user_id: string;
  white_username: string;
  black_username: string;
  fen: string;
  moves: string[];
  result: string;
  clock?: {
    white_ms: number;
    black_ms: number;
    active: "white" | "black";
  } | null;
};

const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";
const WS_BASE = process.env.NEXT_PUBLIC_WS_URL || "ws://127.0.0.1:8080";

export default function OnlinePlayPage() {
  // Auth
  const [token, setToken] = useState<string | null>(null);
  const [user, setUser] = useState<UserPublic | null>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [authMode, setAuthMode] = useState<"login" | "register">("login");
  const [authError, setAuthError] = useState<string | null>(null);
  const [authPending, setAuthPending] = useState(false);

  // Matchmaking / Lobbies
  const [inQueue, setInQueue] = useState(false);
  const [queueTimer, setQueueTimer] = useState(0);
  const [activeGames, setActiveGames] = useState<LiveGameSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isSpectator, setIsSpectator] = useState(false);

  // Live Game
  const [game, setGame] = useState<LiveGameState | null>(null);
  const [selectedSquare, setSelectedSquare] = useState<string | null>(null);
  const [orientation, setOrientation] = useState<Orientation>("white");
  const [replayPly, setReplayPly] = useState<number | null>(null);
  const [legalMoves, setLegalMoves] = useState<string[]>([]);
  const [promotionPending, setPromotionPending] = useState<{
    from: string;
    to: string;
    candidates: string[];
  } | null>(null);

  // Time control & multiplayer match states
  const [timeControl, setTimeControl] = useState<string>("5+3");
  const [drawOffer, setDrawOffer] = useState<string | null>(null);
  const [ratingChange, setRatingChange] = useState<{
    white_delta: number;
    black_delta: number;
    white_new: number;
    black_new: number;
  } | null>(null);
  const [opponentDisconnected, setOpponentDisconnected] = useState(false);

  // Appearance & Sound Settings
  const [themeId, setThemeId] = useState<BoardThemeId>("emerald");
  const [showCoordinates, setShowCoordinates] = useState(true);
  const [soundEnabled, setSoundEnabled] = useState(true);
  const [showSettings, setShowSettings] = useState(false);

  const changeTheme = (newTheme: BoardThemeId) => {
    setThemeId(newTheme);
    localStorage.setItem("axiorynth_board_theme", newTheme);
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

  const wsRef = useRef<WebSocket | null>(null);
  const queueIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const activeGamesIntervalRef = useRef<NodeJS.Timeout | null>(null);

  // Game state helpers (declared early for useEffect dependencies)
  const moves = game?.moves ?? [];
  const sideToMove: Side = moves.length % 2 === 0 ? "white" : "black";
  
  const playerColor: Side | null = useMemo(() => {
    if (!user || !game) return null;
    if (game.white_user_id === user.id) return "white";
    if (game.black_user_id === user.id) return "black";
    return null;
  }, [user, game]);

  // Fetch legal moves when game state changes
  useEffect(() => {
    if (!game || isSpectator || replayPly !== null) {
      setLegalMoves([]);
      return;
    }
    
    // Only fetch if it's our turn to speed things up
    if (playerColor !== sideToMove) {
      setLegalMoves([]);
      return;
    }

    const fetchLegalMoves = async () => {
      try {
        const res = await fetch(`${API_BASE}/api/state`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ moves: game.moves })
        });
        if (res.ok) {
          const data = await res.json();
          setLegalMoves(data.legalMoves || []);
        }
      } catch (err) {
        console.error("Failed to fetch legal moves", err);
      }
    };
    
    fetchLegalMoves();
  }, [game, isSpectator, replayPly, playerColor, sideToMove]);

  // Check saved token on mount & load board settings
  useEffect(() => {
    const savedToken = localStorage.getItem("axiorynth_token");
    if (savedToken) {
      setToken(savedToken);
      void fetchMe(savedToken);
    }

    const savedTheme = localStorage.getItem("axiorynth_board_theme") as BoardThemeId;
    if (savedTheme) setThemeId(savedTheme);
    
    const savedCoords = localStorage.getItem("axiorynth_board_coords");
    if (savedCoords !== null) setShowCoordinates(savedCoords === "true");
    
    const savedSound = localStorage.getItem("axiorynth_board_sound");
    if (savedSound !== null) setSoundEnabled(savedSound === "true");
  }, []);

  const fetchMe = async (authToken: string) => {
    try {
      const response = await fetch(`${API_BASE}/api/auth/me`, {
        headers: {
          Authorization: `Bearer ${authToken}`,
        },
      });
      if (response.ok) {
        const data = await response.json();
        setUser(data);
      } else {
        // Token expired/invalid
        localStorage.removeItem("axiorynth_token");
        setToken(null);
      }
    } catch {
      localStorage.removeItem("axiorynth_token");
      setToken(null);
    }
  };

  const handleAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim() || !password.trim()) {
      setAuthError("Username and password are required.");
      return;
    }
    setAuthError(null);
    setAuthPending(true);

    try {
      const url = authMode === "login" ? "/api/auth/login" : "/api/auth/register";
      const response = await fetch(`${API_BASE}${url}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      const data = await response.json();
      if (response.ok) {
        localStorage.setItem("axiorynth_token", data.token);
        setToken(data.token);
        setUser(data.user);
        setUsername("");
        setPassword("");
      } else {
        setAuthError(data.error ?? "Authentication failed");
      }
    } catch (err) {
      setAuthError("Could not connect to backend server.");
    } finally {
      setAuthPending(false);
    }
  };

  const handleLogout = async () => {
    if (token) {
      const _ = fetch(`${API_BASE}/api/auth/logout`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
      });
    }
    localStorage.removeItem("axiorynth_token");
    setToken(null);
    setUser(null);
    leaveQueue();
    disconnectWs();
    setGame(null);
  };

  // Fetch list of active games for spectating
  const fetchActiveGames = useCallback(async () => {
    try {
      const response = await fetch(`${API_BASE}/api/live/games`);
      if (response.ok) {
        const data = await response.json();
        setActiveGames(data);
      }
    } catch (e) {
      console.error("Failed to load active games:", e);
    }
  }, []);

  // Poll active games when logged in and not in game
  useEffect(() => {
    if (user && !game) {
      void fetchActiveGames();
      activeGamesIntervalRef.current = setInterval(fetchActiveGames, 5000);
    } else {
      if (activeGamesIntervalRef.current) {
        clearInterval(activeGamesIntervalRef.current);
      }
    }
    return () => {
      if (activeGamesIntervalRef.current) {
        clearInterval(activeGamesIntervalRef.current);
      }
    };
  }, [user, game, fetchActiveGames]);

  // Queue timer ticker
  useEffect(() => {
    let interval: NodeJS.Timeout;
    if (inQueue) {
      interval = setInterval(() => {
        setQueueTimer((t) => t + 1);
      }, 1000);
    } else {
      setQueueTimer(0);
    }
    return () => clearInterval(interval);
  }, [inQueue]);

  // Main matchmaking logic: poll queue status & games list
  const startQueuePolling = useCallback(() => {
    if (queueIntervalRef.current) clearInterval(queueIntervalRef.current);
    
    queueIntervalRef.current = setInterval(async () => {
      if (!token || !user) return;
      try {
        // 1. Check if we are still in queue status
        const queueRes = await fetch(`${API_BASE}/api/matchmaking/status`);
        if (queueRes.ok) {
          const queueList = await queueRes.json();
          const stillQueued = queueList.some((entry: any) => entry.user_id === user.id);
          
          if (!stillQueued) {
            // We were removed from the queue! Let's check active games to see if we matched
            const gamesRes = await fetch(`${API_BASE}/api/live/games`);
            if (gamesRes.ok) {
              const games = await gamesRes.json();
              const matchedGame = games.find(
                (g: any) => g.white_user_id === user.id || g.black_user_id === user.id
              );
              
              if (matchedGame) {
                // Matched!
                clearInterval(queueIntervalRef.current!);
                setInQueue(false);
                setIsSpectator(false);
                connectWs(matchedGame.id);
              } else {
                // Not found in active games and not in queue? Player must have left/rebooted
                setInQueue(false);
              }
            }
          }
        }
      } catch (err) {
        console.error("Queue poll error:", err);
      }
    }, 1500);
  }, [token, user]);

  const joinQueue = async () => {
    if (!token) return;
    setError(null);
    try {
      const response = await fetch(`${API_BASE}/api/matchmaking/queue`, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ time_control: timeControl }),
      });
      if (response.ok) {
        const data = await response.json();
        if (data.status === "matched") {
          setIsSpectator(false);
          connectWs(data.match.game_id);
        } else {
          setInQueue(true);
          startQueuePolling();
        }
      } else {
        const errData = await response.text();
        setError(errData || "Failed to join queue");
      }
    } catch {
      setError("Failed to connect to matchmaking server.");
    }
  };

  const leaveQueue = async () => {
    if (!token) return;
    setInQueue(false);
    if (queueIntervalRef.current) {
      clearInterval(queueIntervalRef.current);
    }
    try {
      await fetch(`${API_BASE}/api/matchmaking/queue`, {
        method: "DELETE",
        headers: { Authorization: `Bearer ${token}` },
      });
    } catch (e) {
      console.error("Failed to leave queue:", e);
    }
  };

  const offerDraw = () => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ action: "draw_offer" }));
  };

  const acceptDraw = () => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ action: "draw_accept" }));
    setDrawOffer(null);
  };

  const declineDraw = () => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ action: "draw_decline" }));
    setDrawOffer(null);
  };

  // WebSocket Live Game Connection
  const connectWs = (gameId: string) => {
    disconnectWs();
    setSelectedSquare(null);
    setPromotionPending(null);
    setReplayPly(null);
    setDrawOffer(null);
    setRatingChange(null);
    setOpponentDisconnected(false);

    const wsUrl = token ? `${WS_BASE}/ws/live/${gameId}?token=${encodeURIComponent(token)}` : `${WS_BASE}/ws/live/${gameId}`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.type === "game_state") {
          setGame(data);
          
          // Auto set orientation based on player color
          if (user && data.black_user_id === user.id) {
            setOrientation("black");
          } else {
            setOrientation("white");
          }
        } else if (data.type === "draw_offered") {
          setDrawOffer(data.by);
        } else if (data.type === "draw_declined") {
          setDrawOffer(null);
        } else if (data.type === "player_disconnected") {
          setOpponentDisconnected(true);
        } else if (data.type === "player_reconnected") {
          setOpponentDisconnected(false);
        } else if (data.type === "rating_update") {
          setRatingChange(data);
        } else if (data.type === "error") {
          setError(data.message);
        }
      } catch (e) {
        console.error("Failed to parse game update:", e);
      }
    };

    ws.onclose = () => {
      wsRef.current = null;
    };

    ws.onerror = () => {
      setError("WebSocket connection error.");
    };
  };

  const spectateGame = (gameId: string) => {
    setIsSpectator(true);
    setOrientation("white");
    connectWs(gameId);
  };

  const disconnectWs = () => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
  };

  const makeMove = (uciMove: string) => {
    if (!wsRef.current || !user || isSpectator || !game) return;
    wsRef.current.send(
      JSON.stringify({
        action: "move",
        uci_move: uciMove,
        user_id: user.id,
      })
    );
  };

  const resign = () => {
    if (!wsRef.current || !user || isSpectator || !game) return;
    if (confirm("Are you sure you want to resign?")) {
      wsRef.current.send(
        JSON.stringify({
          action: "resign",
          user_id: user.id,
        })
      );
    }
  };



  const activeFen = useMemo(() => {
    if (!game) return START_FEN;
    if (replayPly === null) return game.fen;
    if (replayPly <= 0) return START_FEN;
    
    // For replay states, since we don't have full history from WebSocket,
    // we can reconstruct the board using the engine logic.
    // However, to keep it fast, we will rely on client replaying.
    // If not matching current ply, let's fall back to game.fen or START_FEN
    return game.fen;
  }, [replayPly, game]);

  const board = useMemo(() => parseFen(activeFen), [activeFen]);
  const squareOrder = useMemo(() => buildSquareOrder(orientation), [orientation]);
  const live = replayPly === null;

  // Derive legal moves from FEN using client-side helper or basic logic.
  // Note: we can generate pseudo-legal moves for pieces to guide highlighting,
  // but actual legal moves are validated server-side.
  const targetSquares = useMemo(() => {
    const targetSet = new Set<string>();
    if (!selectedSquare || !live || isSpectator) return targetSet;
    
    // Check legal moves for the selected piece
    for (const move of legalMoves) {
      if (move.startsWith(selectedSquare)) {
        targetSet.add(move.slice(2, 4));
      }
    }
    return targetSet;
  }, [selectedSquare, live, isSpectator, legalMoves]);

  function handleSquareClick(square: string) {
    if (!game || !live || isSpectator) return;
    if (game.result !== "ongoing") return;

    // Verify it's the player's turn
    if (playerColor !== sideToMove) {
      setError("It's not your turn!");
      return;
    }

    if (selectedSquare) {
      if (selectedSquare === square) {
        setSelectedSquare(null);
        return;
      }

      // Propose UCI move
      const move = `${selectedSquare}${square}`;
      
      // Basic promotion check (Pawn moving to 8th or 1st rank)
      const piece = board[selectedSquare];
      const isPawn = piece?.toLowerCase() === "p";
      const targetRank = square[1];
      const isPromotionRank = (piece === "P" && targetRank === "8") || (piece === "p" && targetRank === "1");

      if (isPawn && isPromotionRank) {
        setPromotionPending({
          from: selectedSquare,
          to: square,
          candidates: [`${move}q`, `${move}r`, `${move}b`, `${move}n`],
        });
      } else {
        makeMove(move);
        setSelectedSquare(null);
      }
    } else {
      const piece = board[square];
      if (piece) {
        const isWhitePiece = piece === piece.toUpperCase();
        if ((playerColor === "white" && isWhitePiece) || (playerColor === "black" && !isWhitePiece)) {
          setSelectedSquare(square);
          setError(null);
        }
      }
    }
  }

  function handleMoveDrop(from: string, to: string) {
    if (!game || !live || isSpectator) return;
    if (game.result !== "ongoing") return;

    if (playerColor !== sideToMove) {
      setError("It's not your turn!");
      return;
    }

    const move = `${from}${to}`;
    const piece = board[from];
    const isPawn = piece?.toLowerCase() === "p";
    const targetRank = to[1];
    const isPromotionRank = (piece === "P" && targetRank === "8") || (piece === "p" && targetRank === "1");

    if (isPawn && isPromotionRank) {
      setPromotionPending({
        from,
        to,
        candidates: [`${move}q`, `${move}r`, `${move}b`, `${move}n`],
      });
    } else {
      makeMove(move);
      setSelectedSquare(null);
    }
  }

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnectWs();
      if (queueIntervalRef.current) clearInterval(queueIntervalRef.current);
      if (activeGamesIntervalRef.current) clearInterval(activeGamesIntervalRef.current);
    };
  }, []);

  // Format active games created time
  const formatTime = (isoString: string) => {
    try {
      const d = new Date(isoString);
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch {
      return "";
    }
  };

  // Replay helpers
  const replayValue = replayPly ?? moves.length;

  return (
    <div className="dark-glow-bg min-h-screen custom-scroll">
      {/* Decorative ambient background glows */}
      <div className="luxury-glow-1" />
      <div className="luxury-glow-2" />
      
      {/* Sleek Floating Turn Banner */}
      {game && (
        <div className="turn-toast border-surface">
          <div 
            className="w-3 h-3 rounded-full animate-pulse"
            style={{ 
              background: sideToMove === "white" ? "#ffffff" : "#1e293b",
              boxShadow: `0 0 8px ${sideToMove === "white" ? "#ffffff" : "#1e293b"}`
            }}
          />
          <span className="text-sm font-bold tracking-wide">
            {game.result !== "ongoing"
              ? `Game Over: ${game.result}`
              : sideToMove === "white"
              ? "White's Turn"
              : "Black's Turn"
            }
          </span>
        </div>
      )}

      <main className="app-shell max-w-[1400px] mx-auto px-6 py-6" style={{ display: "flex", flexDirection: "column", minHeight: "100vh" }}>
        
        {/* Header */}
        <header className="flex justify-between items-center mb-8 border-b border-surface pb-6 z-10">
          <div>
            <Link href="/" className="text-xs font-bold text-accent-cyan tracking-widest uppercase hover:underline">
              ← Back to Main Menu
            </Link>
            <h1 className="text-3xl font-extrabold tracking-tight text-white mt-1">
              Online <span className="hero-gradient-text">Platform</span>
            </h1>
          </div>
          
          <div className="flex gap-4 items-center">
            <ThemeToggle />
            {user && (
              <>
                <div className="flex items-center gap-2 luxury-card/40 border border-surface py-1.5 px-3 rounded-lg text-primary text-xs">
                  <UserIcon size={14} className="text-accent-cyan" />
                  <strong>{user.username}</strong>
                </div>
                <div className="flex items-center gap-2 bg-accent-gold-soft border border-accent-gold-soft py-1.5 px-3 rounded-lg text-accent-gold text-xs font-bold">
                  <Trophy size={14} />
                  <span>{user.rating} Elo</span>
                </div>
                <button 
                  onClick={handleLogout} 
                  className="hero-btn-secondary py-1.5 px-3 min-h-0 text-xs text-accent-rose border-accent-rose-soft hover:bg-accent-rose-soft hover:text-primary"
                >
                  <LogOut size={12} />
                  Logout
                </button>
              </>
            )}
          </div>
        </header>

        {error && (
          <div className="border border-red-800 bg-red-950/20 text-accent-rose text-sm font-bold px-6 py-4 rounded-xl flex justify-between items-center mb-6 z-10">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="text-accent-rose hover:text-primary bg-transparent border-0 min-h-0 p-0"><X size={16} /></button>
          </div>
        )}

        {/* 1. Login / Register screen */}
        {!user ? (
          <div style={{ flex: 1, display: "flex", justifyContent: "center", alignItems: "center", padding: "40px 0" }}>
            <div className="glass-panel w-full max-w-[420px] p-8 border border-surface luxury-glass">
              <h2 className="text-xl font-extrabold text-white text-center mb-2">
                {authMode === "login" ? "Welcome Back" : "Create Account"}
              </h2>
              <p className="text-secondary text-xs text-center mb-6 font-semibold">
                Join the Axiorynth community for real-time multiplayer chess.
              </p>

              {authError && (
                <div className="text-xs font-bold text-accent-rose border border-red-900/30 bg-red-950/20 p-3 rounded-lg mb-4 text-center">
                  {authError}
                </div>
              )}

              <form onSubmit={handleAuth} style={{ display: "flex", flexDirection: "column", gap: "15px" }}>
                <div>
                  <label className="block text-[10px] font-bold uppercase tracking-wider text-secondary mb-1.5">Username</label>
                  <input
                    type="text"
                    value={username}
                    onChange={(e) => setUsername(e.target.value)}
                    placeholder="Enter username"
                    style={{ width: "100%", padding: "10px", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.08)", background: "rgba(0,0,0,0.2)", color: "#ffffff" }}
                    className="focus:outline-none focus:border-accent-cyan"
                  />
                </div>

                <div>
                  <label className="block text-[10px] font-bold uppercase tracking-wider text-secondary mb-1.5">Password</label>
                  <input
                    type="password"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    placeholder="Enter password"
                    style={{ width: "100%", padding: "10px", borderRadius: "8px", border: "1px solid rgba(255,255,255,0.08)", background: "rgba(0,0,0,0.2)", color: "#ffffff" }}
                    className="focus:outline-none focus:border-accent-cyan"
                  />
                </div>

                <button
                  type="submit"
                  disabled={authPending}
                  className="hero-btn-primary w-full mt-4"
                >
                  {authPending ? "Authenticating..." : authMode === "login" ? "Login" : "Register & Start"}
                </button>
              </form>

              <div style={{ marginTop: "20px", textAlign: "center", fontSize: "0.85rem" }} className="text-secondary">
                {authMode === "login" ? (
                  <span>
                    Don't have an account?{" "}
                    <button onClick={() => { setAuthMode("register"); setAuthError(null); }} className="text-accent-cyan hover:text-accent-cyan font-bold underline bg-transparent border-0 min-h-0 p-0">
                      Sign Up
                    </button>
                  </span>
                ) : (
                  <span>
                    Already registered?{" "}
                    <button onClick={() => { setAuthMode("login"); setAuthError(null); }} className="text-accent-cyan hover:text-accent-cyan font-bold underline bg-transparent border-0 min-h-0 p-0">
                      Sign In
                    </button>
                  </span>
                )}
              </div>
            </div>
          </div>
        ) : (
          /* 2. Main Authenticated workspace */
          <div style={{ flex: 1 }} className="z-10">
            
            {/* Dashboard (Not in game) */}
            {!game ? (
              <div className="grid luxury-grid-three gap-8 max-w-4xl mx-auto mt-10">
                
                {/* Queue panel */}
                <div className="glass-panel p-8 flex flex-col items-center justify-center text-center min-h-[300px]">
                  {!inQueue ? (
                    <>
                      <div className="w-16 h-16 rounded-full bg-accent-cyan-soft flex items-center justify-center text-accent-cyan border border-accent-cyan-soft mb-6">
                        <Play size={30} fill="currentColor" />
                      </div>
                      <h2 className="text-lg font-bold text-white mb-2">Multiplayer Matchmaking</h2>
                      <p className="text-secondary text-xs mb-4 max-w-xs leading-relaxed">
                        Join the competitive queue to be paired with an online opponent close to your rating.
                      </p>

                      {/* Time Control Selection */}
                      <div className="flex flex-wrap justify-center gap-2 mb-6 max-w-xs">
                        {[
                          { label: "1+0", desc: "Bullet" },
                          { label: "2+1", desc: "Bullet" },
                          { label: "3+0", desc: "Blitz" },
                          { label: "5+3", desc: "Blitz" },
                          { label: "10+0", desc: "Rapid" },
                          { label: "15+10", desc: "Rapid" },
                          { label: "∞", desc: "Unlimited" },
                        ].map((tc) => (
                          <button
                            key={tc.label}
                            type="button"
                            onClick={() => setTimeControl(tc.label)}
                            className={`px-3 py-1.5 rounded-lg text-xs font-bold transition-all border ${
                              timeControl === tc.label
                                ? "bg-accent-gold-soft border-accent-gold text-accent-gold shadow-sm"
                                : "border-surface text-secondary hover:text-primary hover:border-surface-2"
                            }`}
                          >
                            {tc.label}
                          </button>
                        ))}
                      </div>

                      <button
                        onClick={joinQueue}
                        className="hero-btn-primary py-3 px-8 text-sm"
                      >
                        Find {timeControl} Game
                      </button>
                    </>
                  ) : (
                    <>
                      <div className="w-16 h-16 rounded-full border-4 border-sky-400 border-t-transparent animate-spin mb-6" />
                      <h2 className="text-lg font-bold text-white mb-2">Looking for Opponent...</h2>
                      <p className="text-secondary text-xs mb-1">Rating: {user.rating} Elo</p>
                      <p className="text-accent-cyan text-xs font-bold mb-6">
                        Elapsed: {Math.floor(queueTimer / 60)}:{(queueTimer % 60).toString().padStart(2, "0")}
                      </p>
                      <button
                        onClick={leaveQueue}
                        className="hero-btn-secondary border-accent-rose-soft text-accent-rose hover:bg-accent-rose-soft hover:text-primary py-2 px-6"
                      >
                        Cancel Search
                      </button>
                    </>
                  )}
                </div>

                {/* Lobbies / Live games list */}
                <div className="glass-panel p-8 flex flex-col min-h-[300px]">
                  <div className="flex justify-between items-center mb-6 border-b border-surface pb-3">
                    <h2 className="text-lg font-bold text-white">Active Games</h2>
                    <span className="text-xs font-bold text-accent-cyan bg-accent-cyan-soft py-1 px-3 rounded-full">{activeGames.length} live</span>
                  </div>
                  
                  <div className="flex-1 overflow-y-auto max-h-[220px] custom-scroll flex flex-col gap-3">
                    {activeGames.length === 0 ? (
                      <p className="text-muted text-xs italic text-center py-12">No games are currently being played.</p>
                    ) : (
                      activeGames.map((g) => (
                        <div
                          key={g.id}
                          className="glass-card flex justify-between items-center p-4 border border-surface"
                        >
                          <div className="flex flex-col gap-1">
                            <span className="text-xs font-bold text-primary">
                              Game {g.id.slice(0, 8)}...
                            </span>
                            <span className="text-[10px] text-muted">
                              Started at {formatTime(g.created_at)}
                            </span>
                          </div>
                          <button
                            onClick={() => spectateGame(g.id)}
                            className="hero-btn-secondary py-1.5 px-4 min-h-0 text-xs border-surface text-accent-cyan hover:text-primary hover:bg-accent-cyan hover:border-accent-cyan"
                          >
                            <Eye size={12} />
                            Spectate
                          </button>
                        </div>
                      ))
                    )}
                  </div>
                </div>

              </div>
            ) : (
              /* Active Game Workspace */
              <div className="grid luxury-grid-three gap-8 items-start mt-6">
                
                {/* Board and Controls */}
                <div className=" flex flex-col items-center">
                  
                  {/* Game header panel */}
                  <div className="glass-panel w-full mb-6 p-4 flex justify-between items-center flex-wrap gap-4">
                    <div className="flex gap-6 items-center flex-wrap">
                      <div>
                        <small className="text-[10px] font-bold uppercase tracking-wider text-secondary block mb-0.5">White Player</small>
                        <h3 className="text-sm font-bold text-white">
                          {game.white_username} {user && game.white_user_id === user.id ? "(You)" : ""}
                        </h3>
                      </div>
                      <div className="text-muted font-extrabold text-xs luxury-glass px-2 py-0.5 rounded border border-surface">VS</div>
                      <div>
                        <small className="text-[10px] font-bold uppercase tracking-wider text-secondary block mb-0.5">Black Player</small>
                        <h3 className="text-sm font-bold text-white">
                          {game.black_username} {user && game.black_user_id === user.id ? "(You)" : ""}
                        </h3>
                      </div>
                    </div>

                    <div className="flex gap-2 items-center">
                      {/* Visible Board Theme swatches */}
                      <div className="flex items-center gap-1.5 border-r border-[var(--clay-border-color)] pr-3 mr-1">
                        {(Object.keys(BOARD_THEMES) as BoardThemeId[]).map((id) => {
                          const t = BOARD_THEMES[id];
                          const active = themeId === id;
                          return (
                            <button
                              key={id}
                              onClick={() => changeTheme(id)}
                              title={t.name}
                              className={`w-6 h-6 rounded-full border overflow-hidden transition-all ${active ? "border-[var(--text-accent)] scale-110 ring-2 ring-[var(--text-accent)]/20" : "border-[var(--clay-border-color)] hover:scale-105"}`}
                            >
                              <div className="w-full h-full" style={{ background: `linear-gradient(135deg, ${t.light} 50%, ${t.dark} 50%)` }} />
                            </button>
                          );
                        })}
                      </div>

                      <div className="relative">
                        <button
                          className="hero-btn-secondary p-2 min-h-0 text-xs py-1.5 aspect-square flex items-center justify-center"
                          onClick={() => setShowSettings(!showSettings)}
                          title="Settings"
                        >
                          <Settings size={14} />
                        </button>
                        
                        {showSettings && (
                          <div className="absolute right-0 top-10 glass-panel w-64 p-4 z-50 flex flex-col gap-4 border border-[var(--clay-border-color)] bg-[var(--clay-panel-bg)] shadow-xl">
                            <h4 className="text-sm font-bold text-[var(--text-primary)] border-b border-[var(--clay-border-color)] pb-2">Appearance Settings</h4>
                            <div className="flex justify-between items-center">
                              <span className="text-xs font-semibold text-[var(--text-secondary)]">Show Coordinates</span>
                              <button onClick={toggleCoords} className={`w-10 h-5 rounded-full relative transition-all ${showCoordinates ? "bg-accent-cyan" : "bg-slate-300"}`}>
                                <div className={`w-4 h-4 rounded-full bg-white absolute top-0.5 transition-all ${showCoordinates ? "left-5.5" : "left-0.5"}`} />
                              </button>
                            </div>
                            <div className="flex justify-between items-center">
                              <span className="text-xs font-semibold text-[var(--text-secondary)]">Sound Effects</span>
                              <button onClick={toggleSound} className={`w-10 h-5 rounded-full relative transition-all ${soundEnabled ? "bg-accent-cyan" : "bg-slate-300"}`}>
                                <div className={`w-4 h-4 rounded-full bg-white absolute top-0.5 transition-all ${soundEnabled ? "left-5.5" : "left-0.5"}`} />
                              </button>
                            </div>
                          </div>
                        )}
                      </div>

                      <button
                        className="hero-btn-secondary p-2 min-h-0 text-xs py-1.5 aspect-square flex items-center justify-center"
                        onClick={() => setOrientation((val) => (val === "white" ? "black" : "white"))}
                        title="Flip Board"
                      >
                        <FlipHorizontal2 size={14} />
                      </button>
                      {!isSpectator && game.result === "ongoing" && (
                        <>
                          <button
                            onClick={offerDraw}
                            className="hero-btn-secondary py-1.5 px-3 min-h-0 text-xs border-surface text-secondary hover:text-primary hover:border-surface-2"
                            title="Offer Draw"
                          >
                            Draw
                          </button>
                          <button
                            onClick={resign}
                            className="hero-btn-secondary py-1.5 px-4 min-h-0 text-xs border-accent-rose-soft text-accent-rose hover:bg-accent-rose-soft hover:text-primary"
                          >
                            Resign
                          </button>
                        </>
                      )}
                      {(isSpectator || game.result !== "ongoing") && (
                        <button
                          onClick={() => { disconnectWs(); setGame(null); }}
                          className="hero-btn-primary py-1.5 px-4 min-h-0 text-xs text-white"
                        >
                          Back to Lobbies
                        </button>
                      )}
                    </div>
                  </div>

                  {/* Disconnect Banner */}
                  {opponentDisconnected && (
                    <div className="w-full max-w-[640px] mb-3 p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 text-amber-400 text-xs font-bold text-center animate-pulse">
                      Opponent disconnected. Auto-win in 30 seconds if they do not reconnect.
                    </div>
                  )}

                  {/* Draw Offer Banner */}
                  {drawOffer && !isSpectator && (
                    <div className="w-full max-w-[640px] mb-3 p-3 rounded-lg bg-accent-cyan-soft border border-accent-cyan/30 text-accent-cyan text-xs font-bold flex justify-between items-center">
                      <span>Opponent offered a draw.</span>
                      <div className="flex gap-2">
                        <button
                          onClick={acceptDraw}
                          className="px-3 py-1 bg-accent-emerald text-black text-xs font-bold rounded hover:opacity-90 transition-all"
                        >
                          Accept
                        </button>
                        <button
                          onClick={declineDraw}
                          className="px-3 py-1 bg-surface-2 text-secondary text-xs font-bold rounded hover:text-primary transition-all"
                        >
                          Decline
                        </button>
                      </div>
                    </div>
                  )}

                  {/* Rating Update Banner */}
                  {ratingChange && game.result !== "ongoing" && (
                    <div className="w-full max-w-[640px] mb-4 p-4 rounded-xl luxury-glass border border-accent-gold/20 text-center">
                      <h4 className="text-xs font-bold text-accent-gold uppercase tracking-wider mb-2">Rating Adjustment (Glicko-2)</h4>
                      <div className="flex justify-around items-center text-sm font-bold">
                        <div>
                          <span className="text-secondary text-xs block">{game.white_username} (White)</span>
                          <span className={ratingChange.white_delta >= 0 ? "text-accent-emerald" : "text-accent-rose"}>
                            {ratingChange.white_delta >= 0 ? `+${ratingChange.white_delta}` : ratingChange.white_delta} ({ratingChange.white_new} Elo)
                          </span>
                        </div>
                        <div className="text-muted text-xs">vs</div>
                        <div>
                          <span className="text-secondary text-xs block">{game.black_username} (Black)</span>
                          <span className={ratingChange.black_delta >= 0 ? "text-accent-emerald" : "text-accent-rose"}>
                            {ratingChange.black_delta >= 0 ? `+${ratingChange.black_delta}` : ratingChange.black_delta} ({ratingChange.black_new} Elo)
                          </span>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Top Player HUD: Captured Material & Clock */}
                  <div className="w-full max-w-[640px] flex justify-between items-center mb-2 px-1">
                    <CapturedMaterial fen={activeFen} orientation={orientation} isTop={true} />
                    {game.clock && (
                      <ClockDisplay
                        whiteMs={game.clock.white_ms}
                        blackMs={game.clock.black_ms}
                        activeColor={game.clock.active}
                        orientation={orientation}
                        whiteName={game.white_username}
                        blackName={game.black_username}
                        isTop={true}
                      />
                    )}
                  </div>

                  {/* The Chessboard Container */}
                  <div className="board-frame relative w-full aspect-square max-w-[640px]">
                    <ChessBoard
                      board={board}
                      orientation={orientation}
                      selectedSquare={selectedSquare}
                      targetSquares={targetSquares}
                      lastMove={moves[moves.length - 1] ?? null}
                      onSquareClick={handleSquareClick}
                      onMoveDrop={handleMoveDrop}
                      themeId={themeId}
                      showCoordinates={showCoordinates}
                      soundEnabled={soundEnabled}
                      inCheck={isKingInCheck(board, activeFen)}
                      movesCount={moves.length}
                      fen={activeFen}
                      result={game.result}
                    />

                    {/* Promotion choice overlay */}
                    {promotionPending && (
                      <div className="promotion-overlay">
                        <div className="promotion-modal luxury-glass border-surface text-white shadow-2xl p-6">
                          <h3 className="text-lg font-bold text-accent-cyan mb-2">Choose Promotion Piece</h3>
                          <div className="promotion-choices gap-3 mb-4">
                            {[
                              { piece: "q", label: "Queen", icon: sideToMove === "white" ? "Q" : "q" },
                              { piece: "r", label: "Rook", icon: sideToMove === "white" ? "R" : "r" },
                              { piece: "b", label: "Bishop", icon: sideToMove === "white" ? "B" : "b" },
                              { piece: "n", label: "Knight", icon: sideToMove === "white" ? "N" : "n" },
                            ].map(({ piece, label, icon }) => (
                              <button
                                key={piece}
                                onClick={() => {
                                  const chosenMove = promotionPending.candidates.find((c) => c.endsWith(piece));
                                  if (chosenMove) {
                                    makeMove(chosenMove);
                                  }
                                  setPromotionPending(null);
                                  setSelectedSquare(null);
                                }}
                                className="promotion-choice-btn luxury-card border-surface hover:bg-accent-cyan hover:border-accent-cyan transition-all text-primary"
                              >
                                <ChessPiece piece={icon} />
                                <span className="text-[10px] mt-1">{label}</span>
                              </button>
                            ))}
                          </div>
                          <button
                            className="promotion-cancel-btn border-surface hover:bg-accent-rose-soft hover:text-accent-rose"
                            onClick={() => {
                              setPromotionPending(null);
                              setSelectedSquare(null);
                            }}
                          >
                            Cancel Move
                          </button>
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Bottom Player HUD: Captured Material & Clock */}
                  <div className="w-full max-w-[640px] flex justify-between items-center mt-2 px-1">
                    <CapturedMaterial fen={activeFen} orientation={orientation} isTop={false} />
                    {game.clock && (
                      <ClockDisplay
                        whiteMs={game.clock.white_ms}
                        blackMs={game.clock.black_ms}
                        activeColor={game.clock.active}
                        orientation={orientation}
                        whiteName={game.white_username}
                        blackName={game.black_username}
                        isTop={false}
                      />
                    )}
                  </div>

                  {/* Game outcome notification */}
                  {game.result !== "ongoing" && (
                    <div className="glass-panel w-full max-w-[640px] mt-6 p-6 text-center border-accent-emerald-soft">
                      <h3 className="text-lg font-bold text-accent-emerald mb-1">Game Complete</h3>
                      <p className="text-primary font-bold text-xs mb-4">Result: {game.result}</p>
                      <button
                        onClick={() => { disconnectWs(); setGame(null); }}
                        className="hero-btn-primary py-2 px-6 text-xs text-white"
                      >
                        Return to Dashboard
                      </button>
                    </div>
                  )}

                </div>

                {/* Game details and history */}
                <aside className=" flex flex-col gap-6">
                  
                  {/* Match Info */}
                  <section className="glass-panel p-5 flex flex-col min-h-[200px]">
                    <div className="flex justify-between items-center mb-4 border-b border-surface pb-3">
                      <div>
                        <small className="text-[10px] font-bold uppercase tracking-wider text-secondary block mb-0.5">Match Info</small>
                        <h2 className="text-base font-bold text-white">
                          {game.result === "ongoing"
                            ? `Turn: ${sideToMove === "white" ? "White" : "Black"}`
                            : "Game Concluded"}
                        </h2>
                      </div>
                      <span className="text-xs font-bold text-accent-cyan bg-accent-cyan-soft py-1 px-3 rounded-full">
                        {isSpectator ? "Spectating" : playerColor ? `Playing ${playerColor}` : ""}
                      </span>
                    </div>

                    <div style={{ flex: 1, padding: "10px 0" }}>
                      <p className="text-xs text-secondary mb-2 font-semibold">Active FEN String:</p>
                      <code className="text-[11px] font-mono block luxury-glass p-3 rounded-lg border border-slate-900 text-accent-cyan break-all leading-normal select-all">
                        {game.fen}
                      </code>
                    </div>
                  </section>

                  {/* Move History */}
                  <section className="glass-panel p-5 flex flex-col flex-1 min-h-[300px]">
                    <div className="flex justify-between items-center mb-4 border-b border-surface pb-3">
                      <h3 className="text-sm font-bold text-white">Move Log</h3>
                      <span className="text-xs font-bold text-secondary">{moves.length} plies</span>
                    </div>
                    
                    <div className="overflow-y-auto max-h-[300px] pr-2 custom-scroll flex flex-col gap-2">
                      {moves.length === 0 ? (
                        <p className="text-muted text-xs italic text-center py-12">No moves played yet.</p>
                      ) : (
                        pairMoves(moves).map((pair) => (
                          <div className="grid grid-cols-12 text-xs py-1.5 px-2 rounded hover:luxury-hover transition-all border-b border-surface/20" key={pair.turn}>
                            <span className="col-span-3 text-muted font-bold">{pair.turn}.</span>
                            <span className="col-span-4 font-mono font-bold text-primary">{pair.white}</span>
                            {pair.black && <span className="col-span-5 font-mono font-bold text-primary">{pair.black}</span>}
                          </div>
                        ))
                      )}
                    </div>
                  </section>
                </aside>

              </div>
            )}

          </div>
        )}
      </main>
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
