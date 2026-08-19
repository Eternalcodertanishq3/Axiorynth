"use client";

import React, { useMemo } from "react";

interface CapturedMaterialProps {
  fen: string;
  orientation?: "white" | "black";
  isTop?: boolean;
}

const PIECE_VALUES: Record<string, number> = {
  P: 1, N: 3, B: 3, R: 5, Q: 9,
  p: 1, n: 3, b: 3, r: 5, q: 9,
};

const STARTING_COUNT: Record<string, number> = {
  P: 8, N: 2, B: 2, R: 2, Q: 1,
  p: 8, n: 2, b: 2, r: 2, q: 1,
};

const PIECE_SYMBOLS: Record<string, string> = {
  P: "♙", N: "♘", B: "♗", R: "♖", Q: "♕",
  p: "♟", n: "♞", b: "♝", r: "♜", q: "♛",
};

export default function CapturedMaterial({ fen, orientation = "white", isTop = true }: CapturedMaterialProps) {
  const captured = useMemo(() => {
    const boardPart = fen.split(" ")[0] || "";
    const counts: Record<string, number> = {
      P: 0, N: 0, B: 0, R: 0, Q: 0,
      p: 0, n: 0, b: 0, r: 0, q: 0,
    };
    
    for (const char of boardPart) {
      if (counts[char] !== undefined) {
        counts[char]++;
      }
    }
    
    const cap: Record<string, number> = {};
    let whiteScore = 0;
    let blackScore = 0;
    
    for (const p of Object.keys(STARTING_COUNT)) {
      const diff = STARTING_COUNT[p] - (counts[p] || 0);
      cap[p] = diff > 0 ? diff : 0;
      
      const val = cap[p] * PIECE_VALUES[p];
      if (p === p.toUpperCase()) {
        blackScore += val; // Black captures white pieces
      } else {
        whiteScore += val; // White captures black pieces
      }
    }
    
    return { cap, whiteScore, blackScore };
  }, [fen]);
  
  const topColor = orientation === "white" ? "black" : "white";
  const bottomColor = orientation === "white" ? "white" : "black";
  const cColor = isTop ? topColor : bottomColor;
  
  const scoreDiff = cColor === "white" 
    ? captured.whiteScore - captured.blackScore 
    : captured.blackScore - captured.whiteScore;
    
  const piecesToShow = cColor === "white" 
    ? ["p", "n", "b", "r", "q"] 
    : ["P", "N", "B", "R", "Q"];
    
  const renderedPieces: string[] = [];
  piecesToShow.forEach(p => {
    for (let i = 0; i < captured.cap[p]; i++) {
      renderedPieces.push(PIECE_SYMBOLS[p]);
    }
  });

  if (renderedPieces.length === 0 && scoreDiff <= 0) return <div className="h-5" />;

  return (
    <div className="flex items-center gap-2 text-sm text-[#94a3b8] h-5">
      <div className="flex gap-0.5 tracking-tighter">
        {renderedPieces.map((sym, i) => (
          <span key={i} className="text-base leading-none">{sym}</span>
        ))}
      </div>
      {scoreDiff > 0 && (
        <span className="text-xs font-bold bg-white/10 px-1 rounded text-[#e8c468]">
          +{scoreDiff}
        </span>
      )}
    </div>
  );
}
