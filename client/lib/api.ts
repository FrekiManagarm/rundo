import axios from "axios";
import { storage } from "./storage";

const BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:4000";

export const api = axios.create({ baseURL: BASE_URL });

api.interceptors.request.use((config) => {
  const token = storage.getToken();
  if (token) config.headers.Authorization = `Bearer ${token}`;
  return config;
});

export interface RegisterRequest {
  email: string;
  password: string;
}

export interface LoginRequest {
  email: string;
  password: string;
}

export interface AuthResponse {
  token: string;
  user_id: string;
}

export interface Room {
  id: string;
  name: string;
  kind: "conference" | "stream";
  owner_id: string;
  peer_count: number;
}

export interface CreateRoomRequest {
  name: string;
  kind: "conference" | "stream";
}

export const authApi = {
  register: (body: RegisterRequest) =>
    api.post<AuthResponse>("/auth/register", body),
  login: (body: LoginRequest) =>
    api.post<AuthResponse>("/auth/login", body),
};

export interface IceServer {
  urls: string[];
  username?: string;
  credential?: string;
}

export interface IceConfig {
  ice_servers: IceServer[];
}

export const iceConfigApi = {
  get: () => api.get<IceConfig>("/ice-config"),
};

export const roomsApi = {
  list: () => api.get<Room[]>("/rooms"),
  create: (body: CreateRoomRequest) => api.post<Room>("/rooms", body),
  get: (id: string) => api.get<Room>(`/rooms/${id}`),
  delete: (id: string) => api.delete(`/rooms/${id}`),
};
