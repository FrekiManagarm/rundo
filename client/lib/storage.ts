const TOKEN_KEY = "webrtc_token";
const USER_ID_KEY = "webrtc_user_id";

export const storage = {
  getToken: () =>
    typeof window !== "undefined" ? localStorage.getItem(TOKEN_KEY) : null,
  setToken: (token: string) => localStorage.setItem(TOKEN_KEY, token),
  clearToken: () => localStorage.removeItem(TOKEN_KEY),
  getUserId: () =>
    typeof window !== "undefined" ? localStorage.getItem(USER_ID_KEY) : null,
  setUserId: (id: string) => localStorage.setItem(USER_ID_KEY, id),
  clear: () => {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_ID_KEY);
  },
};
