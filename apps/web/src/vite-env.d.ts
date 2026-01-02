/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

declare module 'virtual:pwa-register' {
  export interface RegisterSWOptions {
    immediate?: boolean
    onNeedRefresh?: () => void
    onOfflineReady?: () => void
    onRegistered?: (registration: ServiceWorkerRegistration | undefined) => void
    onRegisterError?: (error: unknown) => void
  }

  export function registerSW(options?: RegisterSWOptions): (reloadPage?: boolean) => Promise<void>
}

declare module 'resonance-visualizer' {
  export class Visualizer {
    constructor(container: HTMLElement);
    initWithAnalyser(analyser: AnalyserNode, audioContext: AudioContext): void;
    start(): void;
    stop(): void;
    destroy(): void;
    resize(width: number, height: number): void;
    readonly isRunning: boolean;
  }
}
