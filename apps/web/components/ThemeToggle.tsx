"use client";

import React, { useEffect, useState } from "react";
import { Sun, Moon } from "lucide-react";

export default function ThemeToggle() {
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    const saved = localStorage.getItem("axiorynth_theme") as "light" | "dark";
    if (saved) {
      setTheme(saved);
      document.documentElement.classList.toggle("dark-theme", saved === "dark");
    } else {
      document.documentElement.classList.remove("dark-theme");
    }
  }, []);

  const toggleTheme = () => {
    const nextTheme = theme === "light" ? "dark" : "light";
    setTheme(nextTheme);
    localStorage.setItem("axiorynth_theme", nextTheme);
    document.documentElement.classList.toggle("dark-theme", nextTheme === "dark");
  };

  if (!mounted) {
    return (
      <div style={{ width: "40px", height: "40px", display: "inline-block" }} />
    );
  }

  return (
    <button
      onClick={toggleTheme}
      className="luxury-btn-outline"
      style={{
        padding: "0",
        borderRadius: "12px",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "0",
        minWidth: "0",
        width: "40px",
        height: "40px",
        cursor: "pointer",
        background: "var(--bg-secondary)",
        border: "1px solid var(--clay-border-color)",
      }}
      title={theme === "light" ? "Switch to Dark Mode" : "Switch to Light Mode"}
    >
      {theme === "light" ? (
        <Moon size={18} color="var(--text-primary)" />
      ) : (
        <Sun size={18} color="var(--text-primary)" />
      )}
    </button>
  );
}
