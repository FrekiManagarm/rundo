"use client";

import { useState } from "react";
import { roomsApi, Room } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

interface Props {
  onCreated: (room: Room) => void;
}

export function CreateRoomDialog({ onCreated }: Props) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<"conference" | "stream">("conference");
  const [loading, setLoading] = useState(false);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    try {
      const { data } = await roomsApi.create({ name, kind });
      onCreated(data);
      setOpen(false);
      setName("");
      setKind("conference");
    } finally {
      setLoading(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger>
        <Button>New room</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create a room</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleCreate} className="space-y-4 pt-2">
          <div className="space-y-1">
            <Label htmlFor="room-name">Name</Label>
            <Input
              id="room-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </div>
          <div className="space-y-1">
            <Label htmlFor="room-kind">Type</Label>
            <select
              id="room-kind"
              value={kind}
              onChange={(e) => setKind(e.target.value as "conference" | "stream")}
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
            >
              <option value="conference">Conference</option>
              <option value="stream">Stream</option>
            </select>
          </div>
          <Button type="submit" className="w-full" disabled={loading}>
            {loading ? "Creating…" : "Create"}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
