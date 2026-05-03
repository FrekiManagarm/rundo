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
import { Plus } from "lucide-react";

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
      <DialogTrigger render={<Button size="sm" className="gap-1.5 font-semibold" />}>
        <Plus className="w-3.5 h-3.5" />
        New room
      </DialogTrigger>
      <DialogContent className="sm:max-w-md bg-card border-border">
        <DialogHeader>
          <DialogTitle className="text-lg font-bold">Create a room</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleCreate} className="space-y-5 pt-2">
          <div className="space-y-1.5">
            <Label
              htmlFor="room-name"
              className="text-xs font-medium text-muted-foreground uppercase tracking-widest"
            >
              Room name
            </Label>
            <Input
              id="room-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              placeholder="e.g. Design review"
              className="bg-muted border-border focus:border-primary h-10"
            />
          </div>
          <div className="space-y-1.5">
            <Label
              htmlFor="room-kind"
              className="text-xs font-medium text-muted-foreground uppercase tracking-widest"
            >
              Type
            </Label>
            <div className="grid grid-cols-2 gap-2">
              {(["conference", "stream"] as const).map((k) => (
                <button
                  key={k}
                  type="button"
                  onClick={() => setKind(k)}
                  className={`px-3 py-2.5 rounded-lg border text-sm font-semibold transition-all ${
                    kind === k
                      ? "border-primary bg-primary/10 text-primary"
                      : "border-border bg-muted text-muted-foreground hover:border-muted-foreground/40 hover:text-foreground"
                  }`}
                >
                  {k === "conference" ? "Conference" : "Stream"}
                </button>
              ))}
            </div>
          </div>
          <Button
            type="submit"
            className="w-full h-10 font-semibold"
            disabled={loading}
          >
            {loading ? (
              <span className="flex items-center gap-2">
                <span className="w-3.5 h-3.5 rounded-full border-2 border-current border-t-transparent animate-spin" />
                Creating…
              </span>
            ) : (
              "Create room"
            )}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
