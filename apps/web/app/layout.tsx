import type { Metadata } from "next";
import type { ReactNode } from "react";
import { Cinzel, Outfit, Plus_Jakarta_Sans, JetBrains_Mono } from 'next/font/google';
import "./globals.css";

const cinzel = Cinzel({ subsets: ['latin'], variable: '--font-cinzel', display: 'swap' });
const outfit = Outfit({ subsets: ['latin'], variable: '--font-outfit', display: 'swap' });
const jakarta = Plus_Jakarta_Sans({ subsets: ['latin'], variable: '--font-jakarta', display: 'swap' });
const jetbrains = JetBrains_Mono({ subsets: ['latin'], variable: '--font-mono', display: 'swap' });

export const metadata: Metadata = {
  title: "Axiorynth | Grandmaster Chess Platform",
  description: "Axiorynth: The Math-First Chess Sandbox. Experience chess powered by a custom Rust selective-search engine.",
  openGraph: {
    title: "Axiorynth | Grandmaster Chess Platform",
    description: "Axiorynth: The Math-First Chess Sandbox.",
  }
};

export default function RootLayout({
  children,
}: Readonly<{
  children: ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${cinzel.variable} ${outfit.variable} ${jakarta.variable} ${jetbrains.variable}`}>
        {children}
      </body>
    </html>
  );
}
