import { createContext, type RefObject } from 'react';

export interface AudioContextValue {
  seek: (time: number) => void;
  audioRef: RefObject<HTMLAudioElement | null>;
}

export const AudioContext = createContext<AudioContextValue | null>(null);
