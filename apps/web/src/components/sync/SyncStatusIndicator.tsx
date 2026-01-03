/**
 * Sync Status Indicator
 *
 * Displays the current WebSocket connection state as a colored dot with tooltip.
 * Used in the header to show users their cross-device sync status.
 */

import * as Tooltip from '@radix-ui/react-tooltip';
import { useDeviceStore } from '../../stores/deviceStore';
import type { ConnectionState } from '../../sync/types';
import { cn } from '../../lib/utils';

interface StatusConfig {
  color: string;
  label: string;
  animate: boolean;
}

const STATUS_CONFIG: Record<ConnectionState, StatusConfig> = {
  connected: {
    color: 'bg-mint',
    label: 'Connected',
    animate: false,
  },
  connecting: {
    color: 'bg-navy',
    label: 'Connecting...',
    animate: true,
  },
  reconnecting: {
    color: 'bg-warning-text',
    label: 'Reconnecting...',
    animate: true,
  },
  disconnected: {
    color: 'bg-error-text/60',
    label: 'Disconnected',
    animate: false,
  },
};

export function SyncStatusIndicator(): JSX.Element {
  const connectionState = useDeviceStore((s) => s.connectionState);
  const config = STATUS_CONFIG[connectionState];

  return (
    <Tooltip.Provider delayDuration={200}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <button
            type="button"
            className="flex items-center justify-center p-1 rounded-md hover:bg-background-tertiary transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-accent-glow"
            aria-label={`Sync status: ${config.label}`}
          >
            <span
              className={cn(
                'w-2 h-2 rounded-full',
                config.color,
                config.animate && 'animate-pulse'
              )}
            />
          </button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            className="px-3 py-1.5 text-sm bg-background-elevated border border-white/10 rounded-lg shadow-lg animate-fade-in"
            sideOffset={8}
          >
            {config.label}
            <Tooltip.Arrow className="fill-background-elevated" />
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}
