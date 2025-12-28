import { useKeyboardShortcuts } from '../../hooks/useKeyboardShortcuts';

/**
 * Component that enables global keyboard shortcuts for the player.
 * Renders nothing to the DOM.
 */
export function KeyboardShortcuts(): null {
  useKeyboardShortcuts();
  return null;
}
