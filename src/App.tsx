import { BotChatView } from "@/components/bot/BotChatView";
import { SideBarBotPane } from "@/components/bot/SideBarBotPane";
import { useBotActivity } from "@/components/bot/useBotActivity";
import { Toaster } from "@/components/ui/toaster";
import { useState } from "react";

export default function App() {
  const [selected, setSelected] = useState(false);
  useBotActivity();
  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      <aside className="flex w-72 shrink-0 flex-col border-r border-border bg-card/40">
        <div className="border-b border-border px-4 py-4">
          <h1 className="text-lg font-semibold tracking-tight">opendots</h1>
          <p className="mt-1 text-xs text-muted-foreground">Bots and routines</p>
        </div>
        <SideBarBotPane onSelect={() => setSelected(true)} />
      </aside>
      <main className="min-w-0 flex-1">
        <div className={selected ? "h-full" : "hidden"}>
          <BotChatView selected={selected} />
        </div>
        {!selected && (
          <div className="grid h-full place-items-center p-8 text-center">
            <div>
              <h2 className="text-xl font-medium">Your workspace for bots</h2>
              <p className="mt-2 max-w-sm text-sm text-muted-foreground">
                Create or select a bot to start a conversation.
              </p>
            </div>
          </div>
        )}
      </main>
      <Toaster />
    </div>
  );
}
