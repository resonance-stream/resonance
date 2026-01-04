/**
 * ChatInput Component Tests
 *
 * Tests for the chat input component with textarea and send button.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, userEvent, waitFor } from '@/test/test-utils';
import { ChatInput } from './ChatInput';

describe('ChatInput', () => {
  const defaultProps = {
    value: '',
    onChange: vi.fn(),
    onSend: vi.fn(),
    disabled: false,
    placeholder: 'Ask about your music...',
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('rendering', () => {
    it('renders textarea and send button', () => {
      render(<ChatInput {...defaultProps} />);

      expect(screen.getByRole('textbox')).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /send message/i })).toBeInTheDocument();
    });

    it('displays the correct placeholder', () => {
      render(<ChatInput {...defaultProps} placeholder="Type your message..." />);

      expect(screen.getByPlaceholderText('Type your message...')).toBeInTheDocument();
    });

    it('displays the current value in textarea', () => {
      render(<ChatInput {...defaultProps} value="Hello, world!" />);

      expect(screen.getByRole('textbox')).toHaveValue('Hello, world!');
    });
  });

  describe('onChange behavior', () => {
    it('calls onChange when typing', async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<ChatInput {...defaultProps} onChange={onChange} />);

      const textarea = screen.getByRole('textbox');
      await user.type(textarea, 'Hello');

      // onChange is called for each character typed
      // Note: Since the component is controlled, each keystroke fires onChange
      // with the event target value, and the value prop doesn't update during the test
      expect(onChange).toHaveBeenCalledTimes(5);
      // Each call receives just that character since value prop stays empty
      expect(onChange).toHaveBeenNthCalledWith(1, 'H');
      expect(onChange).toHaveBeenNthCalledWith(2, 'e');
      expect(onChange).toHaveBeenNthCalledWith(3, 'l');
      expect(onChange).toHaveBeenNthCalledWith(4, 'l');
      expect(onChange).toHaveBeenNthCalledWith(5, 'o');
    });
  });

  describe('send button behavior', () => {
    it('calls onSend when send button is clicked with non-empty value', async () => {
      const user = userEvent.setup();
      const onSend = vi.fn();
      render(<ChatInput {...defaultProps} value="Test message" onSend={onSend} />);

      await user.click(screen.getByRole('button', { name: /send message/i }));

      expect(onSend).toHaveBeenCalledWith('Test message');
    });

    it('does not call onSend when send button is clicked with empty value', async () => {
      const user = userEvent.setup();
      const onSend = vi.fn();
      render(<ChatInput {...defaultProps} value="" onSend={onSend} />);

      await user.click(screen.getByRole('button', { name: /send message/i }));

      expect(onSend).not.toHaveBeenCalled();
    });

    it('does not call onSend when send button is clicked with whitespace-only value', async () => {
      const user = userEvent.setup();
      const onSend = vi.fn();
      render(<ChatInput {...defaultProps} value="   " onSend={onSend} />);

      await user.click(screen.getByRole('button', { name: /send message/i }));

      expect(onSend).not.toHaveBeenCalled();
    });

    it('disables send button when value is empty', () => {
      render(<ChatInput {...defaultProps} value="" />);

      expect(screen.getByRole('button', { name: /send message/i })).toBeDisabled();
    });

    it('enables send button when value has content', () => {
      render(<ChatInput {...defaultProps} value="Hello" />);

      expect(screen.getByRole('button', { name: /send message/i })).not.toBeDisabled();
    });
  });

  describe('keyboard behavior', () => {
    it('sends message on Enter key (without Shift)', async () => {
      const user = userEvent.setup();
      const onSend = vi.fn();
      render(<ChatInput {...defaultProps} value="Test message" onSend={onSend} />);

      const textarea = screen.getByRole('textbox');
      textarea.focus();
      await user.keyboard('{Enter}');

      expect(onSend).toHaveBeenCalledWith('Test message');
    });

    it('does not send message on Shift+Enter (allows new lines)', async () => {
      const user = userEvent.setup();
      const onSend = vi.fn();
      render(<ChatInput {...defaultProps} value="Test message" onSend={onSend} />);

      const textarea = screen.getByRole('textbox');
      textarea.focus();
      await user.keyboard('{Shift>}{Enter}{/Shift}');

      expect(onSend).not.toHaveBeenCalled();
    });
  });

  describe('disabled state', () => {
    it('disables textarea when disabled prop is true', () => {
      render(<ChatInput {...defaultProps} disabled={true} />);

      expect(screen.getByRole('textbox')).toBeDisabled();
    });

    it('disables send button when disabled prop is true', () => {
      render(<ChatInput {...defaultProps} value="Hello" disabled={true} />);

      expect(screen.getByRole('button', { name: /send message/i })).toBeDisabled();
    });

    it('does not call onSend when disabled even with valid input', async () => {
      const user = userEvent.setup();
      const onSend = vi.fn();
      render(<ChatInput {...defaultProps} value="Hello" disabled={true} onSend={onSend} />);

      await user.click(screen.getByRole('button', { name: /send message/i }));

      expect(onSend).not.toHaveBeenCalled();
    });
  });

  describe('focus behavior', () => {
    it('focuses textarea on initial mount when not disabled', async () => {
      render(<ChatInput {...defaultProps} />);

      await waitFor(() => {
        expect(screen.getByRole('textbox')).toHaveFocus();
      });
    });
  });

  describe('accessibility', () => {
    it('send button has accessible label', () => {
      render(<ChatInput {...defaultProps} />);

      expect(screen.getByRole('button', { name: /send message/i })).toBeInTheDocument();
    });
  });
});
