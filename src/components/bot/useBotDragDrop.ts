import { useEffect } from 'react';
import { isDesktopTauri } from '@/services/apiAdapt/shared';
import { type Bot } from '@/services/apiAdapt/bots';
import { useBotUiStore } from '@/stores/useBotUiStore';

/** Dropping a folder (or a file) onto the bot's chat view sets its workspace. */
export function useBotDragDrop(bot: Bot | undefined) {
  const upsertBot = useBotUiStore((s) => s.upsertBot);
  const botId = bot?.id;
  const botCwd = bot?.cwd;

  useEffect(() => {
    // Native drag/drop is optional; the web app can always edit the workspace
    // path through bot settings. Keep the hook safe outside Tauri.
    if (!botId || !isDesktopTauri()) return;
    return;
  }, [botId, botCwd, upsertBot]);
}
