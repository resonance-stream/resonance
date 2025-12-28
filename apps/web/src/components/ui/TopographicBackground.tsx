import { useEffect, useRef } from 'react';
import { createNoise3D } from 'simplex-noise';

interface TopographicBackgroundProps {
  className?: string;
}

export function TopographicBackground({ className = '' }: TopographicBackgroundProps): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const noise3D = createNoise3D();

    // Configuration
    const config = {
      scale: 0.003,          // Noise scale (smaller = larger features)
      speed: 0.0002,         // Animation speed
      contourLevels: 12,     // Number of contour lines
      lineWidth: 1,          // Line thickness
      primaryColor: [16, 185, 129],   // Mint RGB
      accentColor: [37, 99, 235],     // Navy RGB
      baseOpacity: 0.15,     // Base line opacity
    };

    let time = 0;

    const resize = (): void => {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = window.innerWidth * dpr;
      canvas.height = window.innerHeight * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const drawContours = (width: number, height: number): void => {
      ctx.clearRect(0, 0, width, height);

      const cellSize = 8; // Resolution of the grid
      const cols = Math.ceil(width / cellSize) + 1;
      const rows = Math.ceil(height / cellSize) + 1;

      // Generate noise field
      const field: number[][] = Array.from({ length: rows }, () =>
        Array.from({ length: cols }, () => 0)
      );
      for (let y = 0; y < rows; y++) {
        for (let x = 0; x < cols; x++) {
          const nx = x * cellSize * config.scale;
          const ny = y * cellSize * config.scale;
          field[y]![x] = noise3D(nx, ny, time);
        }
      }

      // Draw contour lines using marching squares
      for (let level = 0; level < config.contourLevels; level++) {
        const threshold = -1 + (2 * level) / (config.contourLevels - 1);

        // Alternate between mint and navy, with mint more prominent
        const usePrimary = level % 4 !== 0; // 3 mint, 1 navy
        const color = usePrimary ? config.primaryColor : config.accentColor;
        const opacity = config.baseOpacity * (usePrimary ? 1 : 0.8);

        ctx.strokeStyle = `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${opacity})`;
        ctx.lineWidth = config.lineWidth;
        ctx.beginPath();

        // Marching squares algorithm
        for (let y = 0; y < rows - 1; y++) {
          for (let x = 0; x < cols - 1; x++) {
            const rowY = field[y]!;
            const rowY1 = field[y + 1]!;

            const tl = rowY[x]! > threshold ? 1 : 0;
            const tr = rowY[x + 1]! > threshold ? 1 : 0;
            const br = rowY1[x + 1]! > threshold ? 1 : 0;
            const bl = rowY1[x]! > threshold ? 1 : 0;

            const caseIndex = tl * 8 + tr * 4 + br * 2 + bl;

            if (caseIndex === 0 || caseIndex === 15) continue;

            const px = x * cellSize;
            const py = y * cellSize;

            // Interpolate edge positions
            const lerp = (a: number, b: number): number => {
              if (Math.abs(b - a) < 0.0001) return 0.5;
              return (threshold - a) / (b - a);
            };

            const top = px + lerp(rowY[x]!, rowY[x + 1]!) * cellSize;
            const bottom = px + lerp(rowY1[x]!, rowY1[x + 1]!) * cellSize;
            const left = py + lerp(rowY[x]!, rowY1[x]!) * cellSize;
            const right = py + lerp(rowY[x + 1]!, rowY1[x + 1]!) * cellSize;

            // Draw line segments based on case
            const drawLine = (x1: number, y1: number, x2: number, y2: number): void => {
              ctx.moveTo(x1, y1);
              ctx.lineTo(x2, y2);
            };

            switch (caseIndex) {
              case 1: case 14: drawLine(px, left, bottom, py + cellSize); break;
              case 2: case 13: drawLine(bottom, py + cellSize, px + cellSize, right); break;
              case 3: case 12: drawLine(px, left, px + cellSize, right); break;
              case 4: case 11: drawLine(top, py, px + cellSize, right); break;
              case 5:
                drawLine(px, left, top, py);
                drawLine(bottom, py + cellSize, px + cellSize, right);
                break;
              case 6: case 9: drawLine(top, py, bottom, py + cellSize); break;
              case 7: case 8: drawLine(px, left, top, py); break;
              case 10:
                drawLine(top, py, px + cellSize, right);
                drawLine(px, left, bottom, py + cellSize);
                break;
            }
          }
        }

        ctx.stroke();
      }
    };

    const animate = (): void => {
      time += config.speed;
      drawContours(window.innerWidth, window.innerHeight);
      animationRef.current = requestAnimationFrame(animate);
    };

    resize();
    animate();

    window.addEventListener('resize', resize);

    return () => {
      window.removeEventListener('resize', resize);
      cancelAnimationFrame(animationRef.current);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className={`fixed inset-0 w-full h-full pointer-events-none ${className}`}
      style={{ zIndex: -1 }}
      aria-hidden="true"
    />
  );
}
