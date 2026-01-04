/**
 * ChatHeader Component Tests
 *
 * Tests for the chat header with title, connection status, and action buttons.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, userEvent } from '@/test/test-utils';
import { ChatHeader } from './ChatHeader';

describe('ChatHeader', () => {
  const defaultProps = {
    onClose: vi.fn(),
    onNewChat: vi.fn(),
    isConnected: true,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('title', () => {
    it('displays default title', () => {
      render(<ChatHeader {...defaultProps} />);

      expect(screen.getByRole('heading', { name: 'Resonance AI' })).toBeInTheDocument();
    });

    it('displays custom title when provided', () => {
      render(<ChatHeader {...defaultProps} title="Custom Assistant" />);

      expect(screen.getByRole('heading', { name: 'Custom Assistant' })).toBeInTheDocument();
    });
  });

  describe('connection status', () => {
    it('shows green indicator when connected', () => {
      render(<ChatHeader {...defaultProps} isConnected={true} />);

      const indicator = screen.getByTitle('Connected');
      expect(indicator).toHaveClass('bg-green-500');
    });

    it('shows red indicator when disconnected', () => {
      render(<ChatHeader {...defaultProps} isConnected={false} />);

      const indicator = screen.getByTitle('Disconnected');
      expect(indicator).toHaveClass('bg-red-500');
    });
  });

  describe('new chat button', () => {
    it('renders new conversation button', () => {
      render(<ChatHeader {...defaultProps} />);

      expect(screen.getByRole('button', { name: /new conversation/i })).toBeInTheDocument();
    });

    it('calls onNewChat when clicked', async () => {
      const user = userEvent.setup();
      const onNewChat = vi.fn();
      render(<ChatHeader {...defaultProps} onNewChat={onNewChat} />);

      await user.click(screen.getByRole('button', { name: /new conversation/i }));

      expect(onNewChat).toHaveBeenCalledTimes(1);
    });
  });

  describe('close button', () => {
    it('renders close button', () => {
      render(<ChatHeader {...defaultProps} />);

      expect(screen.getByRole('button', { name: /close chat/i })).toBeInTheDocument();
    });

    it('calls onClose when clicked', async () => {
      const user = userEvent.setup();
      const onClose = vi.fn();
      render(<ChatHeader {...defaultProps} onClose={onClose} />);

      await user.click(screen.getByRole('button', { name: /close chat/i }));

      expect(onClose).toHaveBeenCalledTimes(1);
    });
  });
});
