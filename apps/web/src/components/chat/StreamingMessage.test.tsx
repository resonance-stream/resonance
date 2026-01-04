/**
 * StreamingMessage Component Tests
 *
 * Tests for the streaming message display and typing indicator components.
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@/test/test-utils';
import { StreamingMessage, TypingIndicator } from './StreamingMessage';

describe('StreamingMessage', () => {
  it('renders the streaming content', () => {
    render(<StreamingMessage content="Hello, I am currently typing..." />);

    expect(screen.getByText('Hello, I am currently typing...')).toBeInTheDocument();
  });

  it('shows typing cursor animation', () => {
    render(<StreamingMessage content="Some content" />);

    // Find the pulsing cursor element (inline-block w-2 h-4 span with animate-pulse)
    const cursor = document.querySelector('.animate-pulse');
    expect(cursor).toBeInTheDocument();
    expect(cursor).toHaveClass('w-2', 'h-4', 'bg-accent-primary');
  });

  it('renders multiline content with preserved whitespace', () => {
    const multilineContent = 'Line 1\nLine 2\nLine 3';
    const { container } = render(<StreamingMessage content={multilineContent} />);

    // Content should be in a pre-wrap container (the div containing content and cursor)
    const preWrapContainer = container.querySelector('.whitespace-pre-wrap');
    expect(preWrapContainer).toBeInTheDocument();
    expect(preWrapContainer).toHaveTextContent('Line 1');
    expect(preWrapContainer).toHaveTextContent('Line 2');
    expect(preWrapContainer).toHaveTextContent('Line 3');
  });
});

describe('TypingIndicator', () => {
  it('renders three bouncing dots', () => {
    render(<TypingIndicator />);

    // TypingIndicator has 3 span elements with animate-bounce
    const dots = document.querySelectorAll('.animate-bounce');
    expect(dots).toHaveLength(3);
  });

  it('has staggered animation delays', () => {
    render(<TypingIndicator />);

    const dots = document.querySelectorAll('.animate-bounce');

    // Check the animation delay styles are staggered
    expect((dots[0] as HTMLElement).style.animationDelay).toBe('0ms');
    expect((dots[1] as HTMLElement).style.animationDelay).toBe('150ms');
    expect((dots[2] as HTMLElement).style.animationDelay).toBe('300ms');
  });
});
