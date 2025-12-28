import { useContext } from 'react';
import { AudioContext, type AudioContextValue } from '../contexts/AudioContext';

export function useAudio(): AudioContextValue {
  const context = useContext(AudioContext);
  if (!context) {
    throw new Error('useAudio must be used within an AudioProvider');
  }
  return context;
}
