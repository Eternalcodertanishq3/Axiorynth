"use client";

import React, { useState, useEffect, useRef } from "react";

interface ClockDisplayProps {
  whiteMs: number;
  blackMs: number;
  activeColor: "white" | "black" | null;
  orientation?: "white" | "black";
  whiteName?: string;
  blackName?: string;
  increment?: number;
  isTop?: boolean;
}

function formatTime(ms: number) {
  if (ms >= 900000000) return "∞";
  if (ms <= 0) return "0:00";
  const secondsTotal = Math.ceil(ms / 1000);
  
  const m = Math.floor(secondsTotal / 60);
  const s = Math.floor(secondsTotal % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export function Clock({
  ms,
  isActive,
}: {
  ms: number;
  isActive: boolean;
  name?: string;
  increment?: number;
}) {
  const isUnlimited = ms >= 900000000;
  const sec = ms / 1000;
  const isWarning = !isUnlimited && sec < 30 && sec >= 10;
  const isCritical = !isUnlimited && sec < 10 && sec > 0;
  const isFlagged = !isUnlimited && ms <= 0;

  let clockColor = "var(--ink-1)";
  if (isActive) {
    clockColor = "var(--accent-gold)";
  }
  if (isWarning) clockColor = "#f59e0b"; // Amber
  if (isCritical || isFlagged) clockColor = "#f43f5e"; // Rose

  return (
    <div
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "6px 14px",
        borderRadius: "var(--radius-md)",
        background: isActive ? "var(--surface-3)" : "var(--surface-1)",
        border: isActive ? `1.5px solid ${clockColor}` : "1px solid var(--surface-glass-border)",
        boxShadow: isActive ? "0 0 12px rgba(232, 196, 104, 0.25)" : "var(--shadow-sm)",
        transition: "all var(--duration-fast) var(--ease-kinetic)",
        minWidth: "90px",
      }}
    >
      <span
        style={{
          fontFamily: "var(--font-mono), monospace",
          fontSize: "1.25rem",
          fontWeight: 800,
          letterSpacing: "0.05em",
          color: clockColor,
          animation: isCritical ? "pulse 0.5s infinite alternate" : isWarning ? "pulse 1s infinite alternate" : "none",
        }}
      >
        {formatTime(ms)}
      </span>
    </div>
  );
}

export default function ClockDisplay({
  whiteMs,
  blackMs,
  activeColor,
  orientation = "white",
  isTop = true,
}: ClockDisplayProps) {
  const topColor = orientation === "white" ? "black" : "white";
  const bottomColor = orientation === "white" ? "white" : "black";
  
  const cColor = isTop ? topColor : bottomColor;
  const ms = cColor === "white" ? whiteMs : blackMs;
  const isActive = activeColor === cColor;

  return (
    <Clock
      ms={ms}
      isActive={isActive}
    />
  );
}
