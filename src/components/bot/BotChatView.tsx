import { Settings2 } from "lucide-react";
import { useState } from "react";
import { useAcpEvents } from "@/components/acp/useAcpEvents";
import { Button } from "@/components/ui/button";
import { useAcpStore } from "@/stores/useAcpStore";
import { useBotUiStore } from "@/stores/useBotUiStore";
import { BotAvatar } from "./BotAvatar";
import { BotComposer } from "./BotComposer";
import { BotMessageList } from "./BotMessageList";
import { BotPermissionGate } from "./BotPermissionGate";
import { BotSettingsDialog } from "./BotSettingsDialog";
import { TRUST_LEVELS } from "./botAgentDef";
import { useBotDragDrop } from "./useBotDragDrop";

export function BotChatView({ selected = true }: { selected?: boolean }) {
  const { bots, selectedBotId, connectionByBot } = useBotUiStore();
  const connectionId = useAcpStore((s) => s.connectionId);
  const [settingsOpen, setSettingsOpen] = useState(false);
  useAcpEvents(selected ? connectionId : null);
  const bot = bots.find((item) => item.id === selectedBotId);
  useBotDragDrop(bot);

  if (!bot) return <div className="grid h-full place-items-center text-sm text-muted-foreground">Pick a bot, or create one.</div>;
  const trust = TRUST_LEVELS.find((level) => level.id === bot.trustLevel);
  const running = Boolean(connectionByBot[bot.id]);
  return <div className="flex h-full min-h-0 flex-col">
    <header className="flex shrink-0 items-center gap-3 border-b border-border px-4 py-3">
      <BotAvatar bot={bot} running={running} />
      <div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{bot.name}</div><div className="truncate text-xs text-muted-foreground">{[bot.title, bot.model, trust?.label].filter(Boolean).join(" · ")}</div></div>
      <Button variant="ghost" size="icon" title="Bot settings" onClick={() => setSettingsOpen(true)}><Settings2 className="size-4" /></Button>
    </header>
    <BotMessageList bot={bot} />
    <BotPermissionGate bot={bot} />
    <BotComposer bot={bot} />
    <BotSettingsDialog bot={bot} open={settingsOpen} onOpenChange={setSettingsOpen} />
  </div>;
}
export default BotChatView;
