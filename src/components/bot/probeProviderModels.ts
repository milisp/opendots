import {
  acpAuthenticate,
  acpNewSession,
  acpStart,
  acpStop,
} from '@/services/apiAdapt/acp';
import { captureBotOptions } from '@/stores/useBotOptionsStore';

/**
 * Ask keke which models a provider serves, for a bot that has never run on it.
 *
 * keke reports models per route and only over a live connection, and the route
 * is chosen with `authenticate`, so this spawns a throwaway process, switches
 * it to `provider`, reads the new session's catalogue into the cache and shuts
 * the process down again.
 */
export async function probeProviderModels(provider: string, cwd: string) {
  const started = await acpStart('keke', cwd);
  try {
    await acpAuthenticate(started.connectionId, provider);
    const session = await acpNewSession(started.connectionId, cwd);
    captureBotOptions(provider, started.initialize, session);
  } finally {
    await acpStop(started.connectionId).catch(() => {});
  }
}
