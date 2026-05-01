"use client";

import { useRouter } from "next/navigation";
import { useCallback } from "react";
import { authApi, AuthResponse } from "@/lib/api";
import { storage } from "@/lib/storage";

export function useAuth() {
  const router = useRouter();

  const saveSession = (data: AuthResponse) => {
    storage.setToken(data.token);
    storage.setUserId(data.user_id);
  };

  const register = useCallback(
    async (email: string, password: string) => {
      const { data } = await authApi.register({ email, password });
      saveSession(data);
      router.push("/rooms");
    },
    [router]
  );

  const login = useCallback(
    async (email: string, password: string) => {
      const { data } = await authApi.login({ email, password });
      saveSession(data);
      router.push("/rooms");
    },
    [router]
  );

  const logout = useCallback(() => {
    storage.clear();
    router.push("/login");
  }, [router]);

  return { register, login, logout };
}
