"use client";

import { useState } from "react";
import Link from "next/link";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export default function LoginPage() {
  const { login } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      await login(email, password);
    } catch {
      setError("Invalid email or password.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="relative flex min-h-screen flex-col items-center justify-center p-4 overflow-hidden bg-background">
      <div className="absolute inset-0 bg-dot-grid pointer-events-none" />
      <div className="absolute inset-0 bg-gradient-to-b from-transparent via-transparent to-background/80 pointer-events-none" />

      <div className="relative flex flex-col items-center w-full max-w-sm">
        <div className="animate-fade-up mb-10 text-center">
          <span className="font-black text-5xl tracking-tight text-primary leading-none">
            rundo
          </span>
          <div className="mt-2 flex items-center justify-center gap-2">
            <div className="h-px w-8 bg-border" />
            <span className="text-[11px] font-mono text-muted-foreground tracking-[0.25em] uppercase">
              video rooms
            </span>
            <div className="h-px w-8 bg-border" />
          </div>
        </div>

        <div className="animate-fade-up-1 w-full rounded-2xl border border-border bg-card shadow-2xl shadow-black/40 overflow-hidden">
          <div className="px-6 pt-6 pb-2">
            <h1 className="text-xl font-bold text-foreground">Sign in</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Enter your credentials to continue
            </p>
          </div>

          <form onSubmit={handleSubmit} className="px-6 pb-6 pt-4 space-y-4">
            <div className="space-y-1.5">
              <Label
                htmlFor="email"
                className="text-xs font-medium text-muted-foreground uppercase tracking-widest"
              >
                Email
              </Label>
              <Input
                id="email"
                type="email"
                autoComplete="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
                className="bg-muted border-border focus:border-primary focus:ring-primary/20 h-10 text-sm"
                placeholder="you@example.com"
              />
            </div>

            <div className="space-y-1.5">
              <Label
                htmlFor="password"
                className="text-xs font-medium text-muted-foreground uppercase tracking-widest"
              >
                Password
              </Label>
              <Input
                id="password"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                className="bg-muted border-border focus:border-primary focus:ring-primary/20 h-10 text-sm"
                placeholder="••••••••"
              />
            </div>

            {error && (
              <p className="text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded-lg px-3 py-2">
                {error}
              </p>
            )}

            <Button
              type="submit"
              className="w-full h-10 font-semibold tracking-wide"
              disabled={loading}
            >
              {loading ? (
                <span className="flex items-center gap-2">
                  <span className="w-3.5 h-3.5 rounded-full border-2 border-current border-t-transparent animate-spin" />
                  Signing in…
                </span>
              ) : (
                "Sign in"
              )}
            </Button>

            <p className="text-center text-sm text-muted-foreground">
              No account?{" "}
              <Link
                href="/register"
                className="text-primary hover:text-primary/80 font-medium transition-colors"
              >
                Register
              </Link>
            </p>
          </form>
        </div>
      </div>
    </main>
  );
}
