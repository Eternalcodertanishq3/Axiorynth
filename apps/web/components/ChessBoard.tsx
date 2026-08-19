"use client";

import React, { useEffect, useRef, useMemo, useState, useCallback } from "react";
import {
  ChessKing,
  ChessQueen,
  ChessRook,
  ChessBishop,
  ChessKnight,
  ChessPawn,
} from "lucide-react";

const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
const START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

export function isKingInCheck(board: Record<string, string>, fen: string): boolean {
  const parts = fen.split(" ");
  const sideToMove = parts[1] || "w";
  const kingChar = sideToMove === "w" ? "K" : "k";
  
  // 1. Find king position
  let kingSq = "";
  for (const [sq, piece] of Object.entries(board)) {
    if (piece === kingChar) {
      kingSq = sq;
      break;
    }
  }
  if (!kingSq) return false;
  
  const kingFile = FILES.indexOf(kingSq[0]);
  const kingRank = parseInt(kingSq[1], 10);
  
  const enemyColor = sideToMove === "w" ? "black" : "white";
  const isEnemy = (p: string) => {
    return enemyColor === "white" ? p === p.toUpperCase() : p === p.toLowerCase();
  };
  const getPiece = (f: number, r: number) => {
    if (f < 0 || f > 7 || r < 1 || r > 8) return null;
    return board[`${FILES[f]}${r}`] || null;
  };
  
  // 2. Check Knight attacks
  const knightOffsets = [
    [-2, -1], [-2, 1], [-1, -2], [-1, 2],
    [1, -2], [1, 2], [2, -1], [2, 1]
  ];
  const enemyKnight = sideToMove === "w" ? "n" : "N";
  for (const [df, dr] of knightOffsets) {
    if (getPiece(kingFile + df, kingRank + dr) === enemyKnight) {
      return true;
    }
  }
  
  // 3. Check Pawn attacks
  const enemyPawn = sideToMove === "w" ? "p" : "P";
  const pawnRankDir = sideToMove === "w" ? 1 : -1;
  if (getPiece(kingFile - 1, kingRank + pawnRankDir) === enemyPawn) return true;
  if (getPiece(kingFile + 1, kingRank + pawnRankDir) === enemyPawn) return true;
  
  // 4. Check King attacks
  const enemyKing = sideToMove === "w" ? "k" : "K";
  const kingDirections = [
    [-1, -1], [-1, 0], [-1, 1],
    [0, -1],           [0, 1],
    [1, -1],  [1, 0],  [1, 1]
  ];
  for (const [df, dr] of kingDirections) {
    if (getPiece(kingFile + df, kingRank + dr) === enemyKing) {
      return true;
    }
  }
  
  // 5. Diagonal sliding attacks (Bishop / Queen)
  const diagDirs = [
    [-1, -1], [-1, 1], [1, -1], [1, 1]
  ];
  const enemyBishop = sideToMove === "w" ? "b" : "B";
  const enemyQueen = sideToMove === "w" ? "q" : "Q";
  for (const [df, dr] of diagDirs) {
    let step = 1;
    while (true) {
      const f = kingFile + df * step;
      const r = kingRank + dr * step;
      if (f < 0 || f > 7 || r < 1 || r > 8) break;
      const p = getPiece(f, r);
      if (p) {
        if (isEnemy(p) && (p === enemyBishop || p === enemyQueen)) {
          return true;
        }
        break;
      }
      step++;
    }
  }
  
  // 6. Orthogonal sliding attacks (Rook / Queen)
  const orthoDirs = [
    [-1, 0], [1, 0], [0, -1], [0, 1]
  ];
  const enemyRook = sideToMove === "w" ? "r" : "R";
  for (const [df, dr] of orthoDirs) {
    let step = 1;
    while (true) {
      const f = kingFile + df * step;
      const r = kingRank + dr * step;
      if (f < 0 || f > 7 || r < 1 || r > 8) break;
      const p = getPiece(f, r);
      if (p) {
        if (isEnemy(p) && (p === enemyRook || p === enemyQueen)) {
          return true;
        }
        break;
      }
      step++;
    }
  }
  
  return false;
}

export type BoardThemeId = "gold" | "emerald" | "midnight" | "cyberpunk" | "wood";
export type AcousticProfile = "walnut" | "ceramic" | "mechanical";

export type BoardTheme = {
  id: BoardThemeId;
  name: string;
  light: string;
  dark: string;
  selected: string;
  lastMove: string;
  target: string;
  kingCheck: string;
  boardBorder: string;
  coordinateLight: string;
  coordinateDark: string;
};

export const BOARD_THEMES: Record<BoardThemeId, BoardTheme> = {
  emerald: {
    id: "emerald",
    name: "Tournament Emerald",
    light: "#ebecd0",
    dark: "#739552",
    selected: "rgba(247, 236, 125, 0.65)",
    lastMove: "rgba(186, 202, 68, 0.55)",
    target: "rgba(35, 45, 25, 0.35)",
    kingCheck: "rgba(239, 68, 68, 0.7)",
    boardBorder: "#2a371e",
    coordinateLight: "#739552",
    coordinateDark: "#ebecd0",
  },
  wood: {
    id: "wood",
    name: "Warm Walnut",
    light: "#f0d9b5",
    dark: "#b58863",
    selected: "rgba(230, 180, 80, 0.6)",
    lastMove: "rgba(215, 140, 60, 0.5)",
    target: "rgba(50, 30, 15, 0.35)",
    kingCheck: "rgba(220, 38, 38, 0.7)",
    boardBorder: "#462e19",
    coordinateLight: "#8a5a36",
    coordinateDark: "#f0d9b5",
  },
  gold: {
    id: "gold",
    name: "Royal Slate",
    light: "#dee3e6",
    dark: "#4b6480",
    selected: "rgba(232, 196, 104, 0.65)",
    lastMove: "rgba(232, 196, 104, 0.4)",
    target: "rgba(15, 23, 42, 0.35)",
    kingCheck: "rgba(244, 63, 94, 0.75)",
    boardBorder: "#1e293b",
    coordinateLight: "#4b6480",
    coordinateDark: "#dee3e6",
  },
  midnight: {
    id: "midnight",
    name: "Midnight Ocean",
    light: "#e2e8f0",
    dark: "#334155",
    selected: "rgba(56, 189, 248, 0.6)",
    lastMove: "rgba(14, 165, 233, 0.45)",
    target: "rgba(15, 23, 42, 0.4)",
    kingCheck: "rgba(244, 63, 94, 0.75)",
    boardBorder: "#0f172a",
    coordinateLight: "#334155",
    coordinateDark: "#f8fafc",
  },
  cyberpunk: {
    id: "cyberpunk",
    name: "Cyber Neon",
    light: "#818cf8",
    dark: "#1e1b4b",
    selected: "rgba(244, 63, 94, 0.65)",
    lastMove: "rgba(34, 211, 238, 0.5)",
    target: "rgba(34, 211, 238, 0.45)",
    kingCheck: "rgba(244, 63, 94, 0.85)",
    boardBorder: "#180828",
    coordinateLight: "#312e81",
    coordinateDark: "#c7d2fe",
  },
};

// ==========================================
// AUTHENTIC STAUNTON CHESS PIECE SVG VECTORS
// ==========================================

function SvgWhitePawn() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <path
        d="M 22.5,9 C 19.8,9 18.5,10.8 18.5,13 C 18.5,14 18.8,14.8 19.3,15.4 C 17.3,16.5 16,18.6 16,21 C 16,23 16.9,24.8 18.4,26 C 15.4,27.1 11,31.6 11,39.5 L 34,39.5 C 34,31.6 29.6,27.1 26.6,26 C 28.1,24.8 29,23 29,21 C 29,18.6 27.7,16.5 25.7,15.4 C 26.2,14.8 26.5,14 26.5,13 C 26.5,10.8 25.2,9 22.5,9 z"
        style={{ fill: "#ffffff", stroke: "#1c1917", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}
      />
    </svg>
  );
}

function SvgBlackPawn() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <path
        d="M 22.5,9 C 19.8,9 18.5,10.8 18.5,13 C 18.5,14 18.8,14.8 19.3,15.4 C 17.3,16.5 16,18.6 16,21 C 16,23 16.9,24.8 18.4,26 C 15.4,27.1 11,31.6 11,39.5 L 34,39.5 C 34,31.6 29.6,27.1 26.6,26 C 28.1,24.8 29,23 29,21 C 29,18.6 27.7,16.5 25.7,15.4 C 26.2,14.8 26.5,14 26.5,13 C 26.5,10.8 25.2,9 22.5,9 z"
        style={{ fill: "#262830", stroke: "#000000", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}
      />
      <path
        d="M 13.5,38.5 L 31.5,38.5"
        style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2, strokeLinecap: "round" }}
      />
    </svg>
  );
}

function SvgWhiteKnight() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "none", stroke: "#1c1917", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 22,10 C 32.5,11 38.5,18 38,39 L 15,39 C 15,30 25,32.5 23,18" style={{ fill: "#ffffff", stroke: "#1c1917" }} />
        <path d="M 24,18 C 24.38,20.91 18.45,25.37 16,27 C 13,29 13.18,31.34 11,31 C 9.96,30.06 12.41,27.96 11,28 C 10,28 11.19,29.23 10,30 C 9,30 6,31 6,26 C 6,24 12,14 12,14 C 12,14 13.89,12.1 14,10.5 C 13.27,9.51 13.5,8.5 13.5,7.5 C 14.5,6.5 16.5,10 16.5,10 L 18.5,10 C 18.5,10 19.28,8 21,7 C 22,7 22,10 22,10" style={{ fill: "#ffffff", stroke: "#1c1917" }} />
        <circle cx="9.5" cy="25.5" r="0.8" style={{ fill: "#1c1917", stroke: "#1c1917" }} />
        <path d="M 15,15.5 C 15,15.5 18,17.5 18,20.5" style={{ fill: "none", stroke: "#1c1917" }} />
      </g>
    </svg>
  );
}

function SvgBlackKnight() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "none", stroke: "#000000", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 22,10 C 32.5,11 38.5,18 38,39 L 15,39 C 15,30 25,32.5 23,18" style={{ fill: "#262830", stroke: "#000000" }} />
        <path d="M 24,18 C 24.38,20.91 18.45,25.37 16,27 C 13,29 13.18,31.34 11,31 C 9.96,30.06 12.41,27.96 11,28 C 10,28 11.19,29.23 10,30 C 9,30 6,31 6,26 C 6,24 12,14 12,14 C 12,14 13.89,12.1 14,10.5 C 13.27,9.51 13.5,8.5 13.5,7.5 C 14.5,6.5 16.5,10 16.5,10 L 18.5,10 C 18.5,10 19.28,8 21,7 C 22,7 22,10 22,10" style={{ fill: "#262830", stroke: "#000000" }} />
        <circle cx="9.5" cy="25.5" r="0.8" style={{ fill: "#ffffff", stroke: "#ffffff" }} />
        <path d="M 15,15.5 C 15,15.5 18,17.5 18,20.5" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2 }} />
      </g>
    </svg>
  );
}

function SvgWhiteBishop() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "none", stroke: "#1c1917", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 9,36 C 12.39,35.03 19.11,36.43 22.5,34 C 25.89,36.43 32.61,35.03 36,36 C 36,36 37.65,36.54 39,38 C 38.32,38.97 37.35,38.99 36,38.5 C 32.61,37.53 25.89,38.96 22.5,37.5 C 19.11,38.96 12.39,37.53 9,38.5 C 7.65,38.99 6.68,38.97 6,38 C 7.35,36.54 9,36 9,36 z" style={{ fill: "#ffffff", stroke: "#1c1917" }} />
        <path d="M 12,36 C 12.27,34.88 13.29,32.84 15,31 C 18,28 17.75,25.5 16,21 C 14.5,17 17.5,13.5 22.5,13.5 C 27.5,13.5 30.5,17 29,21 C 27.25,25.5 27,28 30,31 C 31.71,32.84 32.73,34.88 33,36 C 29.61,35.03 22.89,36.43 19.5,34 C 16.11,36.43 9.39,35.03 6,36" style={{ fill: "#ffffff", stroke: "#1c1917" }} />
        <circle cx="22.5" cy="10" r="2.5" style={{ fill: "#ffffff", stroke: "#1c1917" }} />
        <path d="M 17.5,26 L 27.5,26 M 15,30 L 30,30 M 22.5,15.5 L 22.5,20.5 M 20,18 L 25,18" style={{ fill: "none", stroke: "#1c1917" }} />
      </g>
    </svg>
  );
}

function SvgBlackBishop() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "none", stroke: "#000000", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 9,36 C 12.39,35.03 19.11,36.43 22.5,34 C 25.89,36.43 32.61,35.03 36,36 C 36,36 37.65,36.54 39,38 C 38.32,38.97 37.35,38.99 36,38.5 C 32.61,37.53 25.89,38.96 22.5,37.5 C 19.11,38.96 12.39,37.53 9,38.5 C 7.65,38.99 6.68,38.97 6,38 C 7.35,36.54 9,36 9,36 z" style={{ fill: "#262830", stroke: "#000000" }} />
        <path d="M 12,36 C 12.27,34.88 13.29,32.84 15,31 C 18,28 17.75,25.5 16,21 C 14.5,17 17.5,13.5 22.5,13.5 C 27.5,13.5 30.5,17 29,21 C 27.25,25.5 27,28 30,31 C 31.71,32.84 32.73,34.88 33,36 C 29.61,35.03 22.89,36.43 19.5,34 C 16.11,36.43 9.39,35.03 6,36" style={{ fill: "#262830", stroke: "#000000" }} />
        <circle cx="22.5" cy="10" r="2.5" style={{ fill: "#262830", stroke: "#000000" }} />
        <path d="M 17.5,26 L 27.5,26 M 15,30 L 30,30 M 22.5,15.5 L 22.5,20.5 M 20,18 L 25,18" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2 }} />
      </g>
    </svg>
  );
}

function SvgWhiteRook() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "#ffffff", stroke: "#1c1917", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 9,39 L 36,39 L 36,36 L 9,36 L 9,39 z" />
        <path d="M 12,36 L 12,32 L 33,32 L 33,36 L 12,36 z" />
        <path d="M 11,14 L 11,9 L 15,9 L 15,11 L 20,11 L 20,9 L 25,9 L 25,11 L 30,11 L 30,9 L 34,9 L 34,14" />
        <path d="M 34,14 L 31,17 L 14,17 L 11,14" />
        <path d="M 14,17 L 14,29.5 L 31,29.5 L 31,17" />
        <path d="M 14,29.5 L 11,32 L 34,32 L 31,29.5" />
      </g>
    </svg>
  );
}

function SvgBlackRook() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "#262830", stroke: "#000000", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 9,39 L 36,39 L 36,36 L 9,36 L 9,39 z" />
        <path d="M 12,36 L 12,32 L 33,32 L 33,36 L 12,36 z" />
        <path d="M 11,14 L 11,9 L 15,9 L 15,11 L 20,11 L 20,9 L 25,9 L 25,11 L 30,11 L 30,9 L 34,9 L 34,14" />
        <path d="M 34,14 L 31,17 L 14,17 L 11,14" />
        <path d="M 14,17 L 14,29.5 L 31,29.5 L 31,17" />
        <path d="M 14,29.5 L 11,32 L 34,32 L 31,29.5" />
        <path d="M 12,35.5 L 33,35.5 M 13,31.5 L 32,31.5 M 14,29.5 L 31,29.5 M 14,16.5 L 31,16.5 M 11,14 L 34,14" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2 }} />
      </g>
    </svg>
  );
}

function SvgWhiteQueen() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "#ffffff", stroke: "#1c1917", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 9,26 C 17.5,24.5 30,24.5 36,26 L 38.5,13.5 L 31,25 L 30.7,10.5 L 25.5,24.5 L 22.5,9.5 L 19.5,24.5 L 14.3,10.5 L 14,25 L 6.5,13.5 L 9,26 z" />
        <path d="M 9,26 C 9,28 10.5,28 11.5,30 C 12.5,31.5 12.5,31 12,33.5 C 10.5,34.5 11,36 11,36 C 12,36 12,36 12,36 C 13,35 14,35 15,35 C 16,35 17,35 18,35 C 19,35 20,35 21,35 C 22,35 23,35 24,35 C 25,35 26,35 27,35 C 28,35 29,35 30,35 C 31,35 32,35 33,36 C 33,36 33.5,34.5 32,33.5 C 31.5,31 31.5,31.5 32.5,30 C 33.5,28 35,28 35,26 C 35,26 9,26 9,26 z" />
        <path d="M 11,38.5 A 35,35 1 0 0 34,38.5 L 36,40 L 9,40 L 11,38.5 z" />
        <circle cx="6" cy="12" r="2" />
        <circle cx="14" cy="9" r="2" />
        <circle cx="22.5" cy="7.5" r="2" />
        <circle cx="31" cy="9" r="2" />
        <circle cx="39" cy="12" r="2" />
      </g>
    </svg>
  );
}

function SvgBlackQueen() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "#262830", stroke: "#000000", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 9,26 C 17.5,24.5 30,24.5 36,26 L 38.5,13.5 L 31,25 L 30.7,10.5 L 25.5,24.5 L 22.5,9.5 L 19.5,24.5 L 14.3,10.5 L 14,25 L 6.5,13.5 L 9,26 z" />
        <path d="M 9,26 C 9,28 10.5,28 11.5,30 C 12.5,31.5 12.5,31 12,33.5 C 10.5,34.5 11,36 11,36 C 12,36 12,36 12,36 C 13,35 14,35 15,35 C 16,35 17,35 18,35 C 19,35 20,35 21,35 C 22,35 23,35 24,35 C 25,35 26,35 27,35 C 28,35 29,35 30,35 C 31,35 32,35 33,36 C 33,36 33.5,34.5 32,33.5 C 31.5,31 31.5,31.5 32.5,30 C 33.5,28 35,28 35,26 C 35,26 9,26 9,26 z" />
        <path d="M 11,38.5 A 35,35 1 0 0 34,38.5 L 36,40 L 9,40 L 11,38.5 z" />
        <circle cx="6" cy="12" r="2" />
        <circle cx="14" cy="9" r="2" />
        <circle cx="22.5" cy="7.5" r="2" />
        <circle cx="31" cy="9" r="2" />
        <circle cx="39" cy="12" r="2" />
        <path d="M 9,26 C 17.5,24.5 30,24.5 36,26 M 11.5,30 C 15,29 30,29 33.5,30 M 12,33.5 C 18,32.5 27,32.5 33,33.5" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2 }} />
      </g>
    </svg>
  );
}

function SvgWhiteKing() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "none", stroke: "#1c1917", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 22.5,11.63 L 22.5,6" style={{ fill: "none", stroke: "#1c1917" }} />
        <path d="M 20,8 L 25,8" style={{ fill: "none", stroke: "#1c1917" }} />
        <path d="M 22.5,25 C 22.5,25 27,17.5 25.5,14.5 C 24,11.5 21,11.5 22.5,8.5 C 24,11.5 21,11.5 19.5,14.5 C 18,17.5 22.5,25 22.5,25" style={{ fill: "#ffffff", stroke: "#1c1917" }} />
        <path d="M 11.5,37 C 17,40.5 28,40.5 33.5,37 C 36.5,34 35,27 35,27 C 35,27 30.5,24 22.5,24 C 14.5,24 10,27 10,27 C 10,27 8.5,34 11.5,37 z" style={{ fill: "#ffffff", stroke: "#1c1917" }} />
        <path d="M 11.5,30 C 17,27 28,27 33.5,30" style={{ fill: "none", stroke: "#1c1917" }} />
        <path d="M 11.5,33.5 C 17,30.5 28,30.5 33.5,33.5" style={{ fill: "none", stroke: "#1c1917" }} />
        <path d="M 11.5,37 C 17,34 28,34 33.5,37" style={{ fill: "none", stroke: "#1c1917" }} />
      </g>
    </svg>
  );
}

function SvgBlackKing() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 45 45" style={{ width: "100%", height: "100%" }}>
      <g style={{ fill: "none", stroke: "#000000", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round" }}>
        <path d="M 22.5,11.63 L 22.5,6" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.5 }} />
        <path d="M 20,8 L 25,8" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.5 }} />
        <path d="M 22.5,25 C 22.5,25 27,17.5 25.5,14.5 C 24,11.5 21,11.5 22.5,8.5 C 24,11.5 21,11.5 19.5,14.5 C 18,17.5 22.5,25 22.5,25" style={{ fill: "#262830", stroke: "#000000" }} />
        <path d="M 11.5,37 C 17,40.5 28,40.5 33.5,37 C 36.5,34 35,27 35,27 C 35,27 30.5,24 22.5,24 C 14.5,24 10,27 10,27 C 10,27 8.5,34 11.5,37 z" style={{ fill: "#262830", stroke: "#000000" }} />
        <path d="M 11.5,30 C 17,27 28,27 33.5,30" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2 }} />
        <path d="M 11.5,33.5 C 17,30.5 28,30.5 33.5,33.5" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2 }} />
        <path d="M 11.5,37 C 17,34 28,34 33.5,37" style={{ fill: "none", stroke: "#ffffff", strokeWidth: 1.2 }} />
      </g>
    </svg>
  );
}

const PIECE_COMPONENTS: Record<string, React.ComponentType> = {
  P: SvgWhitePawn,
  N: SvgWhiteKnight,
  B: SvgWhiteBishop,
  R: SvgWhiteRook,
  Q: SvgWhiteQueen,
  K: SvgWhiteKing,
  p: SvgBlackPawn,
  n: SvgBlackKnight,
  b: SvgBlackBishop,
  r: SvgBlackRook,
  q: SvgBlackQueen,
  k: SvgBlackKing,
};

export function ChessPiece({ piece, isGhost = false }: { piece: string; isGhost?: boolean }) {
  const Component = PIECE_COMPONENTS[piece];
  if (!Component) return null;

  return (
    <div
      style={{
        width: "82%",
        height: "82%",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        opacity: isGhost ? 0.45 : 1,
        filter: isGhost
          ? "drop-shadow(0 0 8px rgba(34, 211, 238, 0.6))"
          : "drop-shadow(0 2px 5px rgba(0, 0, 0, 0.35))",
        transition: "transform 0.15s cubic-bezier(0.2, 0.8, 0.2, 1)",
        userSelect: "none",
        pointerEvents: "none",
      }}
    >
      <Component />
    </div>
  );
}

// Multi-Profile Acoustic Web Audio Synthesizer
export const playChessSound = (
  type: "move" | "capture" | "check" | "gameover",
  profile: AcousticProfile = "walnut"
) => {
  if (typeof window === "undefined") return;
  try {
    const ctx = new (window.AudioContext || (window as any).webkitAudioContext)();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    
    osc.connect(gain);
    gain.connect(ctx.destination);
    
    const now = ctx.currentTime;
    
    if (type === "move") {
      if (profile === "ceramic") {
        osc.type = "sine";
        osc.frequency.setValueAtTime(460, now);
        osc.frequency.exponentialRampToValueAtTime(220, now + 0.06);
        gain.gain.setValueAtTime(0.14, now);
        gain.gain.exponentialRampToValueAtTime(0.001, now + 0.06);
        osc.start(now);
        osc.stop(now + 0.06);
      } else if (profile === "mechanical") {
        osc.type = "square";
        osc.frequency.setValueAtTime(680, now);
        osc.frequency.exponentialRampToValueAtTime(320, now + 0.04);
        gain.gain.setValueAtTime(0.08, now);
        gain.gain.exponentialRampToValueAtTime(0.001, now + 0.04);
        osc.start(now);
        osc.stop(now + 0.04);
      } else {
        // Tournament Walnut
        osc.type = "triangle";
        osc.frequency.setValueAtTime(220, now);
        osc.frequency.exponentialRampToValueAtTime(90, now + 0.09);
        gain.gain.setValueAtTime(0.18, now);
        gain.gain.exponentialRampToValueAtTime(0.001, now + 0.09);
        osc.start(now);
        osc.stop(now + 0.09);
      }
    } else if (type === "capture") {
      osc.type = "triangle";
      osc.frequency.setValueAtTime(380, now);
      osc.frequency.exponentialRampToValueAtTime(140, now + 0.11);
      gain.gain.setValueAtTime(0.22, now);
      gain.gain.exponentialRampToValueAtTime(0.001, now + 0.11);
      osc.start(now);
      osc.stop(now + 0.11);
    } else if (type === "check") {
      osc.type = "sine";
      osc.frequency.setValueAtTime(520, now);
      osc.frequency.setValueAtTime(740, now + 0.07);
      gain.gain.setValueAtTime(0.12, now);
      gain.gain.exponentialRampToValueAtTime(0.001, now + 0.24);
      osc.start(now);
      osc.stop(now + 0.24);
    } else if (type === "gameover") {
      osc.type = "sawtooth";
      osc.frequency.setValueAtTime(140, now);
      osc.frequency.exponentialRampToValueAtTime(45, now + 0.6);
      gain.gain.setValueAtTime(0.15, now);
      gain.gain.exponentialRampToValueAtTime(0.001, now + 0.6);
      osc.start(now);
      osc.stop(now + 0.6);
    }
  } catch (e) {
    console.error("Web Audio failed:", e);
  }
};

function countFenPieces(fen: string): number {
  const placement = fen.split(" ")[0] ?? "";
  let count = 0;
  for (const char of placement) {
    if (/[a-zA-Z]/.test(char)) count++;
  }
  return count;
}

export type MoveHintData = {
  move: string;
  dest: string;
  score: number;
  winPct: number;
  drawPct?: number;
  lossPct?: number;
  reply?: string;
  depth?: number;
};

interface ChessBoardProps {
  board: Record<string, string>;
  orientation: "white" | "black";
  selectedSquare: string | null;
  targetSquares: Set<string>;
  lastMove: string | null; // e.g. "e2e4"
  onSquareClick: (square: string) => void;
  onMoveDrop?: (from: string, to: string) => void;
  ghostMove?: string | null; // e.g. "g1f3" candidate preview
  hints?: Record<string, MoveHintData>; // key is destination square
  themeId?: BoardThemeId;
  soundProfile?: AcousticProfile;
  showCoordinates?: boolean;
  soundEnabled?: boolean;
  inCheck?: boolean;
  movesCount?: number;
  fen?: string;
  result?: string;
}


function ArrowLine({ from, to, color, markerId, orientation }: { from: string, to: string, color: string, markerId: string, orientation: "white" | "black" }) {
  const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
  
  const getCoords = (sq: string) => {
    const fileIndex = FILES.indexOf(sq[0]);
    const rankNum = Number(sq[1]);
    
    let x = orientation === "white" ? fileIndex : 7 - fileIndex;
    let y = orientation === "white" ? 8 - rankNum : rankNum - 1;
    
    return { x: (x + 0.5) * 12.5, y: (y + 0.5) * 12.5 };
  };
  
  const c1 = getCoords(from);
  const c2 = getCoords(to);
  
  // Shorten the line slightly so it doesn't overlap the marker weirdly
  const dx = c2.x - c1.x;
  const dy = c2.y - c1.y;
  const len = Math.sqrt(dx * dx + dy * dy);
  const scale = (len - 3) / len; // shrink by 3% roughly
  
  const x2 = c1.x + dx * scale;
  const y2 = c1.y + dy * scale;
  
  return (
    <line
      x1={`${c1.x}%`}
      y1={`${c1.y}%`}
      x2={`${x2}%`}
      y2={`${y2}%`}
      stroke={color}
      strokeWidth="1.8%"
      strokeLinecap="round"
      strokeOpacity={0.7}
      markerEnd={markerId}
    />
  );
}

export default function ChessBoard({
  board,
  orientation,
  selectedSquare,
  targetSquares,
  lastMove,
  onSquareClick,
  onMoveDrop,
  ghostMove = null,
  hints,
  themeId = "emerald",
  soundProfile = "walnut",
  showCoordinates = true,
  soundEnabled = true,
  inCheck = false,
  movesCount = 0,
  fen = START_FEN,
  result = "ongoing",
}: ChessBoardProps) {
  const activeTheme = BOARD_THEMES[themeId] || BOARD_THEMES.emerald;
  const squareOrder = useMemo(() => buildSquareOrder(orientation), [orientation]);

  const boardRef = useRef<HTMLDivElement | null>(null);
  const prevMovesCount = useRef(movesCount);
  const prevFen = useRef(fen);
  const prevResult = useRef(result);

  // Drag-and-drop state
  const [draggingSquare, setDraggingSquare] = useState<string | null>(null);
  const [dragPos, setDragPos] = useState<{ x: number; y: number } | null>(null);
  const [hoverSquare, setHoverSquare] = useState<string | null>(null);
  const lastPointerPos = useRef<{ x: number; y: number; time: number }>({ x: 0, y: 0, time: 0 });
  const [tilt, setTilt] = useState(0);
  const [arrows, setArrows] = useState<{from: string, to: string, color: string}[]>([]);
  const [drawingArrow, setDrawingArrow] = useState<{from: string, color: string} | null>(null);

  // Audio triggering
  useEffect(() => {
    if (!soundEnabled) {
      prevMovesCount.current = movesCount;
      prevFen.current = fen;
      prevResult.current = result;
      return;
    }

    if (result !== "ongoing" && prevResult.current === "ongoing") {
      playChessSound("gameover", soundProfile);
    } else if (movesCount > prevMovesCount.current) {
      const checkActive = inCheck || isKingInCheck(board, fen);
      if (checkActive) {
        playChessSound("check", soundProfile);
      } else {
        const oldPieces = countFenPieces(prevFen.current);
        const newPieces = countFenPieces(fen);
        if (newPieces < oldPieces) {
          playChessSound("capture", soundProfile);
        } else {
          playChessSound("move", soundProfile);
        }
      }
    }

    prevMovesCount.current = movesCount;
    prevFen.current = fen;
    prevResult.current = result;
  }, [movesCount, fen, inCheck, result, soundEnabled, soundProfile, board]);

  // King check highlight square
  const kingInCheckSquare = useMemo(() => {
    const checkActive = inCheck || isKingInCheck(board, fen);
    if (!checkActive) return null;
    
    const sideChar = fen.split(" ")[1] ?? "w";
    const kingChar = sideChar === "w" ? "K" : "k";
    for (const [sq, piece] of Object.entries(board)) {
      if (piece === kingChar) return sq;
    }
    return null;
  }, [inCheck, fen, board]);

  // Coordinate lookup for drag position
  const getSquareFromCoords = useCallback(
    (clientX: number, clientY: number): string | null => {
      if (!boardRef.current) return null;
      const rect = boardRef.current.getBoundingClientRect();
      if (
        clientX < rect.left ||
        clientX > rect.right ||
        clientY < rect.top ||
        clientY > rect.bottom
      ) {
        return null;
      }
      const relX = clientX - rect.left;
      const relY = clientY - rect.top;
      const squareSize = rect.width / 8;
      const col = Math.floor(relX / squareSize);
      const row = Math.floor(relY / squareSize);

      if (col < 0 || col > 7 || row < 0 || row > 7) return null;
      const fileIndex = orientation === "white" ? col : 7 - col;
      const rankIndex = orientation === "white" ? 8 - row : 1 + row;
      return `${FILES[fileIndex]}${rankIndex}`;
    },
    [orientation]
  );

  // Pointer drag listeners
const handlePointerDown = (square: string, e: React.PointerEvent) => {
    if (e.button === 0) {
      setArrows([]);
    }
    if (e.button === 2) {
      const color = e.shiftKey ? "#22d3ee" : e.altKey ? "#f43f5e" : "#e8c468";
      setDrawingArrow({ from: square, color });
      return;
    }
    
    const piece = board[square];
    if (!piece) {
      onSquareClick(square);
      return;
    }

    // Check if player owns this piece
    const sideToMove = fen.split(" ")[1] ?? "w";
    const isPieceSide = sideToMove === "w" ? piece === piece.toUpperCase() : piece === piece.toLowerCase();

    if (isPieceSide) {
      setDraggingSquare(square);
      setDragPos({ x: e.clientX, y: e.clientY });
      lastPointerPos.current = { x: e.clientX, y: e.clientY, time: performance.now() };
      setTilt(0);
      onSquareClick(square);
    } else {
      onSquareClick(square);
    }
  };

  useEffect(() => {
    const handlePointerMove = (e: PointerEvent) => {
      if (!draggingSquare) return;
      setDragPos({ x: e.clientX, y: e.clientY });

      // Calculate velocity for kinetic piece tilt
      const now = performance.now();
      const dt = Math.max(now - lastPointerPos.current.time, 1);
      const dx = e.clientX - lastPointerPos.current.x;
      const vx = (dx / dt) * 16;
      setTilt(Math.max(-7, Math.min(7, vx)));
      lastPointerPos.current = { x: e.clientX, y: e.clientY, time: now };

      const sq = getSquareFromCoords(e.clientX, e.clientY);
      setHoverSquare(sq);
    };

const handlePointerUp = (e: PointerEvent) => {
      const targetSq = getSquareFromCoords(e.clientX, e.clientY);
      
      if (drawingArrow) {
        if (targetSq && targetSq !== drawingArrow.from) {
          setArrows(prev => {
            const existing = prev.findIndex(a => a.from === drawingArrow.from && a.to === targetSq);
            if (existing >= 0) {
              const newArrows = [...prev];
              newArrows.splice(existing, 1);
              return newArrows;
            }
            return [...prev, { from: drawingArrow.from, to: targetSq, color: drawingArrow.color }];
          });
        }
        setDrawingArrow(null);
        return;
      }
      
      if (!draggingSquare) return;
      if (targetSq && targetSq !== draggingSquare) {
        if (onMoveDrop) {
          onMoveDrop(draggingSquare, targetSq);
        } else {
          onSquareClick(targetSq);
        }
      }
      setDraggingSquare(null);
      setDragPos(null);
      setHoverSquare(null);
      setTilt(0);
    };

    if (draggingSquare || drawingArrow) {
      window.addEventListener("pointermove", handlePointerMove);
      window.addEventListener("pointerup", handlePointerUp);
      window.addEventListener("pointercancel", handlePointerUp);
    }
    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", handlePointerUp);
    };
  }, [draggingSquare, getSquareFromCoords, onMoveDrop, onSquareClick]);

  return (
    <div
      className="chessboard-wrapper"
      style={{
        display: "grid",
        justifyItems: "center",
        width: "100%",
        userSelect: "none",
        position: "relative",
      }}
    >
      <div
        onContextMenu={(e) => e.preventDefault()}
        ref={boardRef}
        className="chessboard-container luxury-glass"
        style={{
          aspectRatio: "1",
          border: `10px solid ${activeTheme.boardBorder}`,
          borderRadius: "16px",
          boxShadow: "0 24px 64px rgba(0, 0, 0, 0.6), 0 2px 8px rgba(0, 0, 0, 0.3)",
          display: "grid",
          gridTemplateColumns: "repeat(8, 1fr)",
          gridTemplateRows: "repeat(8, 1fr)",
          overflow: "hidden",
          width: "min(100%, 78vh, 760px)",
          position: "relative",
          touchAction: "none",
          transition: "border-color 0.3s ease, box-shadow 0.3s ease",
        }}
      >
        {squareOrder.map((square) => {
          const piece = board[square];
          const fileChar = square[0];
          const rankChar = square[1];
          const fileIndex = FILES.indexOf(fileChar);
          const rankNum = Number(rankChar);
          
          const isDark = (fileIndex + (rankNum - 1)) % 2 === 1;
          const bgBase = isDark ? activeTheme.dark : activeTheme.light;
          
          // Highlights
          const isSelected = selectedSquare === square;
          const isTarget = targetSquares.has(square);
          const isHoveredTarget = hoverSquare === square && isTarget;
          const isLastMoveFrom = lastMove?.startsWith(square);
          const isLastMoveTo = lastMove?.slice(2, 4) === square;
          const isKingCheck = kingInCheckSquare === square;
          const isBeingDragged = draggingSquare === square;

          // Ghost move preview
          const isGhostOrigin = ghostMove?.startsWith(square);
          const isGhostTarget = ghostMove?.slice(2, 4) === square;
          const ghostPiece = isGhostTarget && ghostMove ? board[ghostMove.slice(0, 2)] : null;

          // Coordinates display logic
          let showRank = false;
          let showFile = false;
          if (showCoordinates) {
            if (orientation === "white") {
              showRank = fileChar === "a";
              showFile = rankChar === "1";
            } else {
              showRank = fileChar === "h";
              showFile = rankChar === "8";
            }
          }

          const coordColor = isDark ? activeTheme.coordinateDark : activeTheme.coordinateLight;

          return (
            <button
              key={square}
              onPointerDown={(e) => handlePointerDown(square, e)}
              onClick={() => onSquareClick(square)}
              type="button"
              className="square-button"
              style={{
                alignItems: "center",
                border: "0",
                background: bgBase,
                display: "flex",
                justifyContent: "center",
                width: "100%",
                height: "100%",
                aspectRatio: "1",
                minHeight: "0",
                minWidth: "0",
                padding: "0",
                position: "relative",
                outline: "none",
                cursor: piece ? "grab" : isTarget ? "pointer" : "default",
                userSelect: "none",
                transition: "background-color 0.15s ease",
              }}
            >
              {/* Last move overlay */}
              {(isLastMoveFrom || isLastMoveTo) && (
                <div
                  style={{
                    position: "absolute",
                    inset: 0,
                    backgroundColor: activeTheme.lastMove,
                    pointerEvents: "none",
                  }}
                />
              )}

              {/* Selected highlight overlay */}
              {isSelected && (
                <div
                  style={{
                    position: "absolute",
                    inset: 0,
                    boxShadow: `inset 0 0 0 4px ${activeTheme.selected}`,
                    backgroundColor: activeTheme.selected,
                    pointerEvents: "none",
                    zIndex: 1,
                  }}
                />
              )}

              {/* Check highlight overlay (pulsing red glow) */}
              {isKingCheck && (
                <div
                  className="pulsing-check-overlay"
                  style={{
                    position: "absolute",
                    inset: 0,
                    backgroundColor: activeTheme.kingCheck,
                    pointerEvents: "none",
                    zIndex: 1,
                  }}
                />
              )}

              {/* Target move dot / ring indicator */}
              {isTarget && (
                <div
                  style={{
                    position: "absolute",
                    width: piece ? "84%" : isHoveredTarget ? "38%" : "28%",
                    height: piece ? "84%" : isHoveredTarget ? "38%" : "28%",
                    borderRadius: piece ? "8px" : "999px",
                    border: piece ? `4px solid ${activeTheme.target}` : "none",
                    backgroundColor: piece ? "transparent" : activeTheme.target,
                    boxShadow: isHoveredTarget ? `0 0 16px ${activeTheme.target}` : "none",
                    zIndex: 2,
                    pointerEvents: "none",
                    transition: "all 0.15s cubic-bezier(0.2, 0.8, 0.2, 1)",
                  }}
                />
              )}

              {/* Win-Possibility Hint Badge (Reality Contract) */}
              {hints && hints[square] && (() => {
                const hintList = Object.values(hints);
                const maxScore = hintList.length > 0 ? Math.max(...hintList.map((h) => h.score)) : -Infinity;
                const isBestMove = hints[square]!.score === maxScore;
                const win = hints[square]!.winPct;

                return (
                  <div
                    className="mono-font"
                    style={{
                      position: "absolute",
                      top: piece ? "2px" : "auto",
                      right: piece ? "2px" : "auto",
                      bottom: piece ? "auto" : "2px",
                      left: piece ? "auto" : "auto",
                      zIndex: 4,
                      fontSize: "clamp(0.55rem, 0.9vw, 0.72rem)",
                      fontWeight: 800,
                      padding: "1px 6px",
                      borderRadius: "6px",
                      backgroundColor: isBestMove
                        ? "rgba(232, 196, 104, 0.95)"
                        : win >= 50
                        ? "rgba(15, 23, 42, 0.82)"
                        : "rgba(244, 63, 94, 0.85)",
                      color: isBestMove
                        ? "#08090d"
                        : win >= 50
                        ? "#34d399"
                        : "#ffffff",
                      boxShadow: isBestMove
                        ? "0 0 12px rgba(232, 196, 104, 0.8)"
                        : "0 2px 5px rgba(0,0,0,0.3)",
                      border: isBestMove
                        ? "1.5px solid #ffffff"
                        : win >= 50
                        ? "1px solid rgba(52, 211, 153, 0.3)"
                        : "1px solid rgba(244, 63, 94, 0.3)",
                      pointerEvents: "none",
                      display: "flex",
                      alignItems: "center",
                      gap: "2px",
                    }}
                  >
                    {isBestMove && <span>★</span>}
                    <span>{win}%</span>
                  </div>
                );
              })()}

              {/* Ghost move indicator */}
              {isGhostOrigin && (
                <div
                  style={{
                    position: "absolute",
                    inset: 0,
                    border: "2px dashed var(--accent-cyan)",
                    backgroundColor: "rgba(34, 211, 238, 0.1)",
                    pointerEvents: "none",
                    zIndex: 1,
                  }}
                />
              )}

              {/* Rank coordinate label */}
              {showRank && (
                <span
                  className="mono-font"
                  style={{
                    position: "absolute",
                    top: "4px",
                    left: "5px",
                    fontSize: "clamp(0.48rem, 1vw, 0.68rem)",
                    fontWeight: 700,
                    color: coordColor,
                    pointerEvents: "none",
                    opacity: 0.85,
                  }}
                >
                  {rankChar}
                </span>
              )}

              {/* File coordinate label */}
              {showFile && (
                <span
                  className="mono-font"
                  style={{
                    position: "absolute",
                    bottom: "2px",
                    right: "4px",
                    fontSize: "clamp(0.48rem, 1vw, 0.68rem)",
                    fontWeight: 700,
                    color: coordColor,
                    pointerEvents: "none",
                    opacity: 0.85,
                  }}
                >
                  {fileChar}
                </span>
              )}

              {/* Piece rendering (hidden in square if actively dragging) */}
              {piece && !isBeingDragged ? <ChessPiece piece={piece} /> : null}

              {/* Ghost projected piece */}
              {!piece && ghostPiece ? <ChessPiece piece={ghostPiece} isGhost={true} /> : null}
            </button>
          );
        })}
        {/* SVG Arrows Overlay */}
      <svg
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          width: "100%",
          height: "100%",
          pointerEvents: "none",
          zIndex: 10,
        }}
      >
        <defs>
          <marker id="arrowhead-gold" markerWidth="4" markerHeight="4" refX="2.5" refY="2" orient="auto">
            <polygon points="0 0, 4 2, 0 4" fill="#e8c468" fillOpacity={0.7} />
          </marker>
          <marker id="arrowhead-cyan" markerWidth="4" markerHeight="4" refX="2.5" refY="2" orient="auto">
            <polygon points="0 0, 4 2, 0 4" fill="#22d3ee" fillOpacity={0.7} />
          </marker>
          <marker id="arrowhead-rose" markerWidth="4" markerHeight="4" refX="2.5" refY="2" orient="auto">
            <polygon points="0 0, 4 2, 0 4" fill="#f43f5e" fillOpacity={0.7} />
          </marker>
        </defs>
        {arrows.map((a, i) => {
          const mColor = a.color === "#22d3ee" ? "cyan" : a.color === "#f43f5e" ? "rose" : "gold";
          return <ArrowLine key={i} from={a.from} to={a.to} color={a.color} markerId={`url(#arrowhead-${mColor})`} orientation={orientation} />
        })}
      </svg>
      </div>

      {/* Kinetic Floating Dragged Piece */}
      {draggingSquare && dragPos && board[draggingSquare] && (
        <div
          style={{
            position: "fixed",
            left: dragPos.x,
            top: dragPos.y,
            transform: `translate(-50%, -65%) scale(1.22) rotate(${tilt}deg)`,
            pointerEvents: "none",
            zIndex: 9999,
            width: "80px",
            height: "80px",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            filter: "drop-shadow(0 20px 32px rgba(0,0,0,0.65)) drop-shadow(0 0 12px rgba(232, 196, 104, 0.4))",
            transition: "transform 0.04s ease-out",
          }}
        >
          <ChessPiece piece={board[draggingSquare]} />
        </div>
      )}
    </div>
  );
}

function buildSquareOrder(orientation: "white" | "black") {
  return Array.from({ length: 64 }, (_, index) => {
    const rankOffset = Math.floor(index / 8);
    const fileOffset = index % 8;
    const rank = orientation === "white" ? 8 - rankOffset : 1 + rankOffset;
    const file = orientation === "white" ? FILES[fileOffset] : FILES[7 - fileOffset];
    return `${file}${rank}`;
  });
}
