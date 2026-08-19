"use client";

import Link from "next/link";
import {
  Play,
  Cpu,
  Globe,
  Activity,
  TrendingUp,
  Brain,
  Zap,
  ShieldAlert,
  Sparkles,
  Volume2,
  Clock,
  Crosshair,
} from "lucide-react";

import ThemeToggle from "../components/ThemeToggle";

export default function LandingPage() {
  return (
    <div className="luxury-landing">
      {/* Decorative ambient background glows */}
      <div className="luxury-glow-1" />
      <div className="luxury-glow-2" />

      {/* Luxurious Header Navigation */}
      <header className="luxury-nav">
        <Link href="/" className="luxury-logo-container">
          <div className="luxury-logo-badge">A</div>
          <span className="luxury-logo-text">AXIORYNTH</span>
        </Link>
        <div className="luxury-nav-actions">
          <ThemeToggle />
          <Link href="/play" className="luxury-btn luxury-btn-outline">
            Engine Lab
          </Link>
          <Link href="/online" className="luxury-btn luxury-btn-gold">
            Play Online
          </Link>
        </div>
      </header>

      {/* Hero Section */}
      <section className="luxury-hero-section">
        {/* Floating status badge */}
        <div className="luxury-hero-badge">
          <Activity size={13} className="text-accent-gold" />
          <span>Tier 1 Grandmaster Architecture Live</span>
        </div>

        <h1 className="luxury-title-grand">
          The Math-First <br />
          <span className="luxury-gradient-text">Chess Sandbox</span>
        </h1>

        <p className="luxury-subtitle-refined">
          Experience grandmaster chess powered by a from-scratch Rust bitboard engine. Observe live search brainwaves, win-probability move hints, self-improving neural evaluations, and server-authoritative multiplayer.
        </p>

        {/* Primary and Secondary Call to Actions */}
        <div className="luxury-hero-actions">
          <Link href="/play" className="luxury-btn luxury-btn-gold">
            <Play size={16} fill="currentColor" />
            Play vs Axiorynth Bot
          </Link>
          <Link href="/online" className="luxury-btn luxury-btn-outline">
            <Globe size={16} />
            Matchmake Multiplayer
          </Link>
        </div>

        {/* Core Pillars (3 Hero Cards) */}
        <div className="luxury-pillar-grid">
          {/* Pillar 1 */}
          <div className="luxury-pillar-card">
            <div className="luxury-pillar-icon">
              <Cpu size={24} />
            </div>
            <h3 className="luxury-pillar-title">Bitboard Rust Engine</h3>
            <p className="luxury-pillar-desc">
              Written from scratch in Rust, representing positions as 64-bit integer words. Verified by extensive perft suites to ensure 100% legal move generation with zero garbage collection pauses.
            </p>
          </div>

          {/* Pillar 2 */}
          <div className="luxury-pillar-card">
            <div className="luxury-pillar-icon">
              <TrendingUp size={24} />
            </div>
            <h3 className="luxury-pillar-title">Selective Pruning</h3>
            <p className="luxury-pillar-desc">
              Prunes millions of useless moves using Principal Variation Search (PVS), Aspiration Windows, Late Move Reductions (LMR), and Null-Move Pruning (NMP) for deep tactical clarity.
            </p>
          </div>

          {/* Pillar 3 */}
          <div className="luxury-pillar-card">
            <div className="luxury-pillar-icon">
              <Brain size={24} />
            </div>
            <h3 className="luxury-pillar-title">NNUE & Syzygy</h3>
            <p className="luxury-pillar-desc">
              Evaluates positions with a HalfKP neural network (40,960 features) trained on self-play games. Instantly solves endgames of 7 or fewer pieces using Syzygy tablebases.
            </p>
          </div>
        </div>

        {/* 4-Stat Telemetry KPI Strip */}
        <div className="stats-strip">
          <div className="stat-item">
            <div className="stat-num">2.5M+</div>
            <div className="stat-label">Nodes / Sec Search</div>
          </div>
          <div className="stat-item">
            <div className="stat-num">40,960</div>
            <div className="stat-label">NNUE Neural Features</div>
          </div>
          <div className="stat-item">
            <div className="stat-num">7-Piece</div>
            <div className="stat-label">Syzygy Solved Endgames</div>
          </div>
          <div className="stat-item">
            <div className="stat-num">Glicko-2</div>
            <div className="stat-label">Server Rating System</div>
          </div>
        </div>
      </section>

      {/* Signature Crown Features Bento Grid */}
      <section style={{ maxWidth: "1200px", margin: "0 auto", padding: "0 24px" }}>
        <h2 className="luxury-section-title">Grandmaster Engineering</h2>
        <p className="luxury-section-subtitle">
          Every component is built under The Reality Contract — zero simulated telemetry, zero placeholder math, and 120 FPS kinetic response.
        </p>

        <div className="bento-grid">
          {/* Card 1: Win-Probability Move Hints (Span 2) */}
          <div className="bento-card bento-card-span-2">
            <div className="bento-badge bento-badge-gold">
              <Sparkles size={11} /> Signature Feature
            </div>
            <div className="bento-icon text-accent-gold">
              <Crosshair size={22} />
            </div>
            <h3 className="bento-title">Live Win-Probability Hints on Piece Click</h3>
            <p className="bento-desc">
              Select any piece to instantly illuminate every legal destination square with calibrated win-probability badges (Gold Ring for the master line, Emerald for winning moves, Rose for blunders) and expected opponent refutations.
            </p>
            <div className="tactical-chip-row">
              <span className="tactical-chip tactical-chip-gold">Best Line: 64% Win</span>
              <span className="tactical-chip tactical-chip-emerald">Tactical Win: 58%</span>
              <span className="tactical-chip tactical-chip-rose">Blunder Risk: 28%</span>
            </div>
          </div>

          {/* Card 2: Synapse HUD & Radar (Span 1) */}
          <div className="bento-card">
            <div className="bento-badge bento-badge-cyan">
              <Activity size={11} /> Live Telemetry
            </div>
            <div className="bento-icon text-accent-cyan">
              <Zap size={22} />
            </div>
            <h3 className="bento-title">Synapse HUD & Radar</h3>
            <p className="bento-desc">
              Watch the engine's cognitive brainwave waveform pulse in real time with depth progression, NPS speedometers, and MultiPV candidate hover projections.
            </p>
          </div>

          {/* Card 3: HalfKP NNUE Neural Network (Span 1) */}
          <div className="bento-card">
            <div className="bento-badge bento-badge-emerald">
              <Brain size={11} /> Deep Learning
            </div>
            <div className="bento-icon text-accent-emerald">
              <Brain size={22} />
            </div>
            <h3 className="bento-title">HalfKP NNUE Neural Net</h3>
            <p className="bento-desc">
              40,960 positional features evaluated with an incremental dual accumulator in &lt;10ns, learning from millions of self-play and human master games.
            </p>
          </div>

          {/* Card 4: Teacher Bot & Live Coach (Span 2) */}
          <div className="bento-card bento-card-span-2">
            <div className="bento-badge bento-badge-rose">
              <ShieldAlert size={11} /> Active Coach
            </div>
            <div className="bento-icon text-accent-rose">
              <ShieldAlert size={22} />
            </div>
            <h3 className="bento-title">Live Tactical Coach & Takeback Refutations</h3>
            <p className="bento-desc">
              The instant you make a mistake (eval drop &ge; 100cp), the Teacher Bot breaks down why the move failed, identifies the tactical motif (Fork, Pin, Overloaded Defender), and lets you take back the move with one click.
            </p>
          </div>

          {/* Card 5: 120 FPS Kinetic Physics (Span 1) */}
          <div className="bento-card">
            <div className="bento-badge bento-badge-gold">
              <Sparkles size={11} /> Kinetic UI
            </div>
            <div className="bento-icon text-accent-gold">
              <Volume2 size={22} />
            </div>
            <h3 className="bento-title">Kinetic Drag & Web Audio</h3>
            <p className="bento-desc">
              Velocity-sensitive piece tilt (&plusmn;7&deg;), magnetic snapping, and zero-asset procedural sound synthesis (Tournament Walnut, Ceramic, Tactile).
            </p>
          </div>

          {/* Card 6: Live Multiplayer & Glicko-2 Clocks (Span 2) */}
          <div className="bento-card bento-card-span-2">
            <div className="bento-badge bento-badge-cyan">
              <Globe size={11} /> Real-Time PvP
            </div>
            <div className="bento-icon text-accent-cyan">
              <Clock size={22} />
            </div>
            <h3 className="bento-title">Server-Authoritative Clocks & Glicko-2</h3>
            <p className="bento-desc">
              7 competitive time controls (Bullet, Blitz, Rapid, Unlimited) with millisecond-exact flag detection, draw agreements, disconnect grace periods, and category-separated Glicko-2 rating engine.
            </p>
          </div>
        </div>
      </section>

      {/* Decorative Divider */}
      <div className="luxury-divider" />

      {/* 3 Ergonomic Workspace Modes Showcase */}
      <section style={{ maxWidth: "1200px", margin: "0 auto", padding: "0 24px" }}>
        <h2 className="luxury-section-title">Three Ergonomic Modes</h2>
        <p className="luxury-section-subtitle">
          Seamlessly adapt your workspace for high-intensity bot sparring, deep positional study, or pure Zen focus.
        </p>

        <div className="modes-grid">
          <div className="mode-card">
            <div className="mode-card-header">
              <h3 className="bento-title" style={{ fontSize: "1.2rem", marginBottom: 0 }}>Battle Arena</h3>
              <span className="mode-tag">Default</span>
            </div>
            <p className="bento-desc">
              Full sensory telemetry: Synapse search waveform, ranked MultiPV candidate radar, move-by-move evaluation graph, and bot personality adjustments.
            </p>
          </div>

          <div className="mode-card">
            <div className="mode-card-header">
              <h3 className="bento-title" style={{ fontSize: "1.2rem", marginBottom: 0 }}>Zen Focus</h3>
              <span className="mode-tag">Distraction-Free</span>
            </div>
            <p className="bento-desc">
              An oversized 78vh board with floating glass clocks. All telemetry is tucked away so you can focus entirely on pure calculation and tactical geometry.
            </p>
          </div>

          <div className="mode-card">
            <div className="mode-card-header">
              <h3 className="bento-title" style={{ fontSize: "1.2rem", marginBottom: 0 }}>Tactical Studio</h3>
              <span className="mode-tag">Deep Research</span>
            </div>
            <p className="bento-desc">
              Deep evaluation math breakdown: piece-square tables, pawn structure tension, king safety attack rings, game phase transitions, and material balance.
            </p>
          </div>
        </div>
      </section>

      {/* Decorative Divider */}
      <div className="luxury-divider" />

      {/* Explainer Section */}
      <section className="luxury-explainer-section">
        <h2 className="luxury-section-title">
          How Axiorynth Calculates Moves
        </h2>
        <p className="luxury-section-subtitle">
          Inside the search architecture that processes millions of nodes per second.
        </p>

        <div className="luxury-steps-layout">
          {/* Step 1 */}
          <div className="luxury-step-card">
            <div className="luxury-step-num">I</div>
            <h4 className="luxury-step-title">Dynamic Move Ordering</h4>
            <p className="luxury-step-desc">
              Before searching, moves are prioritized using the Transposition Table cache, Static Exchange Evaluation (SEE), and killer/history heuristics for maximum beta cutoffs.
            </p>
          </div>

          {/* Step 2 */}
          <div className="luxury-step-card">
            <div className="luxury-step-num">II</div>
            <h4 className="luxury-step-title">Principal Variation Search</h4>
            <p className="luxury-step-desc">
              Traverses the game tree with tight aspiration windows, Null-Move Pruning (NMP), and Late Move Reductions (LMR) to discard millions of mathematically inferior branches.
            </p>
          </div>

          {/* Step 3 */}
          <div className="luxury-step-card">
            <div className="luxury-step-num">III</div>
            <h4 className="luxury-step-title">Quiescence Search</h4>
            <p className="luxury-step-desc">
              Eliminates the 'horizon effect' by extending recursive analysis at leaf nodes for tactical checks, captures, and promotions until a quiet position is achieved.
            </p>
          </div>

          {/* Step 4 */}
          <div className="luxury-step-card">
            <div className="luxury-step-num">IV</div>
            <h4 className="luxury-step-title">Syzygy Tablebases</h4>
            <p className="luxury-step-desc">
              When 7 or fewer pieces remain on the board, the engine instantly queries Syzygy endgame tablebases for perfect mathematical WDL and DTZ conversions.
            </p>
          </div>
        </div>
      </section>

      {/* Grand Call to Action Banner */}
      <section className="grand-cta-section">
        <h2 className="grand-cta-title">Enter the Arena</h2>
        <p className="grand-cta-subtitle">
          Test your calculation against 10 calibrated engine personalities, or climb the Glicko-2 multiplayer ladder.
        </p>
        <div style={{ display: "flex", gap: "16px", justifyContent: "center", flexWrap: "wrap" }}>
          <Link href="/play" className="luxury-btn luxury-btn-gold">
            <Play size={16} fill="currentColor" />
            Play vs Axiorynth Bot
          </Link>
          <Link href="/online" className="luxury-btn luxury-btn-outline">
            <Globe size={16} />
            Join Multiplayer Queue
          </Link>
        </div>
      </section>

      {/* Elite 4-Column Professional Footer */}
      <footer className="luxury-footer-grand">
        <div className="luxury-footer-grid">
          {/* Column 1: Brand & Status */}
          <div className="luxury-footer-brand">
            <div className="luxury-logo-container">
              <div className="luxury-logo-badge">A</div>
              <span className="luxury-logo-text">AXIORYNTH</span>
            </div>
            <p>
              A grandmaster-grade chess ecosystem engineered from scratch in Rust and Next.js with real-time neural search telemetry.
            </p>
            <div className="status-live-indicator">
              <span className="status-live-dot" />
              <span>Production Engine & WebSockets Online</span>
            </div>
          </div>

          {/* Column 2: Product */}
          <div className="luxury-footer-col">
            <h4>Product</h4>
            <ul>
              <li><Link href="/play">Engine Lab</Link></li>
              <li><Link href="/online">Live Multiplayer</Link></li>
              <li><Link href="/play?mode=zen">Zen Focus</Link></li>
              <li><Link href="/play?mode=studio">Tactical Studio</Link></li>
            </ul>
          </div>

          {/* Column 3: Architecture */}
          <div className="luxury-footer-col">
            <h4>Architecture</h4>
            <ul>
              <li><span>64-Bit Bitboards</span></li>
              <li><span>HalfKP NNUE (40,960 Inputs)</span></li>
              <li><span>Syzygy 7-Piece Endgame</span></li>
              <li><span>Axum & Tokio Async</span></li>
            </ul>
          </div>

          {/* Column 4: Standards */}
          <div className="luxury-footer-col">
            <h4>Standards</h4>
            <ul>
              <li><span>The Reality Contract</span></li>
              <li><span>120 FPS Kinetic Budget</span></li>
              <li><span>Glicko-2 Rating Engine</span></li>
              <li><span>Argon2id Security</span></li>
            </ul>
          </div>
        </div>

        {/* Footer Bottom Bar */}
        <div className="luxury-footer-bottom">
          <p>&copy; 2026 Axiorynth. Built for grandmaster chess research and competitive play.</p>
          <div className="luxury-footer-bottom-links">
            <span>Zero Simulation Guaranteed</span>
            <span>MIT / Open Engine</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
