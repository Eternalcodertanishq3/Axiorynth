"use client";

import React, { useEffect, useRef } from "react";

interface Star {
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  baseAlpha: number;
  phase: number;
  phaseSpeed: number;
  color: string;
}

interface FloatingPiece {
  x: number;
  y: number;
  vx: number;
  vy: number;
  rot: number;
  vRot: number;
  scale: number;
  glyph: string;
  alpha: number;
}

export default function CosmicBackground() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animId: number;
    let width = (canvas.width = window.innerWidth);
    let height = (canvas.height = window.innerHeight);

    const handleResize = () => {
      if (!canvas) return;
      width = canvas.width = window.innerWidth;
      height = canvas.height = window.innerHeight;
    };
    window.addEventListener("resize", handleResize);

    // Mouse tracking for gentle zero-G parallax
    let targetMouseX = 0;
    let targetMouseY = 0;
    let currentMouseX = 0;
    let currentMouseY = 0;

    const handleMouseMove = (e: MouseEvent) => {
      targetMouseX = (e.clientX - width / 2) * 0.025;
      targetMouseY = (e.clientY - height / 2) * 0.025;
    };
    window.addEventListener("mousemove", handleMouseMove, { passive: true });

    // Generate Stars & Neural Constellation Nodes
    const starCount = Math.min(Math.floor((width * height) / 18000), 75);
    const stars: Star[] = [];
    const colors = ["#e8c468", "#22d3ee", "#ffffff", "#f43f5e"];

    for (let i = 0; i < starCount; i++) {
      stars.push({
        x: Math.random() * width,
        y: Math.random() * height,
        vx: (Math.random() - 0.5) * 0.2,
        vy: (Math.random() - 0.5) * 0.2,
        radius: Math.random() * 1.5 + 0.6,
        baseAlpha: Math.random() * 0.5 + 0.2,
        phase: Math.random() * Math.PI * 2,
        phaseSpeed: Math.random() * 0.02 + 0.008,
        color: colors[Math.floor(Math.random() * colors.length)],
      });
    }

    // Floating Zero-G Chess Glyphs
    const glyphs = ["♚", "♛", "♞", "♝", "♜", "♟"];
    const pieces: FloatingPiece[] = [];
    const pieceCount = 7;

    for (let i = 0; i < pieceCount; i++) {
      pieces.push({
        x: Math.random() * width,
        y: Math.random() * height,
        vx: (Math.random() - 0.5) * 0.12,
        vy: (Math.random() - 0.5) * 0.12,
        rot: Math.random() * Math.PI * 2,
        vRot: (Math.random() - 0.5) * 0.003,
        scale: Math.random() * 26 + 22,
        glyph: glyphs[i % glyphs.length],
        alpha: Math.random() * 0.04 + 0.02,
      });
    }

    let isVisible = true;
    const handleVisibility = () => {
      isVisible = !document.hidden;
    };
    document.addEventListener("visibilitychange", handleVisibility);

    // Main 120 FPS Render Loop
    const render = () => {
      if (!isVisible) {
        animId = requestAnimationFrame(render);
        return;
      }

      ctx.clearRect(0, 0, width, height);

      // Smooth inertia mouse interpolation
      currentMouseX += (targetMouseX - currentMouseX) * 0.04;
      currentMouseY += (targetMouseY - currentMouseY) * 0.04;

      // Draw Floating Chess Pieces
      for (const p of pieces) {
        p.x += p.vx;
        p.y += p.vy;
        p.rot += p.vRot;

        if (p.x < -60) p.x = width + 60;
        if (p.x > width + 60) p.x = -60;
        if (p.y < -60) p.y = height + 60;
        if (p.y > height + 60) p.y = -60;

        ctx.save();
        ctx.translate(p.x + currentMouseX * 0.4, p.y + currentMouseY * 0.4);
        ctx.rotate(p.rot);
        ctx.font = `${p.scale}px sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillStyle = `rgba(232, 196, 104, ${p.alpha})`;
        ctx.fillText(p.glyph, 0, 0);
        ctx.restore();
      }

      // Update & Draw Constellation Stars
      for (let i = 0; i < stars.length; i++) {
        const s = stars[i];
        s.x += s.vx;
        s.y += s.vy;
        s.phase += s.phaseSpeed;

        if (s.x < 0) s.x = width;
        if (s.x > width) s.x = 0;
        if (s.y < 0) s.y = height;
        if (s.y > height) s.y = 0;

        const dynamicAlpha = s.baseAlpha + Math.sin(s.phase) * 0.2;
        const posX = s.x + currentMouseX;
        const posY = s.y + currentMouseY;

        ctx.beginPath();
        ctx.arc(posX, posY, s.radius, 0, Math.PI * 2);
        ctx.fillStyle = s.color;
        ctx.globalAlpha = Math.max(0.05, Math.min(1, dynamicAlpha));
        ctx.shadowBlur = 8;
        ctx.shadowColor = s.color;
        ctx.fill();
        ctx.shadowBlur = 0;
        ctx.globalAlpha = 1;

        // Neural network constellation connector lines
        for (let j = i + 1; j < stars.length; j++) {
          const s2 = stars[j];
          const dx = posX - (s2.x + currentMouseX);
          const dy = posY - (s2.y + currentMouseY);
          const dist = Math.sqrt(dx * dx + dy * dy);

          if (dist < 130) {
            const lineAlpha = (1 - dist / 130) * 0.12;
            ctx.beginPath();
            ctx.moveTo(posX, posY);
            ctx.lineTo(s2.x + currentMouseX, s2.y + currentMouseY);
            ctx.strokeStyle = "#e8c468";
            ctx.globalAlpha = lineAlpha;
            ctx.lineWidth = 0.75;
            ctx.stroke();
            ctx.globalAlpha = 1;
          }
        }
      }

      animId = requestAnimationFrame(render);
    };

    animId = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener("resize", handleResize);
      window.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("visibilitychange", handleVisibility);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="pointer-events-none fixed inset-0 z-0 h-full w-full"
      style={{ opacity: 0.85 }}
    />
  );
}
