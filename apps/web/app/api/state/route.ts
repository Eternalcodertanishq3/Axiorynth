import { execFile } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { promisify } from "node:util";
import { NextRequest, NextResponse } from "next/server";

export const runtime = "nodejs";

const execFileAsync = promisify(execFile);

type StateRequest = {
  moves?: unknown;
  botLevel?: unknown;
  depth?: unknown;
};

export async function POST(request: NextRequest) {
  try {
    const body = (await request.json()) as StateRequest;
    const moves = sanitizeMoves(body.moves);
    const botLevel = clampNumber(body.botLevel, 3, 1, 10);
    const depth = clampNumber(body.depth, 2, 1, 5);
    const repoRoot = findRepoRoot();

    const { stdout } = await execFileAsync(
      "cargo",
      [
        "run",
        "-q",
        "-p",
        "axiorynth_engine",
        "--bin",
        "axiorynth",
        "--",
        "frontend-state",
        "--bot-level",
        String(botLevel),
        "--depth",
        String(depth),
        ...moves,
      ],
      {
        cwd: repoRoot,
        timeout: 120_000,
        maxBuffer: 1024 * 1024 * 8,
        windowsHide: true,
      },
    );

    return NextResponse.json(JSON.parse(stdout));
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown engine error";
    return NextResponse.json({ error: message }, { status: 400 });
  }
}

function sanitizeMoves(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .filter((move): move is string => typeof move === "string")
    .map((move) => move.trim().toLowerCase())
    .filter((move) => /^[a-h][1-8][a-h][1-8][qrbn]?$/.test(move));
}

function clampNumber(value: unknown, fallback: number, min: number, max: number) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    return fallback;
  }
  return Math.min(max, Math.max(min, Math.trunc(numeric)));
}

function findRepoRoot() {
  const candidates = [
    process.env.AXIORYNTH_ROOT,
    path.resolve(process.cwd(), "..", ".."),
    process.cwd(),
  ].filter(Boolean) as string[];

  const root = candidates.find(
    (candidate) =>
      fs.existsSync(path.join(candidate, "Cargo.toml")) &&
      fs.existsSync(path.join(candidate, "engine", "Cargo.toml")),
  );

  if (!root) {
    throw new Error("Could not locate the Axiorynth Rust workspace");
  }

  return root;
}
