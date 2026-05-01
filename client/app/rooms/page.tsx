"use client";

import { useEffect, useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { roomsApi, Room } from "@/lib/api";
import { storage } from "@/lib/storage";
import { useAuth } from "@/hooks/useAuth";
import { CreateRoomDialog } from "@/components/CreateRoomDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";

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
    <main className="min-h-screen p-6">
      <div className="mx-auto max-w-3xl space-y-6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold">Rooms</h1>
          <div className="flex gap-2">
            <CreateRoomDialog onCreated={handleCreated} />
            <Button variant="outline" onClick={logout}>
              Sign out
            </Button>
          </div>
        </div>
        <Separator />
        {rooms.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No rooms yet. Create one to get started.
          </p>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            {rooms.map((room) => (
              <Card
                key={room.id}
                className="cursor-pointer hover:shadow-md transition-shadow"
                onClick={() => router.push(`/rooms/${room.id}`)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base">{room.name}</CardTitle>
                    <Badge variant="secondary">{room.kind}</Badge>
                  </div>
                </CardHeader>
                <CardContent className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    {room.peer_count} participant
                    {room.peer_count !== 1 ? "s" : ""}
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(room.id);
                    }}
                  >
                    Delete
                  </Button>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </main>
  );
}
