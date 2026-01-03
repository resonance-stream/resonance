/**
 * Tests for Sync Event Emitter
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SyncEventEmitter, syncEvents } from './syncEvents';

describe('SyncEventEmitter', () => {
  let emitter: SyncEventEmitter;

  beforeEach(() => {
    emitter = new SyncEventEmitter();
  });

  describe('on/emit', () => {
    it('should emit events to subscribed handlers', () => {
      const handler = vi.fn();
      emitter.on('connected', handler);

      emitter.emit('connected', {
        deviceId: 'test-device',
        sessionId: 'test-session',
        isReconnect: false,
      });

      expect(handler).toHaveBeenCalledTimes(1);
      expect(handler).toHaveBeenCalledWith({
        deviceId: 'test-device',
        sessionId: 'test-session',
        isReconnect: false,
      });
    });

    it('should support multiple handlers for the same event', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      emitter.on('connected', handler1);
      emitter.on('connected', handler2);

      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });

      expect(handler1).toHaveBeenCalledTimes(1);
      expect(handler2).toHaveBeenCalledTimes(1);
    });

    it('should not call handlers for different event types', () => {
      const connectedHandler = vi.fn();
      const errorHandler = vi.fn();

      emitter.on('connected', connectedHandler);
      emitter.on('error', errorHandler);

      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });

      expect(connectedHandler).toHaveBeenCalledTimes(1);
      expect(errorHandler).not.toHaveBeenCalled();
    });

    it('should return unsubscribe function', () => {
      const handler = vi.fn();
      const unsubscribe = emitter.on('connected', handler);

      // First emit should call handler
      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });
      expect(handler).toHaveBeenCalledTimes(1);

      // Unsubscribe
      unsubscribe();

      // Second emit should not call handler
      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });
      expect(handler).toHaveBeenCalledTimes(1);
    });
  });

  describe('onAny', () => {
    it('should receive all events', () => {
      const wildcardHandler = vi.fn();
      emitter.onAny(wildcardHandler);

      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });

      emitter.emit('error', {
        message: 'test error',
        isAuthError: false,
      });

      expect(wildcardHandler).toHaveBeenCalledTimes(2);
      expect(wildcardHandler).toHaveBeenNthCalledWith(1, 'connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });
      expect(wildcardHandler).toHaveBeenNthCalledWith(2, 'error', {
        message: 'test error',
        isAuthError: false,
      });
    });
  });

  describe('once', () => {
    it('should only trigger handler once', () => {
      const handler = vi.fn();
      emitter.once('connected', handler);

      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });
      emitter.emit('connected', {
        deviceId: 'test2',
        sessionId: 'test2',
        isReconnect: true,
      });

      expect(handler).toHaveBeenCalledTimes(1);
      expect(handler).toHaveBeenCalledWith({
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });
    });
  });

  describe('off', () => {
    it('should remove all handlers for an event type', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      emitter.on('connected', handler1);
      emitter.on('connected', handler2);

      emitter.off('connected');

      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });

      expect(handler1).not.toHaveBeenCalled();
      expect(handler2).not.toHaveBeenCalled();
    });
  });

  describe('clear', () => {
    it('should remove all handlers', () => {
      const connectedHandler = vi.fn();
      const errorHandler = vi.fn();
      const wildcardHandler = vi.fn();

      emitter.on('connected', connectedHandler);
      emitter.on('error', errorHandler);
      emitter.onAny(wildcardHandler);

      emitter.clear();

      emitter.emit('connected', {
        deviceId: 'test',
        sessionId: 'test',
        isReconnect: false,
      });

      expect(connectedHandler).not.toHaveBeenCalled();
      expect(errorHandler).not.toHaveBeenCalled();
      expect(wildcardHandler).not.toHaveBeenCalled();
    });
  });

  describe('listenerCount', () => {
    it('should return the number of handlers', () => {
      expect(emitter.listenerCount('connected')).toBe(0);

      emitter.on('connected', vi.fn());
      expect(emitter.listenerCount('connected')).toBe(1);

      emitter.on('connected', vi.fn());
      expect(emitter.listenerCount('connected')).toBe(2);
    });
  });

  describe('error handling', () => {
    it('should not throw when handler throws', () => {
      const errorHandler = vi.fn(() => {
        throw new Error('Handler error');
      });
      const normalHandler = vi.fn();

      emitter.on('connected', errorHandler);
      emitter.on('connected', normalHandler);

      // Should not throw
      expect(() => {
        emitter.emit('connected', {
          deviceId: 'test',
          sessionId: 'test',
          isReconnect: false,
        });
      }).not.toThrow();

      // Both handlers should have been called
      expect(errorHandler).toHaveBeenCalledTimes(1);
      expect(normalHandler).toHaveBeenCalledTimes(1);
    });
  });
});

describe('syncEvents singleton', () => {
  beforeEach(() => {
    syncEvents.clear();
  });

  it('should be a SyncEventEmitter instance', () => {
    expect(syncEvents).toBeInstanceOf(SyncEventEmitter);
  });

  it('should work as expected', () => {
    const handler = vi.fn();
    syncEvents.on('disconnected', handler);

    syncEvents.emit('disconnected', {
      wasClean: false,
    });

    expect(handler).toHaveBeenCalledWith({
      wasClean: false,
    });
  });
});
