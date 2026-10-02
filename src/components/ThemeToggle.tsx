import { Monitor, Moon, Sun } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useThemeStore } from '@/stores/useThemeStore';

export function ThemeToggle() {
  const { theme, setTheme } = useThemeStore();

  const icons = {
    light: Sun,
    dark: Moon,
    system: Monitor,
  };

  const labels = {
    light: 'Light',
    dark: 'Dark',
    system: 'System',
  };

  const Icon = icons[theme];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="h-8 w-8" title="Change theme">
          <Icon className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-36">
        {(['light', 'dark', 'system'] as const).map((t) => {
          const ItemIcon = icons[t];
          return (
            <DropdownMenuItem
              key={t}
              onSelect={() => setTheme(t)}
              className={`flex items-center gap-2 ${theme === t ? 'bg-accent' : ''}`}
              data-state={theme === t ? 'checked' : 'unchecked'}
            >
              <ItemIcon className="h-4 w-4" />
              {labels[t]}
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}