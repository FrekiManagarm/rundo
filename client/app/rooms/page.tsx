"use client";

import { useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { roomsApi, Room } from "@/lib/api";
import { storage } from "@/lib/storage";
import { useAuth } from "@/hooks/useAuth";
import { CreateRoomDialog } from "@/components/CreateRoomDialog";
import { Button } from "@/components/ui/button";
import { LogOut, Radio, Users, Trash2 } from "lucide-react";

export default function RoomsPage() {
  const router = useRouter();
  const { logout } = useAuth();
  const [rooms, setRooms] = useState<Room[]>([]);

  const fetchRooms = useCallback(async () => {
    try {
      const { data } = await roomsApi.list();
      setRooms(data);
    } catch {
      router.push("/login");
    }
  }, [router]);

  useEffect(() => {
    if (!storage.getToken()) {
      router.push("/login");
      return;
    }
    fetchRooms();
  }, [fetchRooms, router]);

  function handleCreated(room: Room) {
    setRooms((prev) => [room, ...prev]);
  }

  async function handleDelete(id: string) {
    await roomsApi.delete(id);
    setRooms((prev) => prev.filter((r) => r.id !== id));
  }

  return (
    <div className="relative min-h-screen bg-background overflow-hidden">
      <div className="absolute inset-0 bg-dot-grid pointer-events-none opacity-60" />

      <div className="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-primary/50 to-transparent" />

      <div className="relative">
        <header className="border-b border-border/60 bg-background/80 backdrop-blur-sm sticky top-0 z-10">
          <div className="mx-auto max-w-5xl px-6 h-14 flex items-center justify-between">
            <span className="font-black text-2xl tracking-tight text-primary">
              rundo
            </span>
            <div className="flex items-center gap-2">
              <CreateRoomDialog onCreated={handleCreated} />
              <Button
                variant="ghost"
                size="sm"
                onClick={logout}
                className="gap-1.5 text-muted-foreground hover:text-foreground"
              >
                <LogOut className="w-3.5 h-3.5" />
                Sign out
              </Button>
            </div>
          </div>
        </header>

        <main className="mx-auto max-w-5xl px-6 py-10">
          <div className="animate-fade-up flex items-end justify-between mb-8">
            <div>
              <p className="text-xs font-mono text-muted-foreground tracking-[0.2em] uppercase mb-1">
                Your workspace
              </p>
              <h1 className="text-3xl font-black tracking-tight">Rooms</h1>
            </div>
            {rooms.length > 0 && (
              <span className="text-sm text-muted-foreground font-mono">
                {rooms.length} room{rooms.length !== 1 ? "s" : ""}
              </span>
            )}
          </div>

          {rooms.length === 0 ? (
            <div className="animate-fade-up-1 flex flex-col items-center justify-center py-24 text-center">
              <div className="w-16 h-16 rounded-2xl border-2 border-dashed border-border flex items-center justify-center mb-4">
                <Radio className="w-7 h-7 text-muted-foreground/40" />
              </div>
              <p className="text-base font-semibold text-foreground mb-1">
                No rooms yet
              </p>
              <p className="text-sm text-muted-foreground mb-6 max-w-xs">
                Create your first room to start a video call or stream.
              </p>
              <CreateRoomDialog onCreated={handleCreated} />
            </div>
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {rooms.map((room, i) => (
                <RoomCard
                  key={room.id}
                  room={room}
                  index={i}
                  onDelete={handleDelete}
                  onClick={() => router.push(`/rooms/${room.id}`)}
                />
              ))}
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

function RoomCard({
  room,
  index,
  onDelete,
  onClick,
}: {
  room: Room;
  index: number;
  onDelete: (id: string) => void;
  onClick: () => void;
}) {
  const isStream = room.kind === "stream";
  const delayClass = ["animate-fade-up-1", "animate-fade-up-2", "animate-fade-up-3", "animate-fade-up-4"][
    Math.min(index, 3)
  ];

  return (
    <div
      className={`${delayClass} group relative flex flex-col gap-4 p-5 rounded-xl border border-border bg-card cursor-pointer transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-lg hover:shadow-primary/5 overflow-hidden`}
      onClick={onClick}
    >
      <div
        className={`absolute left-0 top-0 bottom-0 w-0.5 ${
          isStream ? "bg-emerald-500" : "bg-primary"
        }`}
      />

      <div className="flex items-start justify-between gap-3 pl-2">
        <div className="min-w-0 flex-1">
          <h3 className="font-bold text-foreground truncate">{room.name}</h3>
          <span
            className={`text-[10px] font-mono tracking-[0.2em] uppercase mt-0.5 block ${
              isStream ? "text-emerald-500/70" : "text-primary/70"
            }`}
          >
            {room.kind}
          </span>
        </div>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete(room.id);
          }}
          className="opacity-0 group-hover:opacity-100 p-1.5 rounded-md text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-all flex-shrink-0"
          aria-label="Delete room"
        >
          <Trash2 className="w-3.5 h-3.5" />
        </button>
      </div>

      <div className="flex items-center gap-2 pl-2">
        <span
          className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
            room.peer_count > 0
              ? "bg-emerald-400 animate-pulse-live"
              : "bg-muted-foreground/30"
          }`}
        />
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Users className="w-3 h-3" />
          <span>
            {room.peer_count} participant{room.peer_count !== 1 ? "s" : ""}
          </span>
        </div>
      </div>
    </div>
  );
}
